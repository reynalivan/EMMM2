use sqlx::SqlitePool;

use crate::domain::corridor::{CorridorRuntime, CorridorSnapshot};
use crate::domain::errors::CorridorError;
use crate::repo::{collection_repo, corridor_repo};
use crate::services::projected_state_service;

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
    let (current_mods, current_objects) =
        crate::services::collection_service::load_live_runtime_state(pool, game_id)
            .await
            .map_err(CorridorError::from)?;
    let projected_state =
        projected_state_service::build_projected_state(&current_mods, &current_objects, None);
    let current_tree_nodes =
        projected_state_service::build_preview_tree_from_projected_state(&projected_state);
    let current_signature =
        projected_state_service::signature_for_projected_state(&projected_state);

    let collections = collection_repo::list_for_game(pool, game_id)
        .await
        .map_err(CorridorError::from)?;
    let named_match = collections.iter().find(|collection| {
        !collection.is_unsaved
            && collection.signature.as_deref() == Some(current_signature.as_str())
    });
    let matched_collection = named_match;

    let active_collection_id = matched_collection.map(|collection| collection.id.clone());
    let active_collection_name = matched_collection.map(|collection| collection.name.clone());
    let active_collection_is_unsaved = false;
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
        undo_collection_id: None,
        current_signature,
        is_dirty,
        current_mods,
        current_objects,
        current_tree_nodes,
        projected_state,
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
    use super::{get_corridor_state, resolve_restore_collection};
    use crate::domain::models::{GameType, ItemStatus};
    use crate::repo::{collection_repo, corridor_repo};
    use crate::services::projected_state_service;
    use crate::test_utils::{
        init_test_db, insert_test_game, insert_test_mod, insert_test_object,
        set_test_collection_snapshot, set_test_corridor_pointers_unchecked, TestGameFixture,
        TestModFixture, TestObjectFixture,
    };

    #[tokio::test]
    async fn get_corridor_state_reads_full_runtime_without_safety_filter() {
        let ctx = init_test_db().await;

        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: "game-runtime",
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
                game_id: "game-runtime",
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
                id: "safe-mod",
                game_id: "game-runtime",
                object_id: Some("object-1"),
                actual_name: "Blue",
                folder_path: "AINOZ/Blue",
                status: ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: Some("E:/Mods"),
            },
        )
        .await
        .expect("insert safe mod");
        insert_test_mod(
            &ctx.pool,
            &TestModFixture {
                id: "unsafe-mod",
                game_id: "game-runtime",
                object_id: Some("object-1"),
                actual_name: "Red",
                folder_path: "AINOZ/Red",
                status: ItemStatus::Enabled,
                is_safe: false,
                object_type: Some("Character"),
                mods_path: Some("E:/Mods"),
            },
        )
        .await
        .expect("insert unsafe mod");

        let snapshot = get_corridor_state(&ctx.pool, "game-runtime", true)
            .await
            .expect("load runtime snapshot");
        let mod_paths = snapshot
            .current_mods
            .iter()
            .map(|member| member.mod_path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(mod_paths, vec!["AINOZ/Blue", "AINOZ/Red"]);
        assert!(snapshot.active_collection_id.is_none());
        assert!(snapshot.active_collection_name.is_none());
        assert!(snapshot.is_dirty);
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
    async fn get_corridor_state_ignores_legacy_unsaved_active_collection() {
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

        assert_eq!(snapshot.active_collection_id.as_deref(), None);
        assert_eq!(snapshot.active_collection_name.as_deref(), None);
        assert!(!snapshot.active_collection_is_unsaved);
    }
}
