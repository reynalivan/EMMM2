use crate::database::models::GameType;
use crate::services::disk_reconcile::reconcile::{
    reconcile_disk_projection, ReconcileDiskProjectionRequest,
};
use crate::services::disk_reconcile::types::{DiskReconcileReason, DiskReconcileStatus};
use crate::services::objects::query::{get_category_counts_service, get_object_by_id_service};
use crate::test_utils::{insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture};
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

async fn run_full_disk_reconcile(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &std::path::Path,
) {
    reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool,
        game_id,
        mods_path,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::ManualRepair,
        changed_paths: &[],
        force_full: true,
        watcher_events: None,
    })
    .await
    .expect("Disk Reconcile should succeed in test");
}

#[tokio::test]
async fn test_get_object_by_id_service() {
    let pool = setup_test_db().await;

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_get_obj",
            name: "Genshin",
            game_type: GameType::GIMI,
            path: "/game_get_obj",
            mods_path: Some("C:\\Mods".into()),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "o1",
            game_id: "g_get_obj",
            name: "MyObj",
            folder_path: Some("my_folder"),
            object_type: "Character",
        },
    )
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

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_cat_counts",
            name: "StarRail",
            game_type: GameType::GIMI,
            path: "/game_cat_counts",
            mods_path: Some("C:\\Mods".into()),
        },
    )
    .await
    .unwrap();

    // 2 Characters safe, 1 Character unsafe, 1 Weapon safe
    for (id, name, folder_path, object_type) in [
        ("o1", "C1", "c1", "Character"),
        ("o2", "C2", "c2", "Character"),
        ("o3", "C3", "c3", "Character"),
        ("o4", "W1", "w1", "Weapon"),
    ] {
        insert_test_object(
            &pool,
            &TestObjectFixture {
                id,
                game_id: "g_cat_counts",
                name,
                folder_path: Some(folder_path),
                object_type,
            },
        )
        .await
        .unwrap();
    }

    // Phase 1 fix: safe_mode no longer filters categories — always returns ALL counts.
    // The _safe_mode param is kept for API compatibility but ignored.
    let safe_counts = get_category_counts_service(&pool, "g_cat_counts", true)
        .await
        .unwrap();
    assert_eq!(safe_counts.len(), 2);
    let char_count = safe_counts
        .iter()
        .find(|c| c.object_type == "Character")
        .unwrap();
    // Now returns ALL characters (3), not just safe ones
    assert_eq!(char_count.count, 3);

    let all_counts = get_category_counts_service(&pool, "g_cat_counts", false)
        .await
        .unwrap();
    let char_count_all = all_counts
        .iter()
        .find(|c| c.object_type == "Character")
        .unwrap();
    assert_eq!(char_count_all.count, 3);
}

#[tokio::test]
async fn test_disk_reconcile_removes_missing_object_rows() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_path = temp_dir.path().join("mods_dir");
    fs::create_dir_all(&mod_path).unwrap();

    // Create a physical folder
    fs::create_dir(mod_path.join("clean_folder")).unwrap();

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_gc_lost",
            name: "ZZZ",
            game_type: GameType::GIMI,
            path: "/game_gc_lost",
            mods_path: Some(mod_path.to_str().unwrap()),
        },
    )
    .await
    .unwrap();

    // Insert object with physical folder
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "o2",
            game_id: "g_gc_lost",
            name: "Obj2",
            folder_path: Some("clean_folder"),
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    // A lost object (no physical folder)
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "o3",
            game_id: "g_gc_lost",
            name: "MissingObj",
            folder_path: Some("deleted_folder"),
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    run_full_disk_reconcile(&pool, "g_gc_lost", &mod_path).await;

    let remaining: Vec<String> =
        sqlx::query_scalar("SELECT id FROM objects WHERE game_id = 'g_gc_lost'")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0], "o2");
}

#[tokio::test]
async fn test_disk_reconcile_keeps_unicode_folder_with_ascii_case_variants() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let mod_path = temp_dir.path().join("mods_dir");
    fs::create_dir_all(&mod_path).unwrap();
    fs::create_dir(mod_path.join("한국Character")).unwrap();

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_gc_unicode",
            name: "ZZZ",
            game_type: GameType::GIMI,
            path: "/game_gc_unicode",
            mods_path: Some(mod_path.to_str().unwrap()),
        },
    )
    .await
    .unwrap();

    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "o_unicode",
            game_id: "g_gc_unicode",
            name: "한국 Character",
            folder_path: Some("한국character"),
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    run_full_disk_reconcile(&pool, "g_gc_unicode", &mod_path).await;

    let remaining: Vec<String> =
        sqlx::query_scalar("SELECT id FROM objects WHERE game_id = 'g_gc_unicode'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, vec!["o_unicode".to_string()]);
}

#[tokio::test]
async fn test_disk_reconcile_missing_mods_path_is_no_write_result() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let missing_mod_path = temp_dir.path().join("missing_mods_dir");

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_missing_path",
            name: "ZZZ",
            game_type: GameType::GIMI,
            path: "/game_missing_path",
            mods_path: Some(missing_mod_path.to_str().unwrap()),
        },
    )
    .await
    .unwrap();

    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "o_missing_path",
            game_id: "g_missing_path",
            name: "Obj",
            folder_path: Some("existing_index_row"),
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    let outcome = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id: "g_missing_path",
        mods_path: &missing_mod_path,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::ManualRepair,
        changed_paths: &[],
        force_full: true,
        watcher_events: None,
    })
    .await
    .expect("missing source should return a typed no-write result");

    assert_eq!(outcome.status, DiskReconcileStatus::SourceUnavailable);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("g_missing_path")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}
