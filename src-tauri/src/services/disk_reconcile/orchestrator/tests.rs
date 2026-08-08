use std::sync::Arc;

use crate::services::disk_reconcile::types::{DiskReconcileReason, DiskReconcileStatus};
use crate::services::scanner::watcher::{ModWatchEvent, WatcherSuppressor};

use super::*;

#[test]
fn enqueue_request_preserves_watcher_events_across_coalesced_batches() {
    let state = DiskReconcileState::new();
    let first_events = vec![ModWatchEvent::Renamed {
        from: "E:/Mods/Alice/Old".to_string(),
        to: "E:/Mods/Alice/New".to_string(),
    }];
    let second_events = vec![ModWatchEvent::Created("E:/Mods/Bob/Blue".to_string())];

    state.enqueue_request(
        "game-1",
        DiskReconcileReason::WatcherBatch,
        &["E:/Mods/Alice/Old".to_string()],
        false,
        &first_events,
    );
    let version = state.enqueue_request(
        "game-1",
        DiskReconcileReason::WatcherBatch,
        &["E:/Mods/Bob/Blue".to_string()],
        false,
        &second_events,
    );

    let request = state
        .take_pending_or_cached("game-1", version)
        .expect("pending request should be readable")
        .expect("pending request should exist");

    assert_eq!(request.watcher_events.len(), 2);
    assert!(matches!(
        request.watcher_events[0],
        ModWatchEvent::Renamed { .. }
    ));
    assert!(matches!(
        request.watcher_events[1],
        ModWatchEvent::Created(_)
    ));
}

#[test]
fn requeue_pending_restores_taken_request_after_failed_run() {
    let state = DiskReconcileState::new();
    let version = state.enqueue_request(
        "game-1",
        DiskReconcileReason::WatcherBatch,
        &["E:/Mods/Alice/Old".to_string()],
        false,
        &[ModWatchEvent::Removed("E:/Mods/Alice/Old".to_string())],
    );

    let taken = state
        .take_pending_or_cached("game-1", version)
        .expect("pending should be readable")
        .expect("pending should exist");

    // A new request arrives while the failed run is being unwound.
    let newer_version = state.enqueue_request(
        "game-1",
        DiskReconcileReason::WatcherBatch,
        &["E:/Mods/Bob/Blue".to_string()],
        true,
        &[ModWatchEvent::Created("E:/Mods/Bob/Blue".to_string())],
    );

    state.requeue_pending("game-1", taken);

    let merged = state
        .take_pending_or_cached("game-1", newer_version)
        .expect("pending should be readable")
        .expect("requeued request should exist");

    assert!(merged.force_full);
    assert_eq!(merged.max_version, newer_version);
    assert_eq!(merged.changed_paths.len(), 2);
    assert_eq!(merged.watcher_events.len(), 2);
    assert!(matches!(
        merged.watcher_events[0],
        ModWatchEvent::Removed(_)
    ));
    assert!(matches!(
        merged.watcher_events[1],
        ModWatchEvent::Created(_)
    ));
}

use crate::domain::models::GameType;
use crate::services::config::ConfigService;
use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};

async fn seed_game_row(pool: &sqlx::SqlitePool, game_id: &str, mods_path: &std::path::Path) {
    insert_test_game(
        pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::GIMI,
            path: mods_path.parent().unwrap().to_string_lossy().as_ref(),
            mods_path: Some(mods_path.to_string_lossy().as_ref()),
        },
    )
    .await
    .expect("game seed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn reconcile_disk_state_errors_when_game_is_not_registered() {
    let ctx = init_test_db().await;
    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();

    let result = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: Arc::new(WatcherSuppressor::new(false)),
        },
        DiskReconcileRequest::manual(
            "missing-game".to_string(),
            DiskReconcileReason::ManualRepair,
            Vec::new(),
            false,
        ),
    )
    .await;

    let error = result.expect_err("unknown game must error").to_string();
    assert!(error.contains("not found for disk reconcile"), "{error}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn reconcile_disk_state_reports_source_unavailable_for_missing_mods_path() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let missing_mods_path = temp.path().join("Missing Mods");
    seed_game_row(&ctx.pool, "game-1", &missing_mods_path).await;
    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();

    let result = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: Arc::new(WatcherSuppressor::new(false)),
        },
        DiskReconcileRequest::manual(
            "game-1".to_string(),
            DiskReconcileReason::ManualRepair,
            Vec::new(),
            false,
        ),
    )
    .await
    .expect("source-unavailable is a result, not an error");

    assert_eq!(result.status, DiskReconcileStatus::SourceUnavailable);
    assert!(result
        .error_message
        .as_deref()
        .unwrap_or_default()
        .contains("unavailable"));
    assert!(!result.objects_changed);
    assert!(!result.folders_changed);
    assert!(!result.collections_changed);
    assert!(!result.overlay_refresh_triggered);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn reconcile_disk_state_applies_new_disk_folders_end_to_end() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let terminal = mods_path.join("Alice").join("Blue Dress");
    std::fs::create_dir_all(&terminal).expect("mod folder");
    std::fs::write(
        terminal.join("mod.ini"),
        "[TextureOverrideAlice]\nhash = abc\n",
    )
    .expect("ini");
    std::fs::write(terminal.join("mesh.buf"), "mesh").expect("asset");
    seed_game_row(&ctx.pool, "game-1", &mods_path).await;
    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();

    let result = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: Arc::new(WatcherSuppressor::new(false)),
        },
        DiskReconcileRequest::manual(
            "game-1".to_string(),
            DiskReconcileReason::ManualRepair,
            Vec::new(),
            false,
        ),
    )
    .await
    .expect("reconcile should succeed");

    assert_eq!(result.status, DiskReconcileStatus::Applied);
    assert!(result.objects_changed);
    assert!(result.folders_changed);
    assert!(result.collections_changed);
    assert!(result.changed_roots.contains(&"Alice".to_string()));
    assert!(result.change_summary.has_user_visible_changes);

    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("game-1")
        .fetch_one(&ctx.pool)
        .await
        .expect("object count");
    let mod_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = ?")
        .bind("game-1")
        .fetch_one(&ctx.pool)
        .await
        .expect("mod count");
    assert_eq!(object_count, 1);
    assert_eq!(mod_count, 1);
}
