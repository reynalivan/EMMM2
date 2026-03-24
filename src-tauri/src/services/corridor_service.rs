use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use sqlx::SqlitePool;

use crate::domain::collection::{CollectionMod, CollectionObject};
use crate::domain::corridor::{CorridorSnapshot, CorridorSwitchPreview, SwitchResult};
use crate::domain::errors::CorridorError;
use crate::repo::{collection_repo, corridor_repo};

// ---------------------------------------------------------------------------
// corridor_service — Business logic for corridor mode switching
// ---------------------------------------------------------------------------

/// Get the current corridor state as a frontend-ready snapshot.
pub async fn get_corridor_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CorridorSnapshot, CorridorError> {
    // Ensure corridor row exists
    corridor_repo::ensure_exists(pool, game_id, is_safe).await?;

    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };
    let collections_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM collections WHERE game_id = ? AND is_safe = ?")
        .bind(game_id)
        .bind(is_safe_i32)
        .fetch_one(pool)
        .await
        .map_err(|e| CorridorError::Db(e.to_string()))?;

    if collections_count == 0 {
        // Auto-initialize Unsaved collection for virgin corridor
        crate::services::collection_service::handle_dirty_state(pool, game_id, is_safe).await.ok();
    }

    let mut snapshot = corridor_repo::get_snapshot(pool, game_id, is_safe).await?;

    // Compute dirty flag by comparing current signature against active collection signature
    if let Some(ref active_id) = snapshot.active_collection_id {
        let collection = crate::repo::collection_repo::get_by_id(pool, active_id)
            .await
            .ok()
            .flatten();

        if let Some(c) = collection {
            snapshot.is_dirty =
                snapshot.current_signature != c.signature.clone().unwrap_or_default();
        } else {
            snapshot.is_dirty = true; // Active collection was deleted
        }
    } else {
        // No active collection — dirty if any mods are enabled
        snapshot.is_dirty = !snapshot.current_signature.is_empty();
    }

    Ok(snapshot)
}

/// Switch corridor mode (safe ↔ unsafe).
/// Delegates to the switch pipeline.
pub async fn switch_corridor(
    pool: &SqlitePool,
    game_id: &str,
    target_safe: bool,
    mods_path: std::path::PathBuf,
    suppressor: Arc<AtomicBool>,
    watcher_state: &crate::services::scanner::watcher::WatcherState,
    settings: crate::services::config::AppSettings,
) -> Result<SwitchResult, CorridorError> {
    // Resolve the game's mods_path
    if !mods_path.exists() {
        return Err(CorridorError::NoModsPath {
            game_id: game_id.to_string(),
        });
    }

    let mut ctx = crate::pipeline::switch_pipeline::SwitchContext::new(
        pool.clone(),
        game_id.to_string(),
        target_safe,
        mods_path,
        suppressor,
        settings,
    );

    crate::pipeline::switch_pipeline::execute(&mut ctx, watcher_state).await
}

/// Preview the differences when switching corridor modes.
pub async fn preview_switch(
    pool: &SqlitePool,
    game_id: &str,
    target_safe: bool,
    _mods_path: Option<&str>,
) -> Result<CorridorSwitchPreview, CorridorError> {
    let leaving_safe = !target_safe;

    // 1. Get leaving state
    let leaving_snapshot = get_corridor_state(pool, game_id, leaving_safe).await?;
    let leaving_state_name = leaving_snapshot.active_collection_name.clone();

    // The currently active items in this corridor
    let is_safe_i32 = if leaving_safe { 1i32 } else { 0i32 };

    let active_objects: Vec<CollectionObject> = sqlx::query_as(
        r#"
        SELECT 
            'object' as kind,
            ? as collection_id, 
            id as object_id, 
            1 as is_enabled,
            name as display_name,
            id as path_key
        FROM objects
        WHERE game_id = ? AND status = 1
        "#,
    )
    .bind("")
    .bind(game_id)
    .fetch_all(pool)
    .await?;

    let active_mods: Vec<CollectionMod> = sqlx::query_as(
        r#"
        SELECT 
            'mod' as kind,
            ? as collection_id, 
            id as mod_id, 
            folder_path as mod_path, 
            folder_path_key as mod_path_key, 
            object_id,
            actual_name as display_name,
            1 as is_enabled
        FROM mods
        WHERE game_id = ? AND is_safe = ? AND status = 1
        "#,
    )
    .bind("")
    .bind(game_id)
    .bind(is_safe_i32)
    .fetch_all(pool)
    .await?;

    // 2. Get target state
    let target_snapshot = get_corridor_state(pool, game_id, target_safe).await?;
    let target_state_name = target_snapshot.active_collection_name.clone();

    // Resolve target members: prefer active collection, fall back to undo snapshot
    let resolve_collection_id = target_snapshot
        .active_collection_id
        .as_deref()
        .or(target_snapshot.undo_collection_id.as_deref());

    let (target_mods, target_objects) = if let Some(coll_id) = resolve_collection_id {
        let t_mods = collection_repo::get_mods(pool, coll_id)
            .await
            .map_err(CorridorError::from)?;
        let t_objs = collection_repo::get_objects(pool, coll_id)
            .await
            .map_err(CorridorError::from)?;
        (t_mods, t_objs)
    } else {
        // First-time entry or empty state — fall back to SYSTEM reason restoration
        let is_safe_i32 = if target_safe { 1i32 } else { 0i32 };
        let system_mods: Vec<CollectionMod> = sqlx::query_as(
            r#"
            SELECT 
                'mod' as kind,
                ? as collection_id, 
                id as mod_id, 
                folder_path as mod_path, 
                folder_path_key as mod_path_key, 
                object_id,
                actual_name as display_name,
                0 as is_enabled
            FROM mods
            WHERE game_id = ? AND is_safe = ? AND disabled_reason = 'SYSTEM'
            "#,
        )
        .bind("")
        .bind(game_id)
        .bind(is_safe_i32)
        .fetch_all(pool)
        .await?;

        // We still yield all physically active Objects so the UI defaults them to "ON"
        let t_objs: Vec<CollectionObject> = sqlx::query_as(
            r#"
            SELECT 
                'object' as kind,
                ? as collection_id, 
                id as object_id, 
                1 as is_enabled,
                name as display_name,
                id as path_key
            FROM objects 
            WHERE game_id = ? AND status = 1
            "#,
        )
        .bind("")
        .bind(game_id)
        .fetch_all(pool)
        .await?;

        (system_mods, t_objs)
    };

    let target_state_kind = if target_snapshot.active_collection_id.is_some() {
        "active_collection".to_string()
    } else if target_snapshot.undo_collection_id.is_some() {
        "undo_snapshot".to_string()
    } else if !target_mods.is_empty() {
        "system_fallback".to_string()
    } else {
        "none".to_string()
    };

    Ok(CorridorSwitchPreview {
        leaving_state_name,
        leaving_mods: active_mods,
        leaving_objects: active_objects,
        target_state_name,
        target_state_kind,
        target_mods,
        target_objects,
    })
}

/// Compute the current corridor signature from enabled mods.
/// This is used after mod toggles to keep the corridor cache up to date.
pub async fn recompute_signature(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<String, CorridorError> {
    let is_safe_i32 = if is_safe { 1i32 } else { 0i32 };

    let mut enabled_keys: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT o.folder_path_key 
        FROM objects o
        JOIN mods m ON m.object_id = o.id
        WHERE m.game_id = ? AND m.is_safe = ? AND m.status = 1
        "#,
    )
    .bind(game_id)
    .bind(is_safe_i32)
    .fetch_all(pool)
    .await?;

    enabled_keys.sort();

    let signature = blake3::hash(enabled_keys.join("\n").as_bytes())
        .to_hex()
        .to_string();

    corridor_repo::update_signature(pool, game_id, is_safe, &signature).await?;

    Ok(signature)
}
