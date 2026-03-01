use crate::database::object_repo::ObjectFilter;
use crate::services::objects::query::{
    get_category_counts_service, get_filtered_objects_with_conflict_check, get_object_by_id_service,
};
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

#[tokio::test]
async fn test_get_object_by_id_service() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'type', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type, is_safe)
         VALUES ('o1', 'g1', 'MyObj', 'my_folder', 'Character', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let obj = get_object_by_id_service(&pool, "o1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(obj.id, "o1");
    assert_eq!(obj.name, "MyObj");
    assert_eq!(obj.object_type, "Character");

    let missing = get_object_by_id_service(&pool, "o2").await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn test_get_category_counts_service() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g2', 'StarRail', 'type', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();

    // 2 Characters safe, 1 Character unsafe, 1 Weapon safe
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type, is_safe) VALUES
         ('o1', 'g2', 'C1', 'c1', 'Character', 1),
         ('o2', 'g2', 'C2', 'c2', 'Character', 1),
         ('o3', 'g2', 'C3', 'c3', 'Character', 0),
         ('o4', 'g2', 'W1', 'w1', 'Weapon', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let safe_counts = get_category_counts_service(&pool, "g2", true)
        .await
        .unwrap();
    assert_eq!(safe_counts.len(), 2);
    let char_count = safe_counts
        .iter()
        .find(|c| c.object_type == "Character")
        .unwrap();
    assert_eq!(char_count.count, 2);

    let all_counts = get_category_counts_service(&pool, "g2", false)
        .await
        .unwrap();
    let char_count_all = all_counts
        .iter()
        .find(|c| c.object_type == "Character")
        .unwrap();
    assert_eq!(char_count_all.count, 3);
}

#[tokio::test]
async fn test_get_filtered_objects_with_conflict_check() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_path = temp_dir.path().join("mods_dir");
    fs::create_dir_all(&mod_path).unwrap();

    // Create a conflict: "obj_folder" and "DISABLED obj_folder"
    fs::create_dir(mod_path.join("obj_folder")).unwrap();
    fs::create_dir(mod_path.join("DISABLED obj_folder")).unwrap();

    sqlx::query("INSERT INTO games (id, name, game_type, path, mod_path) VALUES ('g3', 'ZZZ', 'type', '/', ?)")
        .bind(mod_path.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type, is_safe)
         VALUES ('o1', 'g3', 'Obj1', 'obj_folder', 'Character', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Another object without conflict
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type, is_safe)
         VALUES ('o2', 'g3', 'Obj2', 'clean_folder', 'Character', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let filter = ObjectFilter {
        game_id: "g3".to_string(),
        search_query: None,
        object_type: None,
        safe_mode: true,
        meta_filters: None,
        sort_by: Some("name".to_string()),
        status_filter: None,
    };

    let objects = get_filtered_objects_with_conflict_check(&pool, &filter)
        .await
        .unwrap();
    assert_eq!(objects.len(), 2);

    // Sort by name ASC -> Obj1, Obj2
    assert_eq!(objects[0].id, "o1");
    assert!(
        objects[0].has_naming_conflict,
        "o1 should have a naming conflict"
    );

    assert_eq!(objects[1].id, "o2");
    assert!(
        !objects[1].has_naming_conflict,
        "o2 should not have a naming conflict"
    );
}
