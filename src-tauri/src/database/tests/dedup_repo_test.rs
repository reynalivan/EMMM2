use super::*;
use crate::database::game_repo::{upsert_game, GameRow};
use sqlx::SqlitePool;

async fn setup_pool() -> SqlitePool {
    let ctx = crate::test_utils::init_test_db().await;
    ctx.pool
}

#[tokio::test]
async fn test_whitelist_pairs() {
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

    // Insert whitelist pair
    sqlx::query("INSERT INTO mods (id, game_id, actual_name, folder_path, status, is_safe) VALUES (?, ?, 'A', '/A', 'ENABLED', 1), (?, ?, 'B', '/B', 'ENABLED', 1)")
        .bind("modA")
        .bind("g1")
        .bind("modB")
        .bind("g1")
        .execute(&pool)
        .await
        .unwrap();
    insert_whitelist_pair(&pool, "g1", "modA", "modB")
        .await
        .unwrap();

    // Get whitelist
    let pairs = get_duplicate_whitelist_pairs(&pool, "g1").await.unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], ("modA".to_string(), "modB".to_string()));

    // Ignore duplicate inserts
    insert_whitelist_pair(&pool, "g1", "modA", "modB")
        .await
        .unwrap();
    let pairs = get_duplicate_whitelist_pairs(&pool, "g1").await.unwrap();
    assert_eq!(pairs.len(), 1);
}

#[tokio::test]
async fn test_update_group_status() {
    let pool = setup_pool().await;

    sqlx::query(
        "INSERT INTO games (id, name, game_type, path) VALUES ('g1', 'Genshin', 'type', '/')",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Provide job first
    sqlx::query("INSERT INTO dedup_jobs (id, status, game_id) VALUES (?, ?, ?)")
        .bind("job1")
        .bind("in_progress")
        .bind("g1")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO mods (id, game_id, actual_name, folder_path, status, is_safe) VALUES ('root1', 'g1', 'R1', '/root1', 'ENABLED', 1)").execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO dedup_groups (id, job_id, resolution_status, primary_signal) VALUES (?, ?, ?, ?)"
    )
    .bind("group1")
    .bind("job1")
    .bind("pending")
    .bind("some_signal")
    .execute(&pool)
    .await.unwrap();

    // Update status
    let affected = update_group_status(&pool, "group1", "resolved", true)
        .await
        .unwrap();
    assert_eq!(affected, 1);

    // Verify
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT resolution_status, resolved_at FROM dedup_groups WHERE id = ?")
            .bind("group1")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(row.0, "resolved");
    assert!(row.1.is_some());
}
