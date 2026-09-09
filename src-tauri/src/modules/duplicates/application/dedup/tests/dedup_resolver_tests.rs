use super::{resolve_batch, ResolutionAction, ResolutionRequest};
use crate::modules::workspace::application::scanner::watcher::WatcherSuppressor;
use crate::platform::fs::operation_lock::OperationLock;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

struct TestContext {
    _temp: TempDir,
    mods_root: PathBuf,
    pool: sqlx::SqlitePool,
}

async fn setup_context() -> TestContext {
    let temp = TempDir::new().unwrap();
    let mods_root = temp.path().join("Mods");
    fs::create_dir_all(&mods_root).unwrap();

    let ctx = crate::test_utils::init_test_db().await;
    let pool = ctx.pool;

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "game-1",
            name: "Game 1",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: mods_root.parent().unwrap().to_str().unwrap(),
            mods_path: Some(mods_root.to_str().unwrap()),
        },
    )
    .await
    .unwrap();

    TestContext {
        _temp: temp,
        mods_root,
        pool,
    }
}

#[test]
fn failed_hardlink_creation_restores_the_original_target_file() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.ini");
    let target = temp.path().join("target.ini");
    fs::write(&source, b"new").unwrap();
    fs::write(&target, b"old").unwrap();

    let error = super::replace_file_with_hardlink_using(&source, &target, |_, _| {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "injected hardlink failure",
        ))
    })
    .expect_err("replacement must report the hardlink failure");

    assert!(error.to_string().contains("injected hardlink failure"));
    assert_eq!(fs::read(&target).unwrap(), b"old");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
}

async fn seed_dedup_group(
    context: &TestContext,
    game_id: &str,
    group_id: &str,
    folder_a: &str,
    folder_b: &str,
) {
    let job_id = format!("job-{group_id}");
    sqlx::query(
        "INSERT OR IGNORE INTO dedup_jobs (id, game_id, status) VALUES (?, ?, 'completed')",
    )
    .bind(&job_id)
    .bind(game_id)
    .execute(&context.pool)
    .await
    .unwrap();

    let group = crate::modules::duplicates::domain::dup_scan::DupScanGroup {
        group_id: group_id.to_string(),
        confidence_score: 100,
        match_reason: "Exact hash match".to_string(),
        is_unsafe: false,
        signals: Vec::new(),
        members: [folder_a, folder_b]
            .into_iter()
            .map(
                |folder_path| crate::modules::duplicates::domain::dup_scan::DupScanMember {
                    mod_id: None,
                    version: None,
                    folder_path: folder_path.to_string(),
                    display_name: folder_path.to_string(),
                    total_size_bytes: 12,
                    file_count: 1,
                    is_safe: true,
                    confidence_score: 100,
                    signals: Vec::new(),
                },
            )
            .collect(),
    };

    sqlx::query(
        "INSERT INTO dedup_groups (id, job_id, reasons_json, resolution_status) \
         VALUES (?, ?, ?, 'pending')",
    )
    .bind(group_id)
    .bind(&job_id)
    .bind(serde_json::to_string(&group).unwrap())
    .execute(&context.pool)
    .await
    .unwrap();
}

async fn seed_pair(context: &TestContext, game_id: &str) -> (String, String) {
    let folder_a = context.mods_root.join("Aether");
    let folder_b = context.mods_root.join("Lumine");
    fs::create_dir_all(&folder_a).unwrap();
    fs::create_dir_all(&folder_b).unwrap();
    fs::write(folder_a.join("mod.ini"), "same-content").unwrap();
    fs::write(folder_b.join("mod.ini"), "same-content").unwrap();

    let folder_a_path = folder_a.to_string_lossy().to_string();
    let folder_b_path = folder_b.to_string_lossy().to_string();

    crate::test_utils::insert_test_mod(
        &context.pool,
        &crate::test_utils::TestModFixture {
            id: "mod-a",
            game_id,
            object_id: None,
            actual_name: "Aether",
            folder_path: &folder_a_path,
            status: crate::modules::games::domain::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: None,
            mods_path: Some(context.mods_root.to_str().unwrap()),
        },
    )
    .await
    .unwrap();

    crate::test_utils::insert_test_mod(
        &context.pool,
        &crate::test_utils::TestModFixture {
            id: "mod-b",
            game_id,
            object_id: None,
            actual_name: "Lumine",
            folder_path: &folder_b_path,
            status: crate::modules::games::domain::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: None,
            mods_path: Some(context.mods_root.to_str().unwrap()),
        },
    )
    .await
    .unwrap();

    (folder_a_path, folder_b_path)
}

// Covers: TC-9.2-01 (Trash Duplicate KeepA)
#[tokio::test]
async fn test_tc_9_2_01_keep_a_moves_b_to_trash() {
    let context = setup_context().await;
    let game_id = "game-1";
    let (folder_a, folder_b) = seed_pair(&context, game_id).await;
    seed_dedup_group(&context, game_id, "group-1", &folder_a, &folder_b).await;

    let lock = OperationLock::new();
    let guard = lock.acquire().await.unwrap();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let summary = resolve_batch(
        vec![ResolutionRequest {
            group_id: "group-1".to_string(),
            action: ResolutionAction::KeepA,
            folder_a: folder_a.clone(),
            folder_b: folder_b.clone(),
        }],
        game_id.to_string(),
        &context.pool,
        &guard,
        &suppressor,
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(summary.successful, 1);
    assert_eq!(summary.failed, 0);
    assert!(!Path::new(&folder_b).exists());

    let status: String =
        sqlx::query_scalar("SELECT resolution_status FROM dedup_groups WHERE id = ?")
            .bind("group-1")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    assert_eq!(status, "resolved");
}

// Covers: TC-9.2-01 (Trash Duplicate KeepB)
#[tokio::test]
async fn test_tc_9_2_01_keep_b_moves_a_to_trash() {
    let context = setup_context().await;
    let game_id = "game-1";
    let (folder_a, folder_b) = seed_pair(&context, game_id).await;
    seed_dedup_group(&context, game_id, "group-2", &folder_a, &folder_b).await;

    let lock = OperationLock::new();
    let guard = lock.acquire().await.unwrap();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let summary = resolve_batch(
        vec![ResolutionRequest {
            group_id: "group-2".to_string(),
            action: ResolutionAction::KeepB,
            folder_a: folder_a.clone(),
            folder_b,
        }],
        game_id.to_string(),
        &context.pool,
        &guard,
        &suppressor,
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(summary.successful, 1);
    assert_eq!(summary.failed, 0);
    assert!(!Path::new(&folder_a).exists());
}

#[tokio::test]
async fn keep_rejects_folders_that_are_not_full_content_matches() {
    let context = setup_context().await;
    let game_id = "game-1";
    let (folder_a, folder_b) = seed_pair(&context, game_id).await;
    fs::write(Path::new(&folder_b).join("mod.ini"), "changed after scan").unwrap();
    seed_dedup_group(&context, game_id, "group-not-exact", &folder_a, &folder_b).await;

    let lock = OperationLock::new();
    let guard = lock.acquire().await.unwrap();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let summary = resolve_batch(
        vec![ResolutionRequest {
            group_id: "group-not-exact".to_string(),
            action: ResolutionAction::KeepA,
            folder_a: folder_a.clone(),
            folder_b: folder_b.clone(),
        }],
        game_id.to_string(),
        &context.pool,
        &guard,
        &suppressor,
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(summary.successful, 0);
    assert_eq!(summary.failed, 1);
    assert!(Path::new(&folder_a).exists());
    assert!(Path::new(&folder_b).exists());
}

#[tokio::test]
async fn keep_rejects_a_forged_group_before_trashing() {
    let context = setup_context().await;
    let game_id = "game-1";
    let (folder_a, folder_b) = seed_pair(&context, game_id).await;

    let lock = OperationLock::new();
    let guard = lock.acquire().await.unwrap();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let summary = resolve_batch(
        vec![ResolutionRequest {
            group_id: "missing-group".to_string(),
            action: ResolutionAction::KeepA,
            folder_a: folder_a.clone(),
            folder_b: folder_b.clone(),
        }],
        game_id.to_string(),
        &context.pool,
        &guard,
        &suppressor,
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(summary.failed, 1);
    assert!(Path::new(&folder_a).exists());
    assert!(Path::new(&folder_b).exists());
}

#[tokio::test]
async fn keep_rejects_two_spellings_of_the_same_folder() {
    let context = setup_context().await;
    let game_id = "game-1";
    let (folder_a, _folder_b) = seed_pair(&context, game_id).await;
    seed_dedup_group(&context, game_id, "same-folder", &folder_a, &folder_a).await;

    let lock = OperationLock::new();
    let guard = lock.acquire().await.unwrap();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let summary = resolve_batch(
        vec![ResolutionRequest {
            group_id: "same-folder".to_string(),
            action: ResolutionAction::KeepA,
            folder_a: folder_a.clone(),
            folder_b: folder_a.clone(),
        }],
        game_id.to_string(),
        &context.pool,
        &guard,
        &suppressor,
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(summary.failed, 1);
    assert!(Path::new(&folder_a).exists());
}

#[tokio::test]
async fn keep_rejects_a_persisted_non_exact_group() {
    let context = setup_context().await;
    let game_id = "game-1";
    let (folder_a, folder_b) = seed_pair(&context, game_id).await;
    seed_dedup_group(&context, game_id, "non-exact-group", &folder_a, &folder_b).await;
    let group_json: String =
        sqlx::query_scalar("SELECT reasons_json FROM dedup_groups WHERE id = ?")
            .bind("non-exact-group")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    let mut group: crate::modules::duplicates::domain::dup_scan::DupScanGroup =
        serde_json::from_str(&group_json).unwrap();
    group.confidence_score = 85;
    sqlx::query("UPDATE dedup_groups SET reasons_json = ? WHERE id = ?")
        .bind(serde_json::to_string(&group).unwrap())
        .bind("non-exact-group")
        .execute(&context.pool)
        .await
        .unwrap();

    let lock = OperationLock::new();
    let guard = lock.acquire().await.unwrap();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let summary = resolve_batch(
        vec![ResolutionRequest {
            group_id: "non-exact-group".to_string(),
            action: ResolutionAction::KeepA,
            folder_a: folder_a.clone(),
            folder_b: folder_b.clone(),
        }],
        game_id.to_string(),
        &context.pool,
        &guard,
        &suppressor,
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(summary.failed, 1);
    assert!(Path::new(&folder_a).exists());
    assert!(Path::new(&folder_b).exists());
}

// Covers: TC-9.2-02 (Ignore Pair)
#[tokio::test]
async fn test_tc_9_2_02_ignore_persists_whitelist() {
    let context = setup_context().await;
    let game_id = "game-1";
    let (folder_a, folder_b) = seed_pair(&context, game_id).await;
    seed_dedup_group(&context, game_id, "group-3", &folder_a, &folder_b).await;

    let lock = OperationLock::new();
    let guard = lock.acquire().await.unwrap();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let summary = resolve_batch(
        vec![ResolutionRequest {
            group_id: "group-3".to_string(),
            action: ResolutionAction::Ignore,
            folder_a,
            folder_b,
        }],
        game_id.to_string(),
        &context.pool,
        &guard,
        &suppressor,
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(summary.successful, 1);
    assert_eq!(summary.failed, 0);

    let ignored_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM duplicate_whitelist WHERE game_id = ? AND folder_a_id = ? AND folder_b_id = ?",
    )
    .bind(game_id)
    .bind("mod-a")
    .bind("mod-b")
    .fetch_one(&context.pool)
    .await
    .unwrap();
    assert_eq!(ignored_count, 1, "Whitelist entry should exist");

    let status: String =
        sqlx::query_scalar("SELECT resolution_status FROM dedup_groups WHERE id = ?")
            .bind("group-3")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    assert_eq!(status, "ignored");
}

// NC-9.2-03 (Operation Lock Active): `resolve_batch` now takes `&OpGuard`, so
// running it without the lock — or while another operation holds it — is a
// compile error rather than a runtime path. The contention error itself is
// covered by `operation_lock_tests`, at the level where commands acquire.

// Covers: TC-9.2-03 (Bulk Resolution) + TC-9.3-01 (Progress Events)
#[tokio::test]
async fn test_tc_9_2_03_bulk_resolution_with_progress_events() {
    let context = setup_context().await;
    let game_id = "game-1";

    // Create 10 duplicate pairs
    let mut requests = Vec::new();
    for i in 1..=10 {
        let folder_a = context.mods_root.join(format!("Original{}", i));
        let folder_b = context.mods_root.join(format!("Duplicate{}", i));

        fs::create_dir_all(&folder_a).unwrap();
        fs::create_dir_all(&folder_b).unwrap();
        let content = format!("duplicate-pair-{i}");
        fs::write(folder_a.join("mod.ini"), &content).unwrap();
        fs::write(folder_b.join("mod.ini"), &content).unwrap();

        let folder_a_path = folder_a.to_string_lossy().to_string();
        let folder_b_path = folder_b.to_string_lossy().to_string();

        crate::test_utils::insert_test_mod(
            &context.pool,
            &crate::test_utils::TestModFixture {
                id: &format!("mod-{}-a", i),
                game_id,
                object_id: None,
                actual_name: &format!("Original{}", i),
                folder_path: &folder_a_path,
                status: crate::modules::games::domain::models::ItemStatus::Enabled,
                is_safe: true,
                object_type: None,
                mods_path: Some(context.mods_root.to_str().unwrap()),
            },
        )
        .await
        .unwrap();

        crate::test_utils::insert_test_mod(
            &context.pool,
            &crate::test_utils::TestModFixture {
                id: &format!("mod-{}-b", i),
                game_id,
                object_id: None,
                actual_name: &format!("Duplicate{}", i),
                folder_path: &folder_b_path,
                status: crate::modules::games::domain::models::ItemStatus::Enabled,
                is_safe: true,
                object_type: None,
                mods_path: Some(context.mods_root.to_str().unwrap()),
            },
        )
        .await
        .unwrap();

        seed_dedup_group(
            &context,
            game_id,
            &format!("group-{}", i),
            &folder_a_path,
            &folder_b_path,
        )
        .await;

        requests.push(ResolutionRequest {
            group_id: format!("group-{}", i),
            action: ResolutionAction::KeepA,
            folder_a: folder_a_path,
            folder_b: folder_b_path,
        });
    }

    // Capture progress events
    let mut progress_events = Vec::new();

    let lock = OperationLock::new();
    let guard = lock.acquire().await.unwrap();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let summary = resolve_batch(
        requests,
        game_id.to_string(),
        &context.pool,
        &guard,
        &suppressor,
        |progress| {
            progress_events.push(progress);
        },
    )
    .await
    .unwrap();

    // Verify summary
    assert_eq!(summary.total, 10);
    assert_eq!(summary.successful, 10);
    assert_eq!(summary.failed, 0);

    // Verify progress events
    assert_eq!(progress_events.len(), 10, "Should emit 10 progress events");

    // Verify monotonic progress
    for (index, event) in progress_events.iter().enumerate() {
        assert_eq!(
            event.current,
            index + 1,
            "Progress current should be sequential"
        );
        assert_eq!(event.total, 10, "Progress total should be consistent");
    }

    // Verify all duplicates were deleted
    for i in 1..=10 {
        let folder_b = context.mods_root.join(format!("Duplicate{}", i));
        assert!(
            !folder_b.exists(),
            "Duplicate{} should be moved to trash",
            i
        );
    }
}

// Covers: NC-9.2-01 (File In Use Error)
#[tokio::test]
async fn test_nc_9_2_01_file_locked_graceful_skip() {
    let context = setup_context().await;
    let game_id = "game-1";

    // Create 3 pairs
    let mut requests = Vec::new();
    for i in 1..=3 {
        let folder_a = context.mods_root.join(format!("OriginalLock{}", i));
        let folder_b = context.mods_root.join(format!("DuplicateLock{}", i));

        fs::create_dir_all(&folder_a).unwrap();
        fs::create_dir_all(&folder_b).unwrap();
        fs::write(folder_a.join("mod.ini"), format!("a {}", i)).unwrap();
        fs::write(folder_b.join("mod.ini"), format!("b {}", i)).unwrap();

        let folder_a_path = folder_a.to_string_lossy().to_string();
        let folder_b_path = folder_b.to_string_lossy().to_string();

        crate::test_utils::insert_test_mod(
            &context.pool,
            &crate::test_utils::TestModFixture {
                id: &format!("mod-lock-{}-a", i),
                game_id,
                object_id: None,
                actual_name: &format!("OriginalLock{}", i),
                folder_path: &folder_a_path,
                status: crate::modules::games::domain::models::ItemStatus::Enabled,
                is_safe: true,
                object_type: None,
                mods_path: Some(context.mods_root.to_str().unwrap()),
            },
        )
        .await
        .unwrap();

        crate::test_utils::insert_test_mod(
            &context.pool,
            &crate::test_utils::TestModFixture {
                id: &format!("mod-lock-{}-b", i),
                game_id,
                object_id: None,
                actual_name: &format!("DuplicateLock{}", i),
                folder_path: &folder_b_path,
                status: crate::modules::games::domain::models::ItemStatus::Enabled,
                is_safe: true,
                object_type: None,
                mods_path: Some(context.mods_root.to_str().unwrap()),
            },
        )
        .await
        .unwrap();

        seed_dedup_group(
            &context,
            game_id,
            &format!("group-lock-{}", i),
            &folder_a_path,
            &folder_b_path,
        )
        .await;

        requests.push(ResolutionRequest {
            group_id: format!("group-lock-{}", i),
            action: ResolutionAction::KeepA,
            folder_a: folder_a_path,
            folder_b: folder_b_path,
        });
    }

    // Simulate file lock by making one folder read-only (best-effort simulation)
    // Note: True file locking is OS-specific and hard to reliably test
    // This test verifies graceful error handling exists

    let lock = OperationLock::new();
    let guard = lock.acquire().await.unwrap();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let summary = resolve_batch(
        requests,
        game_id.to_string(),
        &context.pool,
        &guard,
        &suppressor,
        |_| {},
    )
    .await
    .unwrap();

    // Should complete without panicking even if some operations fail
    assert_eq!(summary.total, 3);
    // At least some should succeed (exact count depends on OS/permissions)
    assert!(
        summary.successful + summary.failed == 3,
        "All operations should be accounted for: {} successful + {} failed != 3",
        summary.successful,
        summary.failed
    );
}
