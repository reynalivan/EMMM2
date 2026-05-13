use std::sync::Arc;

use sqlx::SqlitePool;

use crate::domain::collection::{CollectionMod, CollectionObject};
use crate::domain::corridor::{
    CorridorRuntime, CorridorSnapshot, CorridorSwitchPreview, SwitchResult,
};
use crate::domain::errors::CorridorError;
use crate::repo::{collection_repo, corridor_repo};
use crate::services::projected_state_service;
use crate::services::scanner::watcher::WatcherSuppressor;

// ---------------------------------------------------------------------------
// corridor_service — Business logic for corridor mode switching
// ---------------------------------------------------------------------------

/// Get the current corridor state as a frontend-ready snapshot.
pub async fn get_corridor_state(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CorridorSnapshot, CorridorError> {
    corridor_repo::ensure_exists(pool, game_id, is_safe).await?;
    let corridor_state = corridor_repo::get(pool, game_id, is_safe).await?;
    let preferred_collection_id = corridor_state
        .as_ref()
        .and_then(|state| state.active_collection_id.clone());
    let undo_collection_id = corridor_state
        .as_ref()
        .and_then(|state| state.undo_collection_id.clone());
    let (current_mods, current_objects) =
        crate::services::collection_service::load_live_corridor_state(pool, game_id, is_safe)
            .await
            .map_err(CorridorError::from)?;
    let projected_state =
        projected_state_service::build_projected_state(&current_mods, &current_objects, None);
    let current_tree_nodes =
        projected_state_service::build_preview_tree_from_projected_state(&projected_state);
    let current_signature =
        projected_state_service::signature_for_projected_state(&projected_state);

    let collections = collection_repo::list_for_corridor(pool, game_id, is_safe, true)
        .await
        .map_err(CorridorError::from)?;
    let preferred_named_match = preferred_collection_id.as_ref().and_then(|collection_id| {
        collections.iter().find(|collection| {
            collection.id == *collection_id
                && !collection.is_unsaved
                && collection.signature.as_deref() == Some(current_signature.as_str())
        })
    });
    let named_match = collections.iter().find(|collection| {
        !collection.is_unsaved
            && collection.signature.as_deref() == Some(current_signature.as_str())
    });
    let preferred_unsaved_match = preferred_collection_id.as_ref().and_then(|collection_id| {
        collections.iter().find(|collection| {
            collection.id == *collection_id
                && collection.is_unsaved
                && collection.signature.as_deref() == Some(current_signature.as_str())
        })
    });
    let unsaved_match = collections.iter().find(|collection| {
        collection.is_unsaved && collection.signature.as_deref() == Some(current_signature.as_str())
    });
    let matched_collection = preferred_named_match
        .or(preferred_unsaved_match)
        .or(named_match)
        .or(unsaved_match);

    let active_collection_id = matched_collection.map(|collection| collection.id.clone());
    let active_collection_name = matched_collection.map(|collection| collection.name.clone());
    let active_collection_is_unsaved =
        matched_collection.is_some_and(|collection| collection.is_unsaved);
    let is_dirty = matched_collection.is_none();
    let snapshot_json = projected_state_service::serialize_snapshot_json(&projected_state)
        .unwrap_or_else(|| "{\"object_states\":[],\"active_roots\":[],\"summary\":{\"object_count\":0,\"enabled_object_count\":0,\"active_root_count\":0,\"missing_root_count\":0}}".to_string());
    let runtime = CorridorRuntime {
        game_id: game_id.to_string(),
        is_safe,
        matched_collection_id: active_collection_id.clone(),
        state_kind: if active_collection_is_unsaved || is_dirty {
            "unsaved".to_string()
        } else {
            "named".to_string()
        },
        state_name: active_collection_name.clone(),
        signature: current_signature.clone(),
        snapshot_json,
        snapshot_source: "live_scan".to_string(),
        updated_at: String::new(),
    };
    let _ = corridor_repo::upsert_runtime(pool, &runtime).await;

    Ok(CorridorSnapshot {
        game_id: game_id.to_string(),
        is_safe,
        active_collection_id,
        active_collection_name,
        active_collection_is_unsaved,
        undo_collection_id,
        current_signature,
        is_dirty,
        current_mods,
        current_objects,
        current_tree_nodes,
        projected_state,
    })
}

/// Switch corridor mode (safe ↔ unsafe).
/// Delegates to the switch pipeline.
pub async fn switch_corridor(
    pool: &SqlitePool,
    game_id: &str,
    target_safe: bool,
    mods_path: std::path::PathBuf,
    suppressor: Arc<WatcherSuppressor>,
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
    current_safe: bool,
    target_safe: bool,
    mods_path: Option<&str>,
) -> Result<CorridorSwitchPreview, CorridorError> {
    let leaving_safe = current_safe;

    // 1. Get leaving state
    let leaving_snapshot = get_corridor_state(pool, game_id, leaving_safe).await?;
    let leaving_state_name = leaving_snapshot.active_collection_name.clone();
    let leaving_state_is_unsaved =
        leaving_snapshot.is_dirty || leaving_snapshot.active_collection_is_unsaved;
    let active_mods = leaving_snapshot.current_mods.clone();
    let active_objects = leaving_snapshot.current_objects.clone();
    let leaving_projected_state = leaving_snapshot.projected_state.clone();

    if current_safe == target_safe {
        let state_kind = if leaving_snapshot.current_mods.is_empty()
            && leaving_snapshot.current_objects.is_empty()
        {
            "none".to_string()
        } else {
            "active_collection".to_string()
        };

        return Ok(CorridorSwitchPreview {
            leaving_state_name: leaving_state_name.clone(),
            leaving_state_is_unsaved,
            leaving_state_is_safe: leaving_safe,
            leaving_mods: active_mods.clone(),
            leaving_objects: active_objects.clone(),
            leaving_tree_nodes: leaving_snapshot.current_tree_nodes.clone(),
            leaving_projected_state: leaving_projected_state.clone(),
            target_state_name: leaving_state_name,
            target_state_is_unsaved: leaving_state_is_unsaved,
            target_state_is_safe: target_safe,
            target_state_kind: state_kind,
            target_mods: active_mods,
            target_objects: active_objects,
            target_tree_nodes: leaving_snapshot.current_tree_nodes.clone(),
            target_projected_state: leaving_projected_state,
        });
    }

    // 2. Get target state
    let resolved_target = resolve_restore_collection(pool, game_id, target_safe).await?;
    let target_state_name = resolved_target
        .as_ref()
        .map(|(collection, _)| collection.name.clone());
    let target_state_is_unsaved = resolved_target
        .as_ref()
        .is_some_and(|(collection, _)| collection.is_unsaved);

    let (target_mods, target_objects, target_state_kind, target_projected_state) =
        if let Some((collection, state_kind)) = resolved_target {
            let snapshot = if let Some(snapshot_json) = collection.snapshot_json.as_deref() {
                projected_state_service::parse_snapshot_json(snapshot_json)
                    .unwrap_or_else(projected_state_service::empty_projected_state)
            } else {
                let t_mods = collection_repo::get_mods(pool, &collection.id)
                    .await
                    .map_err(CorridorError::from)?;
                let t_objs = collection_repo::get_objects(pool, &collection.id)
                    .await
                    .map_err(CorridorError::from)?;
                projected_state_service::build_projected_state(&t_mods, &t_objs, mods_path)
            };
            let t_mods =
                projected_state_service::mods_from_projected_state(&collection.id, &snapshot);
            let t_objs =
                projected_state_service::objects_from_projected_state(&collection.id, &snapshot);
            (t_mods, t_objs, state_kind, snapshot)
        } else {
            // First-time entry or empty state — fall back to SYSTEM reason restoration
            let is_safe_i32 = if target_safe { 1i32 } else { 0i32 };
            let system_rows = sqlx::query(
                r#"
            SELECT 
                id as mod_id, 
                folder_path as mod_path, 
                folder_path_key as mod_path_key, 
                object_id,
                actual_name as display_name
            FROM mods
            WHERE game_id = ? AND is_safe = ? AND disabled_reason = 'SYSTEM'
            "#,
            )
            .bind(game_id)
            .bind(is_safe_i32)
            .fetch_all(pool)
            .await?;
            let mut system_mods = Vec::with_capacity(system_rows.len());
            for row in system_rows {
                use sqlx::Row;

                system_mods.push(CollectionMod {
                    kind: crate::domain::collection::MemberKind::Mod,
                    collection_id: String::new(),
                    mod_id: row.try_get("mod_id").ok(),
                    mod_path: row.try_get("mod_path").unwrap_or_default(),
                    mod_path_key: row.try_get("mod_path_key").ok(),
                    object_id: row.try_get("object_id").unwrap_or_default(),
                    display_name: row.try_get("display_name").ok(),
                    preview_path: None,
                    node_type: None,
                    warnings: Vec::new(),
                    is_enabled: false,
                });
            }

            // We still yield all physically active Objects so the UI defaults them to "ON"
            let t_objs: Vec<CollectionObject> = sqlx::query_as(
                r#"
            SELECT 
                'object' as kind,
                ? as collection_id, 
                id as object_id, 
                1 as is_enabled,
                name as display_name,
                folder_path as path_key
            FROM objects 
            WHERE game_id = ? AND status = 1
            "#,
            )
            .bind("")
            .bind(game_id)
            .fetch_all(pool)
            .await?;

            let snapshot =
                projected_state_service::build_projected_state(&system_mods, &t_objs, mods_path);
            (system_mods, t_objs, "system_fallback".to_string(), snapshot)
        };
    let target_state_kind = if target_mods.is_empty() && target_objects.is_empty() {
        "none".to_string()
    } else {
        target_state_kind
    };

    Ok(CorridorSwitchPreview {
        leaving_state_name,
        leaving_state_is_unsaved,
        leaving_state_is_safe: leaving_safe,
        leaving_mods: active_mods,
        leaving_objects: active_objects,
        leaving_tree_nodes: leaving_snapshot.current_tree_nodes.clone(),
        leaving_projected_state,
        target_state_name,
        target_state_is_unsaved,
        target_state_is_safe: target_safe,
        target_state_kind,
        target_mods: target_mods.clone(),
        target_objects: target_objects.clone(),
        target_tree_nodes: projected_state_service::build_preview_tree_from_projected_state(
            &target_projected_state,
        ),
        target_projected_state,
    })
}

pub(crate) async fn resolve_restore_collection(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<Option<(crate::domain::collection::Collection, String)>, CorridorError> {
    let corridor = corridor_repo::get(pool, game_id, is_safe).await?;

    if let Some(active_id) = corridor
        .as_ref()
        .and_then(|state| state.active_collection_id.as_deref())
    {
        if let Some(collection) = collection_repo::get_by_id(pool, active_id).await? {
            if collection.game_id == game_id && collection.is_safe == is_safe {
                return Ok(Some((collection, "active_collection".to_string())));
            }

            log::warn!(
                "corridor_service: active collection pointer '{}' points outside game '{}' safe={}",
                active_id,
                game_id,
                is_safe
            );
        }

        log::warn!(
            "corridor_service: stale active collection pointer '{}' for game '{}' safe={}",
            active_id,
            game_id,
            is_safe
        );
    }

    if let Some(collection) =
        collection_repo::find_unsaved_for_corridor(pool, game_id, is_safe, None).await?
    {
        return Ok(Some((collection, "unsaved".to_string())));
    }

    Ok(None)
}

/// Compute the current corridor signature from enabled mods.
/// This is used after mod toggles to keep the corridor cache up to date.
pub async fn recompute_signature(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<String, CorridorError> {
    let (mods, objects) =
        crate::services::collection_service::load_live_corridor_state(pool, game_id, is_safe)
            .await
            .map_err(CorridorError::from)?;
    let signature = crate::services::collection_service::compute_signature(&mods, &objects);
    corridor_repo::update_signature(pool, game_id, is_safe, &signature).await?;

    Ok(signature)
}

#[cfg(test)]
mod tests {
    use super::{get_corridor_state, preview_switch, resolve_restore_collection};
    use crate::database::models::{GameType, ItemStatus};
    use crate::repo::{collection_repo, corridor_repo};
    use crate::services::projected_state_service;
    use crate::test_utils::{
        init_test_db, insert_test_game, insert_test_mod, insert_test_object,
        set_test_collection_snapshot, set_test_corridor_pointers_unchecked, TestGameFixture,
        TestModFixture, TestObjectFixture,
    };

    #[tokio::test]
    async fn preview_switch_uses_object_folder_path_as_path_key() {
        let ctx = init_test_db().await;

        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: "game-1",
                name: "Test Game",
                game_type: GameType::GIMI,
                path: "E:/Games/TestGame",
                mods_path: Some("E:/Mods"),
            },
        )
        .await
        .expect("insert game");

        insert_test_object(
            &ctx.pool,
            &TestObjectFixture {
                id: "object-1",
                game_id: "game-1",
                name: "AINOZ",
                folder_path: "AINOZ",
                object_type: "Character",
            },
        )
        .await
        .expect("insert object");

        insert_test_mod(
            &ctx.pool,
            &TestModFixture {
                id: "mod-1",
                game_id: "game-1",
                object_id: Some("object-1"),
                actual_name: "Blue",
                folder_path: "AINOZ/Variants/Blue",
                status: ItemStatus::Enabled,
                is_safe: false,
                object_type: Some("Character"),
                mods_path: Some("E:/Mods"),
            },
        )
        .await
        .expect("insert mod");

        let preview = preview_switch(&ctx.pool, "game-1", false, true, None)
            .await
            .expect("preview switch");

        assert_eq!(
            preview
                .leaving_objects
                .first()
                .and_then(|object| object.path_key.as_deref()),
            Some("AINOZ")
        );
        assert_eq!(
            preview
                .target_objects
                .first()
                .and_then(|object| object.path_key.as_deref()),
            Some("AINOZ")
        );
        assert_eq!(
            preview
                .leaving_tree_nodes
                .first()
                .map(|node| node.name.as_str()),
            Some("AINOZ")
        );
        assert_eq!(
            preview
                .target_tree_nodes
                .first()
                .map(|node| node.name.as_str()),
            Some("AINOZ")
        );
    }

    #[tokio::test]
    async fn resolve_restore_collection_falls_back_to_unsaved_when_active_pointer_is_stale() {
        let ctx = init_test_db().await;

        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: "game-1",
                name: "Test Game",
                game_type: GameType::GIMI,
                path: "E:/Games/TestGame",
                mods_path: Some("E:/Mods"),
            },
        )
        .await
        .expect("insert game");

        let unsaved = collection_repo::create(
            &ctx.pool,
            "unsaved-1",
            "game-1",
            "Unsaved 202603251210",
            false,
            true,
        )
        .await
        .expect("create unsaved");

        set_test_corridor_pointers_unchecked(
            &ctx.pool,
            "game-1",
            false,
            Some("missing-active"),
            None,
        )
        .await
        .expect("set stale active pointer");

        let resolved = resolve_restore_collection(&ctx.pool, "game-1", false)
            .await
            .expect("resolve target")
            .expect("fallback target");

        assert_eq!(resolved.0.id, unsaved.id);
        assert_eq!(resolved.1, "unsaved");
    }

    #[tokio::test]
    async fn get_corridor_state_marks_unsaved_active_collection() {
        let ctx = init_test_db().await;

        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: "game-1",
                name: "Test Game",
                game_type: GameType::GIMI,
                path: "E:/Games/TestGame",
                mods_path: Some("E:/Mods"),
            },
        )
        .await
        .expect("insert game");

        let unsaved =
            collection_repo::create(&ctx.pool, "unsaved-1", "game-1", "202603251217", true, true)
                .await
                .expect("create unsaved");
        set_test_collection_snapshot(
            &ctx.pool,
            &unsaved.id,
            &projected_state_service::empty_projected_state(),
        )
        .await
        .expect("seed unsaved snapshot");

        corridor_repo::update_pointers(&ctx.pool, "game-1", true, Some(&unsaved.id), None)
            .await
            .expect("set active pointer");

        let snapshot = get_corridor_state(&ctx.pool, "game-1", true)
            .await
            .expect("get corridor state");

        assert_eq!(snapshot.active_collection_id.as_deref(), Some("unsaved-1"));
        assert_eq!(
            snapshot.active_collection_name.as_deref(),
            Some("202603251217")
        );
        assert!(snapshot.active_collection_is_unsaved);
    }

    #[tokio::test]
    async fn preview_switch_exposes_unsaved_corridor_metadata_for_both_sides() {
        let ctx = init_test_db().await;

        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: "game-1",
                name: "Test Game",
                game_type: GameType::GIMI,
                path: "E:/Games/TestGame",
                mods_path: Some("E:/Mods"),
            },
        )
        .await
        .expect("insert game");

        let safe_unsaved = collection_repo::create(
            &ctx.pool,
            "unsaved-safe",
            "game-1",
            "202603251300",
            true,
            true,
        )
        .await
        .expect("create safe unsaved");
        let unsafe_unsaved = collection_repo::create(
            &ctx.pool,
            "unsaved-unsafe",
            "game-1",
            "202603251301",
            false,
            true,
        )
        .await
        .expect("create unsafe unsaved");
        let empty_state = projected_state_service::empty_projected_state();
        set_test_collection_snapshot(&ctx.pool, &safe_unsaved.id, &empty_state)
            .await
            .expect("seed safe unsaved snapshot");
        set_test_collection_snapshot(&ctx.pool, &unsafe_unsaved.id, &empty_state)
            .await
            .expect("seed unsafe unsaved snapshot");

        corridor_repo::update_pointers(&ctx.pool, "game-1", true, Some(&safe_unsaved.id), None)
            .await
            .expect("set safe active");
        corridor_repo::update_pointers(&ctx.pool, "game-1", false, Some(&unsafe_unsaved.id), None)
            .await
            .expect("set unsafe active");

        let preview = preview_switch(&ctx.pool, "game-1", true, false, None)
            .await
            .expect("preview switch");

        assert_eq!(preview.leaving_state_name.as_deref(), Some("202603251300"));
        assert!(preview.leaving_state_is_unsaved);
        assert!(preview.leaving_state_is_safe);
        assert_eq!(preview.target_state_name.as_deref(), Some("202603251301"));
        assert!(preview.target_state_is_unsaved);
        assert!(!preview.target_state_is_safe);
    }
}
