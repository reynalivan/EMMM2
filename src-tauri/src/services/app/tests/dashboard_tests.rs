use crate::database::models::{GameType, ItemStatus};
use crate::services::app::dashboard::{get_active_keybindings_service, get_dashboard_payload};
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

#[tokio::test]
async fn test_get_dashboard_payload_empty() {
    let pool = setup_test_db().await;

    let payload = get_dashboard_payload(&pool, true)
        .await
        .expect("Failed to get dashboard payload");

    assert_eq!(payload.stats.total_games, 0);
    assert_eq!(payload.stats.total_mods, 0);
    assert_eq!(payload.stats.total_collections, 0);
    assert_eq!(payload.duplicate_waste_bytes, 0);
    assert!(payload.category_distribution.is_empty());
    assert!(payload.game_distribution.is_empty());
    assert!(payload.recent_mods.is_empty());
}

#[tokio::test]
async fn test_get_dashboard_payload_populated() {
    let pool = setup_test_db().await;

    // Insert dummy data
    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Game 1",
            game_type: GameType::GIMI,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_object(
        &pool,
        &crate::test_utils::TestObjectFixture {
            id: "obj1",
            game_id: "g1",
            name: "Obj 1",
            folder_path: "folder",
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "mod1",
            game_id: "g1",
            object_id: Some("obj1"),
            actual_name: "Mod 1",
            folder_path: "/tmp/mod1",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    // Size is 0 in insert_test_mod by default, let's update it if needed or just leave as is
    sqlx::query("UPDATE mods SET size_bytes = 1024 WHERE id = 'mod1'")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO collections (id, name, name_key, game_id, is_safe, is_last_unsaved) VALUES ('coll1', 'Collection 1', 'collection_1', 'g1', 1, 0)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let payload = get_dashboard_payload(&pool, true)
        .await
        .expect("Failed to get payload");

    assert_eq!(payload.stats.total_games, 1);
    assert_eq!(payload.stats.total_mods, 1);
    assert_eq!(payload.stats.enabled_mods, 1);
    assert_eq!(payload.stats.total_collections, 1);

    assert_eq!(payload.category_distribution.len(), 1);
    assert_eq!(&payload.category_distribution[0].category, "Character");
    assert_eq!(payload.category_distribution[0].count, 1);

    assert_eq!(payload.game_distribution.len(), 1);
    assert_eq!(payload.game_distribution[0].game_name, "Game 1");
    assert_eq!(payload.game_distribution[0].count, 1);

    assert_eq!(payload.recent_mods.len(), 1);
    assert_eq!(payload.recent_mods[0].name, "Mod 1");
}

#[tokio::test]
async fn test_get_active_keybindings_service_with_ini() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_dir = temp_dir.path().join("mod1");
    fs::create_dir_all(&mod_dir).unwrap();

    let ini_content = "[KeyBinding1]\nkey = F4\nback = shift";
    fs::write(mod_dir.join("mod.ini"), ini_content).unwrap();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "G1",
            game_type: GameType::GIMI,
            path: "/g1",
            mods_path: Some("/g1/Mods"),
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "mod1",
            game_id: "g1",
            object_id: None,
            actual_name: "Mod 1",
            folder_path: mod_dir.to_str().unwrap(),
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Other"),
            mods_path: Some("/g1/Mods"),
        },
    )
    .await
    .unwrap();

    let bindings = get_active_keybindings_service(&pool, "g1").await.unwrap();

    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].mod_name, "Mod 1");
    assert_eq!(bindings[0].section_name, "KeyBinding1");
    assert_eq!(bindings[0].key.as_deref(), Some("F4"));
    assert_eq!(bindings[0].back.as_deref(), Some("shift"));
}
