use super::{default_switch_refresh_scopes, resolve_mod_target_path, WorkspaceRefreshScope};
use crate::domain::models::{GameType, ItemStatus};
use crate::test_utils::{
    insert_test_game, insert_test_mod, insert_test_object, TestGameFixture, TestModFixture,
    TestObjectFixture,
};

#[test]
fn switch_refresh_scopes_include_collections_for_unsaved_corridor_counts() {
    let scopes = default_switch_refresh_scopes();

    assert!(scopes.contains(&WorkspaceRefreshScope::CollectionsChanged));
}

#[test]
fn switch_impact_rewrites_original_cache_path_to_resolved_path() {
    let impact = super::build_switch_impact(
        Some("Alice/DISABLED Blue Dress"),
        Some("Alice/Blue Dress"),
        &["Alice/Blue Dress".to_string()],
        &["o1".to_string()],
    );

    assert_eq!(impact.rewrites.len(), 1);
    assert_eq!(impact.rewrites[0].old_path, "Alice/DISABLED Blue Dress");
    assert_eq!(impact.rewrites[0].new_path, "Alice/Blue Dress");
}

#[tokio::test]
async fn resolves_stale_disabled_cache_path_to_enabled_disk_sibling() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mods_path = temp_dir.path().join("mods");
    let enabled_path = mods_path.join("Alice").join("Blue Dress");
    std::fs::create_dir_all(&enabled_path).expect("enabled mod folder should exist");

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_switch_resolve_enabled",
            name: "ZZZ",
            game_type: GameType::GIMI,
            path: "/game_switch_resolve_enabled",
            mods_path: Some(mods_path.to_str().unwrap()),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "o_switch_resolve_enabled",
            game_id: "g_switch_resolve_enabled",
            name: "Alice",
            folder_path: "Alice",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    insert_test_mod(
        &pool,
        &TestModFixture {
            id: "m_switch_resolve_enabled",
            game_id: "g_switch_resolve_enabled",
            object_id: Some("o_switch_resolve_enabled"),
            actual_name: "Blue Dress",
            folder_path: "Alice/DISABLED Blue Dress",
            status: ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(mods_path.to_str().unwrap()),
        },
    )
    .await
    .unwrap();

    let stale_path = mods_path.join("Alice").join("DISABLED Blue Dress");
    let (resolved_path, changed_object_ids) = resolve_mod_target_path(
        &pool,
        "g_switch_resolve_enabled",
        &stale_path.to_string_lossy(),
        true,
    )
    .await
    .unwrap();

    assert_eq!(resolved_path, enabled_path.to_string_lossy());
    assert_eq!(
        changed_object_ids,
        vec!["o_switch_resolve_enabled".to_string()]
    );
}

#[tokio::test]
async fn resolves_stale_enabled_cache_path_to_disabled_disk_sibling() {
    let pool = crate::test_utils::init_test_db().await.pool;
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mods_path = temp_dir.path().join("mods");
    let disabled_path = mods_path.join("Alice").join("DISABLED Blue Dress");
    std::fs::create_dir_all(&disabled_path).expect("disabled mod folder should exist");

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_switch_resolve_disabled",
            name: "ZZZ",
            game_type: GameType::GIMI,
            path: "/game_switch_resolve_disabled",
            mods_path: Some(mods_path.to_str().unwrap()),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "o_switch_resolve_disabled",
            game_id: "g_switch_resolve_disabled",
            name: "Alice",
            folder_path: "Alice",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    insert_test_mod(
        &pool,
        &TestModFixture {
            id: "m_switch_resolve_disabled",
            game_id: "g_switch_resolve_disabled",
            object_id: Some("o_switch_resolve_disabled"),
            actual_name: "Blue Dress",
            folder_path: "Alice/Blue Dress",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(mods_path.to_str().unwrap()),
        },
    )
    .await
    .unwrap();

    let stale_path = mods_path.join("Alice").join("Blue Dress");
    let (resolved_path, changed_object_ids) = resolve_mod_target_path(
        &pool,
        "g_switch_resolve_disabled",
        &stale_path.to_string_lossy(),
        false,
    )
    .await
    .unwrap();

    assert_eq!(resolved_path, disabled_path.to_string_lossy());
    assert_eq!(
        changed_object_ids,
        vec!["o_switch_resolve_disabled".to_string()]
    );
}
