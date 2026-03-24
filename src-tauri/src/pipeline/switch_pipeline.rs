use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use sqlx::SqlitePool;

use crate::domain::collection::{CollectionMod, CollectionObject, CollectionRoot, MemberKind};
use crate::domain::corridor::SwitchResult;
use crate::domain::errors::CorridorError;
use crate::repo::{collection_repo, corridor_repo};
use crate::services::app::post_apply::PostApplyContext;
use crate::services::config::AppSettings;
use crate::services::corridor_constants::DISABLED_REASON_SYSTEM;
use crate::services::mods::bulk_ops;
use crate::services::scanner::watcher::WatcherState;

// ---------------------------------------------------------------------------
// SwitchPipeline — Composable corridor switch operation
// ---------------------------------------------------------------------------

/// Context passed through all pipeline steps during a corridor switch.
pub struct SwitchContext {
    // Inputs
    pub pool: SqlitePool,
    pub game_id: String,
    pub target_safe: bool,
    pub mods_path: PathBuf,
    pub suppressor: Arc<AtomicBool>,
    pub settings: AppSettings,

    // State during execution
    pub snapshot_id: Option<String>,
    pub mods_disabled: usize,
    pub mods_restored: usize,
    pub new_signature: String,
    pub restored_collection_id: Option<String>,
}

impl SwitchContext {
    pub fn new(
        pool: SqlitePool,
        game_id: String,
        target_safe: bool,
        mods_path: PathBuf,
        suppressor: Arc<AtomicBool>,
        settings: AppSettings,
    ) -> Self {
        Self {
            pool,
            game_id,
            target_safe,
            mods_path,
            suppressor,
            settings,
            snapshot_id: None,
            mods_disabled: 0,
            mods_restored: 0,
            new_signature: String::new(),
            restored_collection_id: None,
        }
    }
}

/// Execute the full corridor switch pipeline.
/// `watcher_state` is passed by reference since WatcherState is not Clone.
pub async fn execute(
    ctx: &mut SwitchContext,
    watcher_state: &WatcherState,
) -> Result<SwitchResult, CorridorError> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let target_str = ctx.target_safe.to_string();
    crate::repo::task_repo::create_task(
        &ctx.pool,
        &task_id,
        &ctx.game_id,
        "switch_corridor",
        Some(&target_str),
    )
    .await
    .map_err(|e| CorridorError::Db(e.to_string()))?;

    let result = execute_inner(ctx, watcher_state).await;

    let status = if result.is_ok() {
        crate::domain::task::TaskStatus::Completed
    } else {
        crate::domain::task::TaskStatus::Failed
    };
    let _ = crate::repo::task_repo::update_status(&ctx.pool, &task_id, status).await;

    result
}

async fn execute_inner(
    ctx: &mut SwitchContext,
    watcher_state: &WatcherState,
) -> Result<SwitchResult, CorridorError> {
    // Step 1: Snapshot the leaving corridor state
    snapshot_leaving(ctx).await?;

    // Step 2: Disable all enabled mods in the leaving corridor (FS + DB)
    disable_leaving(ctx, watcher_state).await?;

    // Step 3: Restore mods in the target corridor (FS + DB)
    restore_target(ctx, watcher_state).await?;

    // Step 4: Update corridor state + recompute signature
    update_state(ctx).await?;

    // Step 5: Run post-apply tasks (KeyViewer, Conflicts, Status)
    let post_ctx = PostApplyContext {
        pool: ctx.pool.clone(),
        game_id: ctx.game_id.clone(),
        is_safe: ctx.target_safe,
        mods_path: ctx.mods_path.clone(),
        suppressor: ctx.suppressor.clone(),
        settings: ctx.settings.clone(),
        status_fields: None,
    };
    let _ = crate::services::app::post_apply::run_post_apply_tasks(post_ctx).await;

    Ok(SwitchResult {
        success: true,
        mods_disabled: ctx.mods_disabled,
        mods_restored: ctx.mods_restored,
        new_signature: ctx.new_signature.clone(),
        restored_collection_id: ctx.restored_collection_id.clone(),
    })
}

// ---------------------------------------------------------------------------
// Individual switch steps
// ---------------------------------------------------------------------------

/// Step 1: Create an undo snapshot of the leaving corridor's currently-enabled mods.
/// This lets the user return to this state when switching back.
async fn snapshot_leaving(ctx: &mut SwitchContext) -> Result<(), CorridorError> {
    let leaving_safe = !ctx.target_safe;
    let is_safe_i32 = if leaving_safe { 1i32 } else { 0i32 };

    // Ensure corridor rows exist for both sides
    corridor_repo::ensure_exists(&ctx.pool, &ctx.game_id, leaving_safe).await?;
    corridor_repo::ensure_exists(&ctx.pool, &ctx.game_id, ctx.target_safe).await?;

    // Get currently-enabled items for the leaving corridor
    let enabled_objects: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, name, folder_path FROM objects WHERE game_id = ? AND status = 1",
    )
    .bind(&ctx.game_id)
    .fetch_all(&ctx.pool)
    .await?;

    let enabled_mods: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, object_id, folder_path, folder_path_key, actual_name FROM mods WHERE game_id = ? AND is_safe = ? AND status = 1"
    )
    .bind(&ctx.game_id)
    .bind(is_safe_i32)
    .fetch_all(&ctx.pool)
    .await?;

    if enabled_mods.is_empty() && enabled_objects.is_empty() {
        log::info!(
            "switch_pipeline: no enabled items in leaving corridor (safe={}), skipping snapshot",
            leaving_safe
        );
        return Ok(());
    }

    // Create undo snapshot collection
    let snapshot_id = format!("undo-switch-{}", uuid::Uuid::new_v4());
    collection_repo::create(
        &ctx.pool,
        &snapshot_id,
        &ctx.game_id,
        "Switch Undo Snapshot",
        leaving_safe,
        false, // not unsaved
    )
    .await
    .map_err(|e| CorridorError::Db(e.to_string()))?;

    // Build members
    let mut mods = Vec::new();
    let mut objects = Vec::new();
    let mut roots = Vec::new();

    for (oid, name, path) in enabled_objects {
        objects.push(CollectionObject {
            kind: MemberKind::Object,
            collection_id: snapshot_id.clone(),
            object_id: oid.clone(),
            is_enabled: true,
            display_name: Some(name.clone()),
            path_key: Some(oid.clone()),
        });
        roots.push(CollectionRoot {
            kind: MemberKind::Root,
            collection_id: snapshot_id.clone(),
            root_path: path.clone(),
            root_path_key: oid.clone(),
            display_name: name,
            display_name_key: oid.clone(),
            object_id: Some(oid),
            object_name: None,
            object_type: None,
            root_kind: "object".to_string(),
            is_safe: leaving_safe,
            is_enabled: true,
            thumbnail_hint: None,
            corridor_source: None,
        });
    }

    for (mid, oid, path, key, actual_name) in enabled_mods {
        mods.push(CollectionMod {
            kind: MemberKind::Mod,
            collection_id: snapshot_id.clone(),
            mod_id: Some(mid.clone()),
            mod_path: path.clone(),
            mod_path_key: Some(key.clone()),
            object_id: oid.clone(),
            display_name: Some(actual_name),
            is_enabled: true,
        });
        roots.push(CollectionRoot {
            kind: MemberKind::Root,
            collection_id: snapshot_id.clone(),
            root_path: path,
            root_path_key: key,
            display_name: "Mod".to_string(), // Placeholder
            display_name_key: mid,
            object_id: Some(oid),
            object_name: None,
            object_type: None,
            root_kind: "mod".to_string(),
            is_safe: leaving_safe,
            is_enabled: true,
            thumbnail_hint: None,
            corridor_source: None,
        });
    }

    // Compute signature
    let signature = crate::services::collection_service::compute_signature(&mods);

    collection_repo::replace_all_state(
        &ctx.pool,
        &snapshot_id,
        &mods,
        &objects,
        &roots,
        Some(&signature),
        None,
    )
    .await
    .map_err(|e| CorridorError::Db(e.to_string()))?;

    // Preserve existing active_collection_id, set undo to this snapshot
    let corridor = corridor_repo::get(&ctx.pool, &ctx.game_id, leaving_safe).await?;
    let active_id = corridor
        .as_ref()
        .and_then(|c| c.active_collection_id.as_deref());

    corridor_repo::update_pointers(
        &ctx.pool,
        &ctx.game_id,
        leaving_safe,
        active_id,
        Some(&snapshot_id),
    )
    .await?;

    // Also update runtime cache signature for the leaving side
    sqlx::query(
        "UPDATE corridor_runtime_cache SET signature = ? WHERE game_id = ? AND is_safe = ?",
    )
    .bind(&signature)
    .bind(&ctx.game_id)
    .bind(is_safe_i32)
    .execute(&ctx.pool)
    .await
    .map_err(|e| CorridorError::Db(e.to_string()))?;

    ctx.snapshot_id = Some(snapshot_id.clone());

    log::info!(
        "switch_pipeline: created undo snapshot '{}' for leaving corridor (safe={})",
        snapshot_id,
        leaving_safe
    );

    Ok(())
}

/// Step 2: Disable all enabled mods in the leaving corridor via FS rename + DB update.
async fn disable_leaving(
    ctx: &mut SwitchContext,
    watcher_state: &WatcherState,
) -> Result<(), CorridorError> {
    let leaving_safe = !ctx.target_safe;
    let is_safe_i32 = if leaving_safe { 1i32 } else { 0i32 };
    let mods_path = ctx.mods_path.to_string_lossy().to_string();

    // Get enabled mods (id, folder_path) — filter out depth-1 object containers
    let all_enabled: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT id, folder_path FROM mods
        WHERE game_id = ? AND is_safe = ? AND status = 1
        AND object_id IS NOT NULL"#,
    )
    .bind(&ctx.game_id)
    .bind(is_safe_i32)
    .fetch_all(&ctx.pool)
    .await?;

    let to_disable: Vec<(String, String)> = all_enabled;

    if to_disable.is_empty() {
        ctx.mods_disabled = 0;
        return Ok(());
    }

    let (disabled_count, warnings) = bulk_ops::bulk_toggle_mods(
        &ctx.pool,
        watcher_state,
        &mods_path,
        &ctx.game_id,
        to_disable,
        false, // disable
        Some(DISABLED_REASON_SYSTEM),
    )
    .await
    .map_err(|e| CorridorError::Db(e.to_string()))?;

    for warning in &warnings {
        log::warn!("switch_pipeline[disable]: {}", warning);
    }

    ctx.mods_disabled = disabled_count;

    log::info!(
        "switch_pipeline: disabled {} mods in leaving corridor (safe={})",
        disabled_count,
        leaving_safe
    );

    Ok(())
}

/// Step 3: Restore the target corridor's state.
async fn restore_target(
    ctx: &mut SwitchContext,
    watcher_state: &WatcherState,
) -> Result<(), CorridorError> {
    // Read the target corridor's saved pointers
    let target_corridor = corridor_repo::get(&ctx.pool, &ctx.game_id, ctx.target_safe).await?;

    // Priority: active collection first, then undo snapshot
    let restore_id = target_corridor
        .as_ref()
        .and_then(|c| c.active_collection_id.clone())
        .or_else(|| {
            target_corridor
                .as_ref()
                .and_then(|c| c.undo_collection_id.clone())
        });

    let collection_id = match restore_id {
        Some(id) => id,
        None => {
            log::info!(
                "switch_pipeline: target corridor (safe={}) has no saved state, falling back to SYSTEM reason",
                ctx.target_safe
            );
            return restore_via_system_reason(ctx, watcher_state).await;
        }
    };

    // Verify the collection still exists in DB
    let collection = collection_repo::get_by_id(&ctx.pool, &collection_id)
        .await
        .ok()
        .flatten();

    if collection.is_none() {
        log::warn!(
            "switch_pipeline: target collection '{}' no longer exists, nothing to restore",
            collection_id
        );
        ctx.mods_restored = 0;
        return Ok(());
    }

    let kind_label = if collection.as_ref().map(|c| c.is_unsaved).unwrap_or(false) {
        "unsaved"
    } else {
        "named"
    };

    log::info!(
        "switch_pipeline: restoring target corridor (safe={}) via {} collection '{}'",
        ctx.target_safe,
        kind_label,
        collection_id
    );

    // Delegate to apply_pipeline — same code path for named and autosaved collections
    ctx.restored_collection_id = Some(collection_id.clone());
    let apply_result = crate::services::collection_service::apply_collection(
        &ctx.pool,
        &ctx.game_id,
        &collection_id,
        ctx.target_safe,
        ctx.mods_path.clone(),
        ctx.suppressor.clone(),
        true, // ignore_missing during corridor restore
        ctx.settings.clone(),
    )
    .await
    .map_err(|e| CorridorError::Db(e.to_string()))?;

    ctx.mods_restored = apply_result.mods_enabled;

    log::info!(
        "switch_pipeline: restored via '{}' — {} enabled, {} disabled",
        collection_id,
        apply_result.mods_enabled,
        apply_result.mods_disabled
    );

    Ok(())
}

/// Step 4: Recompute signature from newly-enabled mods and record switch timestamp.
async fn update_state(ctx: &mut SwitchContext) -> Result<(), CorridorError> {
    // Recompute signature for the target corridor
    ctx.new_signature = crate::services::corridor_service::recompute_signature(
        &ctx.pool,
        &ctx.game_id,
        ctx.target_safe,
    )
    .await?;

    // Record the switch timestamp
    corridor_repo::record_switch(&ctx.pool, &ctx.game_id, ctx.target_safe).await?;

    log::info!(
        "switch_pipeline: corridor switch complete for game '{}' → safe={}, sig='{}'",
        ctx.game_id,
        ctx.target_safe,
        &ctx.new_signature[..8.min(ctx.new_signature.len())]
    );

    Ok(())
}

/// Fallback restoration when no collection exists: enable all mods where `disabled_reason = 'SYSTEM'`.
async fn restore_via_system_reason(
    ctx: &mut SwitchContext,
    watcher_state: &WatcherState,
) -> Result<(), CorridorError> {
    let is_safe_i32 = if ctx.target_safe { 1i32 } else { 0i32 };

    // Find mods that were disabled by SYSTEM in this corridor
    let mods_to_restore: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, folder_path FROM mods WHERE game_id = ? AND is_safe = ? AND status = 0 AND disabled_reason = ?",
    )
    .bind(&ctx.game_id)
    .bind(is_safe_i32)
    .bind(DISABLED_REASON_SYSTEM)
    .fetch_all(&ctx.pool)
    .await
    .map_err(|e| CorridorError::Db(e.to_string()))?;

    if mods_to_restore.is_empty() {
        log::info!("switch_pipeline: no mods found with SYSTEM reason to restore");
        ctx.mods_restored = 0;
        return Ok(());
    }

    log::info!(
        "switch_pipeline: restoring {} mods via SYSTEM reason",
        mods_to_restore.len()
    );

    let mods_path_str = ctx.mods_path.to_string_lossy().to_string();

    // Use bulk_ops for parallel restoration
    let (success_count, warnings) = bulk_ops::bulk_toggle_mods(
        &ctx.pool,
        watcher_state,
        &mods_path_str,
        &ctx.game_id,
        mods_to_restore,
        true, // target_enabled
        None, // clear disabled_reason
    )
    .await
    .map_err(|e| CorridorError::Db(e.to_string()))?;

    ctx.mods_restored = success_count;

    if !warnings.is_empty() {
        log::warn!(
            "switch_pipeline: system restoration had {} warnings: {:?}",
            warnings.len(),
            warnings
        );
    }

    Ok(())
}
