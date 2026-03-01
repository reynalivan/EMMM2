use super::*;
use crate::database::game_repo::{upsert_game, GameRow};
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

#[tokio::test]
async fn test_collection_crud() {
    let pool = setup_pool().await;
    
    // Insert game
    let game = GameRow {
        id: "g1".into(),
        name: "Game 1".into(),
        game_type: "GIMI".into(),
        path: "C:\\Game1".into(),
        mod_path: None,
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    let mut conn = pool.acquire().await.unwrap();

    // insert
    insert_collection(&mut conn, "col1", "My Collection", "g1", false).await.unwrap();
    
    // list
    let cols = list_collections(&pool, "g1", false).await.unwrap();
    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0].id, "col1");
    assert_eq!(cols[0].name, "My Collection");
    assert_eq!(cols[0].is_safe, false);

    // check exists
    let exists = check_collection_exists(&mut conn, "g1", "My Collection", false).await.unwrap();
    assert!(exists);
    
    // update name
    update_collection_name(&mut conn, "col1", "g1", "Updated Named").await.unwrap();
    let name = get_collection_name(&mut conn, "col1", "g1").await.unwrap();
    assert_eq!(name.as_deref(), Some("Updated Named"));
    
    // delete
    delete_collection(&pool, "col1", "g1").await.unwrap();
    let cols = list_collections(&pool, "g1", false).await.unwrap();
    assert!(cols.is_empty());
}
