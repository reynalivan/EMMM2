use super::*;
use crate::repo::game_repo::{upsert_game, GameRow};
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
        game_type: crate::database::models::GameType::GIMI,
        path: "C:\\Game1".into(),
        mods_path: Some("C:\\Mods".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(&pool, &game).await.unwrap();

    // Insert whitelist pair
    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "modA",
            game_id: "g1",
            object_id: None,
            actual_name: "A",
            folder_path: "/A",
            status: crate::database::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: None,
            mods_path: Some("C:\\Mods"),
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "modB",
            game_id: "g1",
            object_id: None,
            actual_name: "B",
            folder_path: "/B",
            status: crate::database::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: None,
            mods_path: Some("C:\\Mods"),
        },
    )
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

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    // Provide job first
    sqlx::query("INSERT INTO dedup_jobs (id, status, game_id) VALUES (?, 'running', ?)")
        .bind("job1")
        .bind("g1")
        .execute(&pool)
        .await
        .unwrap();

    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "root1",
            game_id: "g1",
            object_id: None,
            actual_name: "R1",
            folder_path: "/root1",
            status: crate::database::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: None,
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    sqlx::query("INSERT INTO dedup_groups (id, job_id, resolution_status) VALUES (?, ?, ?)")
        .bind("group1")
        .bind("job1")
        .bind("pending")
        .execute(&pool)
        .await
        .unwrap();

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
