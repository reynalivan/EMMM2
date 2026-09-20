use super::{
    get_workspace_preview, get_workspace_structure, get_workspace_view_model,
    get_workspace_view_model_with_listing_mode,
};
use crate::modules::catalog::domain::objects::ObjectFilter;
use crate::modules::games::domain::models::{GameType, ItemStatus};
use crate::modules::workspace::domain::workspace::{
    WorkspaceDisplayMode, WorkspaceExplorerPageInput, WorkspaceExplorerQuery,
    WorkspaceExplorerSafetyFilter, WorkspaceExplorerSortField, WorkspaceExplorerSortOrder,
    WorkspaceNode, WorkspaceNodeKind, WorkspacePreviewContextStatus, WorkspacePreviewInput,
    WorkspaceReasonCode, WorkspaceSelectionReconciliationReason,
    WorkspaceSelectionReconciliationStatus, WorkspaceSourceStatus, WorkspaceStructureInput,
    WorkspaceSwitchPolicyKey, WorkspaceViewModelInput, WorkspaceWarningCode,
};
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
    TestModFixture, TestObjectFixture,
};
use std::fs;
use tempfile::TempDir;

fn build_filter(game_id: &str) -> ObjectFilter {
    ObjectFilter {
        game_id: game_id.to_string(),
        ..Default::default()
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

mod drift;
mod preview;
mod roots;
mod safety_filter;
