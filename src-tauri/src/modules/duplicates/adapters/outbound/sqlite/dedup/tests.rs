use super::*;
use crate::modules::games::adapters::outbound::sqlite::game::{upsert_game, GameRow};
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
        game_type: crate::modules::games::domain::models::GameType::GIMI,
        path: "C:\\Game1".into(),
        mods_path: Some("C:\\Mods".into()),
        ready_to_move_path: None,
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
            status: crate::modules::games::domain::models::ItemStatus::Enabled,
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
            status: crate::modules::games::domain::models::ItemStatus::Enabled,
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
            game_type: crate::modules::games::domain::models::GameType::GIMI,
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
            status: crate::modules::games::domain::models::ItemStatus::Enabled,
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

#[tokio::test]
async fn completed_report_round_trips_per_game() {
    let pool = setup_pool().await;
    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "/game",
            mods_path: Some("/game/Mods"),
        },
    )
    .await
    .unwrap();

    for (id, name, path) in [("mod-a", "A", "A"), ("mod-b", "B", "B")] {
        crate::test_utils::insert_test_mod(
            &pool,
            &crate::test_utils::TestModFixture {
                id,
                game_id: "g1",
                object_id: None,
                actual_name: name,
                folder_path: path,
                status: crate::modules::games::domain::models::ItemStatus::Enabled,
                is_safe: true,
                object_type: None,
                mods_path: Some("/game/Mods"),
            },
        )
        .await
        .unwrap();
    }

    let report = crate::types::dup_scan::DupScanReport {
        scan_id: "scan-g1".to_string(),
        game_id: "g1".to_string(),
        root_path: "/game/Mods".to_string(),
        total_groups: 1,
        total_members: 2,
        groups: vec![crate::types::dup_scan::DupScanGroup {
            group_id: "scan-g1:group-1".to_string(),
            confidence_score: 100,
            match_reason: "Exact hash match".to_string(),
            is_unsafe: false,
            signals: Vec::new(),
            members: [
                ("mod-a", "/game/Mods/A", "A"),
                ("mod-b", "/game/Mods/B", "B"),
            ]
            .into_iter()
            .map(
                |(mod_id, folder_path, display_name)| crate::types::dup_scan::DupScanMember {
                    mod_id: Some(mod_id.to_string()),
                    version: None,
                    folder_path: folder_path.to_string(),
                    display_name: display_name.to_string(),
                    total_size_bytes: 10,
                    file_count: 1,
                    is_safe: true,
                    confidence_score: 100,
                    signals: Vec::new(),
                },
            )
            .collect(),
        }],
    };

    persist_completed_report(&pool, &report).await.unwrap();
    let loaded = load_latest_completed_report(&pool, "g1")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(loaded.scan_id, report.scan_id);
    assert_eq!(loaded.root_path, report.root_path);
    assert_eq!(
        serde_json::to_value(loaded.groups).unwrap(),
        serde_json::to_value(report.groups).unwrap()
    );

    update_group_status(&pool, "scan-g1:group-1", "resolved", true)
        .await
        .unwrap();
    let reloaded = load_latest_completed_report(&pool, "g1")
        .await
        .unwrap()
        .unwrap();
    assert!(reloaded.groups.is_empty());
    assert_eq!(reloaded.total_groups, 0);
    assert_eq!(reloaded.total_members, 0);
}
