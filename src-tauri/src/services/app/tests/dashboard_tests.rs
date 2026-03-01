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
    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Game 1', 'GIMI', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO objects (id, name, game_id, object_type, folder_path) VALUES ('obj1', 'Obj 1', 'g1', 'Character', 'folder')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO mods (id, actual_name, folder_path, game_id, object_id, status, is_safe, size_bytes, object_type) 
                 VALUES ('mod1', 'Mod 1', '/tmp/mod1', 'g1', 'obj1', 'ENABLED', 1, 1024, 'Character')")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO collections (id, name, game_id) VALUES ('coll1', 'Collection 1', 'g1')",
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

    sqlx::query("INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'G1', 'GIMI', '/g1')")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO mods (id, actual_name, folder_path, game_id, status, is_safe) VALUES ('mod1', 'Mod 1', ?, 'g1', 'ENABLED', 1)")
        .bind(mod_dir.to_str().unwrap())
        .execute(&pool).await.unwrap();

    let bindings = get_active_keybindings_service(&pool, "g1").await.unwrap();

    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].mod_name, "Mod 1");
    assert_eq!(bindings[0].section_name, "KeyBinding1");
    assert_eq!(bindings[0].key.as_deref(), Some("F4"));
    assert_eq!(bindings[0].back.as_deref(), Some("shift"));
}
