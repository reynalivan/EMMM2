use super::get_workspace_view_model;
use crate::database::models::GameType;
use crate::domain::workspace::{
    WorkspaceDisplayMode, WorkspaceNode, WorkspaceNodeKind, WorkspaceReasonCode,
    WorkspaceSelectionReconciliationReason, WorkspaceSelectionReconciliationStatus,
    WorkspaceSourceStatus, WorkspaceSwitchPolicyKey, WorkspaceViewModelInput, WorkspaceWarningCode,
};
use crate::repo::object_repo::ObjectFilter;
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture,
};
use std::fs;
use tempfile::TempDir;

fn build_filter(game_id: &str) -> ObjectFilter {
    ObjectFilter {
        game_id: game_id.to_string(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    }
}

fn write_file(path: &std::path::Path, content: &str) {
    fs::write(path, content).expect("write test file");
}

async fn setup_workspace_fixture(
    object_folder: &str,
) -> (sqlx::SqlitePool, TempDir, String, String) {
    let ctx = init_test_db().await;
    let mods_root = TempDir::new().expect("tempdir");
    let mods_path = mods_root.path().join("Mods");
    fs::create_dir_all(&mods_path).expect("mods root");

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game_workspace",
            name: "Test Game",
            game_type: GameType::GIMI,
            path: mods_root.path().to_string_lossy().as_ref(),
            mods_path: Some(mods_path.to_string_lossy().as_ref()),
        },
    )
    .await
    .expect("insert game");

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "obj_workspace",
            game_id: "game_workspace",
            name: object_folder,
            folder_path: object_folder,
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");

    (
        ctx.pool,
        mods_root,
        mods_path.to_string_lossy().to_string(),
        object_folder.to_string(),
    )
}

mod corridor;
mod drift;
mod preview;
