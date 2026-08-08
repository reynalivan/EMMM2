use super::*;
use crate::domain::models::{GameType, ItemStatus};
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, TestGameFixture, TestModFixture,
};

#[tokio::test]
async fn toggle_mods_mixed_returns_runtime_path_rewrites() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(mods_path.join("Variant")).expect("mod folder");
    let mods_path_string = mods_path.to_string_lossy().to_string();

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-runtime-toggle",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("insert game");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-runtime-toggle",
            game_id: "game-runtime-toggle",
            object_id: None,
            actual_name: "Variant",
            folder_path: "Variant",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("insert mod");

    let result = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: "game-runtime-toggle".to_string(),
            mods_path: mods_path.clone(),
            operations: vec![RuntimeToggleOperation {
                id: "mod-runtime-toggle".to_string(),
                folder_path: "Variant".to_string(),
                target_enabled: false,
                disabled_reason: None,
            }],
        },
    )
    .await
    .expect("toggle");

    assert_eq!(result.path_rewrites.len(), 1);
    assert_eq!(
        result.path_rewrites[0].old_path,
        mods_path.join("Variant").to_string_lossy().to_string()
    );
    assert_eq!(
        result.path_rewrites[0].new_path,
        mods_path
            .join("DISABLED Variant")
            .to_string_lossy()
            .to_string()
    );
}

#[tokio::test]
async fn toggle_mods_mixed_repairs_stale_disabled_db_path_when_disk_is_enabled() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let enabled_path = mods_path.join("Variant");
    std::fs::create_dir_all(&enabled_path).expect("mod folder");
    let mods_path_string = mods_path.to_string_lossy().to_string();

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-runtime-toggle-repair",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("insert game");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-runtime-toggle-repair",
            game_id: "game-runtime-toggle-repair",
            object_id: None,
            actual_name: "Variant",
            folder_path: "DISABLED Variant",
            status: ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("insert mod");

    let result = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: "game-runtime-toggle-repair".to_string(),
            mods_path: mods_path.clone(),
            operations: vec![RuntimeToggleOperation {
                id: "mod-runtime-toggle-repair".to_string(),
                folder_path: "DISABLED Variant".to_string(),
                target_enabled: true,
                disabled_reason: None,
            }],
        },
    )
    .await
    .expect("toggle");

    assert_eq!(result.enabled_count + result.disabled_count, 1);
    assert_eq!(result.path_rewrites.len(), 1);
    assert_eq!(
        result.path_rewrites[0].old_path,
        mods_path
            .join("DISABLED Variant")
            .to_string_lossy()
            .to_string()
    );
    assert_eq!(
        result.path_rewrites[0].new_path,
        enabled_path.to_string_lossy().to_string()
    );

    let row: (String, i64) = sqlx::query_as("SELECT folder_path, status FROM mods WHERE id = ?")
        .bind("mod-runtime-toggle-repair")
        .fetch_one(&ctx.pool)
        .await
        .expect("mod row");
    assert_eq!(row.0, "Variant");
    assert_eq!(row.1, ItemStatus::Enabled as i64);
}

#[tokio::test]
async fn toggle_mods_mixed_returns_empty_result_for_empty_operations() {
    let ctx = init_test_db().await;

    let result = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: "game-1".to_string(),
            mods_path: PathBuf::from("does-not-matter"),
            operations: Vec::new(),
        },
    )
    .await
    .expect("empty batch is a no-op");

    assert_eq!(result.enabled_count + result.disabled_count, 0);
    assert_eq!(result.enabled_count, 0);
    assert_eq!(result.disabled_count, 0);
    assert!(result.warnings.is_empty());
    assert!(result.path_rewrites.is_empty());
}

#[tokio::test]
async fn toggle_mods_mixed_rejects_absolute_and_traversal_paths() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let absolute = temp.path().join("Variant").to_string_lossy().to_string();

    let absolute_error = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: "game-1".to_string(),
            mods_path: temp.path().to_path_buf(),
            operations: vec![RuntimeToggleOperation {
                id: "mod-1".to_string(),
                folder_path: absolute,
                target_enabled: false,
                disabled_reason: None,
            }],
        },
    )
    .await
    .expect_err("absolute path must be rejected");
    assert!(
        absolute_error
            .to_string()
            .contains("Absolute mod folder path is not allowed"),
        "{absolute_error}"
    );

    let traversal_error = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: "game-1".to_string(),
            mods_path: temp.path().to_path_buf(),
            operations: vec![RuntimeToggleOperation {
                id: "mod-1".to_string(),
                folder_path: "../Escape".to_string(),
                target_enabled: false,
                disabled_reason: None,
            }],
        },
    )
    .await
    .expect_err("parent traversal must be rejected");
    assert!(
        traversal_error
            .to_string()
            .contains("Unsafe mod folder path is not allowed"),
        "{traversal_error}"
    );
}

#[tokio::test]
async fn toggle_mods_mixed_errors_when_mod_folder_is_missing_on_disk() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");

    let error = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: "game-1".to_string(),
            mods_path: temp.path().to_path_buf(),
            operations: vec![RuntimeToggleOperation {
                id: "mod-1".to_string(),
                folder_path: "Ghost".to_string(),
                target_enabled: false,
                disabled_reason: None,
            }],
        },
    )
    .await
    .expect_err("missing folder must error");
    assert!(
        error.to_string().contains("Mod folder does not exist"),
        "{error}"
    );
}

#[tokio::test]
async fn toggle_mods_mixed_rejects_duplicate_source_paths_before_renaming() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(mods_path.join("Variant")).expect("mod folder");

    let operation = RuntimeToggleOperation {
        id: "mod-1".to_string(),
        folder_path: "Variant".to_string(),
        target_enabled: false,
        disabled_reason: None,
    };
    let error = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: "game-1".to_string(),
            mods_path: mods_path.clone(),
            operations: vec![operation.clone(), operation],
        },
    )
    .await
    .expect_err("duplicate source must be rejected");

    assert!(
        error
            .to_string()
            .contains("Duplicate mutation source path detected"),
        "{error}"
    );
    // Validation happens before any rename: disk untouched.
    assert!(mods_path.join("Variant").exists());
    assert!(!mods_path.join("DISABLED Variant").exists());
}

#[tokio::test]
async fn toggle_mods_mixed_rolls_back_filesystem_rename_when_db_commit_fails() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(mods_path.join("Orphan")).expect("mod folder");
    let mods_path_string = mods_path.to_string_lossy().to_string();

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-rollback",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("insert game");

    // No matching mods row exists, so the DB commit fails after the
    // filesystem rename already happened.
    let error = toggle_mods_mixed(
        &ctx.pool,
        RuntimeToggleBatchRequest {
            game_id: "game-rollback".to_string(),
            mods_path: mods_path.clone(),
            operations: vec![RuntimeToggleOperation {
                id: "mod-not-in-db".to_string(),
                folder_path: "Orphan".to_string(),
                target_enabled: false,
                disabled_reason: None,
            }],
        },
    )
    .await
    .expect_err("db failure must surface");

    assert!(error.to_string().contains("rollback attempted"), "{error}");
    // The folder must be restored to its original name.
    assert!(mods_path.join("Orphan").exists());
    assert!(!mods_path.join("DISABLED Orphan").exists());
}
