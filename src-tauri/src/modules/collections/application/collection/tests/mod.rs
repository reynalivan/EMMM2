use super::projection::persist_projected_state;
use super::{
    apply_collection, capture_last_changes_if_needed, collection_preflight_scope_paths,
    create_collection, delete_collection, get_collection_preview, handle_mod_missing,
    handle_mod_moved_or_renamed, handle_object_renamed_tx, list_collections, preview_apply,
    replace_collection_with_current_state, update_collection, ApplyCollectionRequest,
};
use crate::modules::collections::adapters::sqlite as collection;
use crate::modules::collections::domain::collection::{
    CollectionMod, CollectionObject, CreateCollectionInput, CreateCollectionMode, MemberKind,
    ProjectedCollectionState, ProjectedStateSummary, UpdateCollectionInput,
};
use crate::modules::games::domain::models::{GameType, ItemStatus};
use crate::modules::settings::application::config::AppSettings;
use crate::modules::workspace::application::projected_state;
use crate::modules::workspace::application::scanner::watcher::WatcherSuppressor;
use crate::shared::errors::CollectionError;
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
    TestModFixture, TestObjectFixture,
};
use std::sync::Arc;

// ── Shared fixtures ─────────────────────────────────────────────────────────

/// The one game every test in this module uses.
async fn seed_game(pool: &sqlx::SqlitePool, id: &str, mods_path: Option<&str>) {
    insert_test_game(
        pool,
        &TestGameFixture {
            id,
            name: id,
            game_type: GameType::GIMI,
            path: if id == "game-1" {
                "E:/Games/TestGame"
            } else {
                "E:/Games/TestGame2"
            },
            mods_path,
        },
    )
    .await
    .expect("insert game");
}

/// The one character object every test in this module uses.
async fn seed_ainoz_object(pool: &sqlx::SqlitePool, id: &str, game_id: &str) {
    insert_test_object(
        pool,
        &TestObjectFixture {
            id,
            game_id,
            name: "AINOZ",
            folder_path: "AINOZ",
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");
}

fn test_collection_mod(collection_id: &str, mod_path: &str, display_name: &str) -> CollectionMod {
    CollectionMod {
        kind: MemberKind::Mod,
        collection_id: collection_id.to_string(),
        mod_id: None,
        mod_path: mod_path.to_string(),
        mod_path_key: Some(crate::shared::path_key::folder_path_key(mod_path, None)),
        object_id: "object-1".to_string(),
        display_name: Some(display_name.to_string()),
        preview_path: Some(mod_path.to_string()),
        node_type: Some("FlatModRoot".to_string()),
        warnings: Vec::new(),
        is_enabled: true,
        is_safe: true,
        safety_source: Some("manual".to_string()),
    }
}

fn test_collection_object(collection_id: &str) -> CollectionObject {
    CollectionObject {
        kind: MemberKind::Object,
        collection_id: collection_id.to_string(),
        object_id: "object-1".to_string(),
        is_enabled: true,
        display_name: Some("AINOZ".to_string()),
        path_key: Some("AINOZ".to_string()),
    }
}

fn create_flat_mod_folder(mods_root: &std::path::Path, relative_path: &str) {
    let target = mods_root.join(relative_path);
    std::fs::create_dir_all(&target).expect("create flat mod folder");
    std::fs::write(
        target.join("mod.ini"),
        "[TextureOverrideUnitTest]\nhash = 12345678\n",
    )
    .expect("write mod ini");
}

mod apply_tests;
mod characterization_tests;
mod create_tests;
mod delete_tests;
mod lifecycle_tests;
mod preview_tests;
mod references_tests;
mod replace_tests;
mod update_tests;
