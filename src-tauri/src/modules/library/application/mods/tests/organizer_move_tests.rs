use super::*;
use crate::modules::games::domain::models::{GameType, ItemStatus};
use crate::modules::settings::application::config::{AppSettings, ConfigService, GameConfig};
use crate::platform::fs::guard::validate_path;
use crate::platform::fs::operation_lock::OperationLock;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::test_utils::{
    insert_test_collection, insert_test_game, insert_test_mod, insert_test_object,
    TestCollectionFixture, TestGameFixture, TestModFixture, TestObjectFixture,
};
use std::path::PathBuf;
use tempfile::TempDir;

fn normalized_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn organizer_move_leaves_projection_and_collection_reference_for_terminal_reconcile() {
    let temp = TempDir::new().expect("temporary mods root");
    let mods_root = temp.path();
    let source_path = mods_root.join("Source").join("Old Mod");
    let target_object_path = mods_root.join("Target");
    std::fs::create_dir_all(&source_path).expect("source folder");
    std::fs::write(
        source_path.join("mod.ini"),
        "[TextureOverrideTest]\nhash = abc\n",
    )
    .expect("mod metadata");
    std::fs::create_dir_all(&target_object_path).expect("target object folder");

    let pool = crate::test_utils::init_test_db().await.pool;
    let mods_root_text = mods_root.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "game",
            name: "Game",
            game_type: GameType::GIMI,
            path: &mods_root_text,
            mods_path: Some(&mods_root_text),
        },
    )
    .await
    .expect("game");
    for (id, name, folder_path) in [
        ("source", "Source", "Source"),
        ("target", "Target", "Target"),
    ] {
        insert_test_object(
            &pool,
            &TestObjectFixture {
                id,
                game_id: "game",
                name,
                folder_path,
                object_type: "Other",
            },
        )
        .await
        .expect("object");
    }
    insert_test_mod(
        &pool,
        &TestModFixture {
            id: "mod",
            game_id: "game",
            object_id: Some("source"),
            actual_name: "Old Mod",
            folder_path: "Source/Old Mod",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Other"),
            mods_path: Some(&mods_root_text),
        },
    )
    .await
    .expect("mod");
    insert_test_collection(
        &pool,
        &TestCollectionFixture {
            id: "collection",
            name: "Saved",
            game_id: "game",
            is_safe: true,
        },
    )
    .await
    .expect("collection");
    sqlx::query(
        "INSERT INTO collection_mods (collection_id, mod_id, mod_path, mod_path_key, object_ref_key, object_id, node_type) \
         VALUES ('collection', 'mod', 'Source/Old Mod', 'source/old mod', 'source', 'source', 'FlatModRoot')",
    )
    .execute(&pool)
    .await
    .expect("collection member");

    let config = ConfigService::new_for_test_async(pool.clone()).await;
    config
        .save_settings(AppSettings {
            games: vec![GameConfig {
                id: "game".to_string(),
                name: "Game".to_string(),
                game_type: GameType::GIMI,
                mod_path: PathBuf::from(mods_root),
                ready_to_move_path: None,
                game_exe: PathBuf::from(mods_root),
                loader_exe: None,
                launch_args: None,
                warnings: Vec::new(),
            }],
            ..AppSettings::default()
        })
        .expect("test game configuration");
    let folder = validate_path(&config, "game", &source_path.to_string_lossy())
        .expect("validated source folder");
    let lock = OperationLock::new();
    let guard = lock.acquire().await.expect("operation lock");
    let outcome = move_mods_to_object_service(
        &pool,
        &guard,
        &WatcherState::new(),
        MoveModsToObjectParams {
            game_id: "game",
            folder_paths: &[folder],
            target_object_id: "target",
            target_subpath: None,
            status: None,
        },
    )
    .await
    .expect("filesystem move");
    assert!(
        outcome.result.failures.is_empty(),
        "move must reach the terminal reconcile boundary"
    );
    assert_eq!(
        outcome
            .result
            .success
            .iter()
            .map(|path| normalized_path(path))
            .collect::<Vec<_>>(),
        vec!["Target/Old Mod"]
    );
    assert_eq!(outcome.path_hints.len(), 1);
    assert_eq!(
        normalized_path(&outcome.path_hints[0].old_path),
        "Source/Old Mod"
    );
    assert_eq!(
        normalized_path(&outcome.path_hints[0].new_path),
        "Target/Old Mod"
    );
    assert_eq!(outcome.path_hints[0].target_object_id, "target");

    assert!(target_object_path.join("Old Mod").is_dir());
    let mod_row: (Option<String>, String) =
        sqlx::query_as("SELECT object_id, folder_path FROM mods WHERE id = 'mod'")
            .fetch_one(&pool)
            .await
            .expect("mod projection");
    assert_eq!(
        mod_row,
        (Some("source".to_string()), "Source/Old Mod".to_string())
    );
    let collection_path: String = sqlx::query_scalar(
        "SELECT mod_path FROM collection_mods WHERE collection_id = 'collection'",
    )
    .fetch_one(&pool)
    .await
    .expect("collection reference");
    assert_eq!(collection_path, "Source/Old Mod");

    let changed_paths = vec![
        source_path.to_string_lossy().to_string(),
        target_object_path
            .join("Old Mod")
            .to_string_lossy()
            .to_string(),
    ];
    let reconcile_hints = outcome
        .path_hints
        .iter()
        .map(
            |hint| crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcilePathHint {
                old_path: hint.old_path.clone(),
                new_path: hint.new_path.clone(),
                target_object_id: hint.target_object_id.clone(),
            },
        )
        .collect::<Vec<_>>();
    crate::modules::reconciliation::application::disk_reconcile::reconcile::reconcile_disk_projection(
        crate::modules::reconciliation::application::disk_reconcile::reconcile::ReconcileDiskProjectionRequest {
            pool: &pool,
            game_id: "game",
            mods_path: mods_root,
            safe_mode_keywords: &[],
            reason: &crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::InternalMutation,
            changed_paths: &changed_paths,
            force_full: false,
            watcher_events: None,
            path_hints: &reconcile_hints,
            progress_reporter: None,
        },
    )
    .await
    .expect("terminal reconcile");

    let healed_mod: (Option<String>, String) =
        sqlx::query_as("SELECT object_id, folder_path FROM mods WHERE game_id = 'game'")
            .fetch_one(&pool)
            .await
            .expect("healed mod projection");
    assert_eq!(
        (healed_mod.0, normalized_path(&healed_mod.1)),
        (Some("target".to_string()), "Target/Old Mod".to_string())
    );
    let healed_collection_path: String = sqlx::query_scalar(
        "SELECT mod_path FROM collection_mods WHERE collection_id = 'collection'",
    )
    .fetch_one(&pool)
    .await
    .expect("healed collection reference");
    assert_eq!(normalized_path(&healed_collection_path), "Target/Old Mod");
}
