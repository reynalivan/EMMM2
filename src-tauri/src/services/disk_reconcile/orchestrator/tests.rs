use std::sync::Arc;

use crate::services::disk_reconcile::types::{DiskReconcileReason, DiskReconcileStatus};
use crate::services::scanner::watcher::{ModWatchEvent, WatcherState, WatcherSuppressor};

use super::*;

fn applied_result(game_id: &str) -> crate::services::disk_reconcile::types::DiskReconcileResult {
    crate::services::disk_reconcile::types::DiskReconcileResult {
        game_id: game_id.to_string(),
        reason: DiskReconcileReason::StartupBoot,
        status: DiskReconcileStatus::Applied,
        folder_conflicts: Vec::new(),
        rename_confirmations: Vec::new(),
        error_message: None,
        changed_roots: Vec::new(),
        objects_changed: false,
        folders_changed: false,
        collections_changed: false,
        runtime_file_changed: false,
        thumbnail_roots: Vec::new(),
        cleared_selection_paths: Vec::new(),
        path_updates: Vec::new(),
        collection_reference_impact: Default::default(),
        change_summary: Default::default(),
        pending_runtime_effects: Default::default(),
        warnings: Vec::new(),
    }
}

#[test]
fn recent_applied_result_remains_available_for_activation_reuse() {
    let state = DiskReconcileState::new();
    state.record_result("game-1", &applied_result("game-1"));

    assert!(state
        .recent_applied_result("game-1", std::time::Duration::from_secs(5))
        .is_some());
}

use crate::domain::models::{GameType, ItemStatus};
use crate::services::config::ConfigService;
use crate::services::fs_utils::operation_lock::OperationLock;
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
    TestModFixture, TestObjectFixture,
};

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
    let operation_lock = OperationLock::new();

    let result = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: Arc::new(WatcherSuppressor::new(false)),
            operation_lock: &operation_lock,
            progress_reporter: None,
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
    let operation_lock = OperationLock::new();

    let result = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: Arc::new(WatcherSuppressor::new(false)),
            operation_lock: &operation_lock,
            progress_reporter: None,
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
    let operation_lock = OperationLock::new();

    let result = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: Arc::new(WatcherSuppressor::new(false)),
            operation_lock: &operation_lock,
            progress_reporter: None,
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn failed_request_does_not_leak_scope_or_path_hints_into_the_next_request() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let stale_old = mods_path.join("Alice").join("Old");
    let stale_new = mods_path.join("Alice").join("New");
    let bob = mods_path.join("Bob").join("Blue");
    std::fs::create_dir_all(&bob).expect("bob mod folder");
    std::fs::write(bob.join("mod.ini"), "[TextureOverrideBob]\nhash = abc\n").expect("bob ini");

    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();
    let operation_lock = OperationLock::new();
    let suppressor = Arc::new(WatcherSuppressor::new(false));

    let failed = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: suppressor.clone(),
            operation_lock: &operation_lock,
            progress_reporter: None,
        },
        DiskReconcileRequest::manual_with_path_hints(
            "game-1".to_string(),
            DiskReconcileReason::InternalMutation,
            vec![
                stale_old.to_string_lossy().to_string(),
                stale_new.to_string_lossy().to_string(),
            ],
            vec![DiskReconcilePathHint {
                old_path: stale_old.to_string_lossy().to_string(),
                new_path: stale_new.to_string_lossy().to_string(),
                target_object_id: "stale-object".to_string(),
            }],
        ),
    )
    .await;
    assert!(failed.is_err(), "the unregistered game must fail");

    seed_game_row(&ctx.pool, "game-1", &mods_path).await;
    let config = ConfigService::new_for_test(ctx.pool.clone());
    let result = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: suppressor,
            operation_lock: &operation_lock,
            progress_reporter: None,
        },
        DiskReconcileRequest::watcher_batch(
            "game-1".to_string(),
            vec![bob.to_string_lossy().to_string()],
            &[ModWatchEvent::Created(bob.to_string_lossy().to_string())],
        ),
    )
    .await
    .expect("the next independent request should reconcile");

    assert_eq!(result.changed_roots, vec!["Bob".to_string()]);
    assert!(result
        .path_updates
        .iter()
        .all(|update| !update.from.contains("Alice") && !update.to.contains("Alice")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serialized_public_requests_keep_their_own_watcher_events_and_path_hints() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let alice_old = mods_path.join("Alice").join("Old Blue");
    let alice = mods_path.join("Alice").join("Blue");
    let bob_old = mods_path.join("Bob").join("Old Red");
    let bob = mods_path.join("Bob").join("Red");
    for (folder, owner) in [(&alice_old, "Alice"), (&bob_old, "Bob")] {
        std::fs::create_dir_all(folder).expect("mod folder");
        std::fs::write(
            folder.join("mod.ini"),
            format!("[TextureOverride{owner}]\nhash = abc\n"),
        )
        .expect("mod ini");
    }
    seed_game_row(&ctx.pool, "game-1", &mods_path).await;
    for (id, name) in [("alice-object", "Alice"), ("bob-object", "Bob")] {
        insert_test_object(
            &ctx.pool,
            &TestObjectFixture {
                id,
                game_id: "game-1",
                name,
                folder_path: name,
                object_type: "Character",
            },
        )
        .await
        .expect("object seed");
    }
    let mods_path_text = mods_path.to_string_lossy().to_string();
    for (id, object_id, name, folder_path) in [
        ("alice-mod", "alice-object", "Old Blue", "Alice/Old Blue"),
        ("bob-mod", "bob-object", "Old Red", "Bob/Old Red"),
    ] {
        insert_test_mod(
            &ctx.pool,
            &TestModFixture {
                id,
                game_id: "game-1",
                object_id: Some(object_id),
                actual_name: name,
                folder_path,
                status: ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Character"),
                mods_path: Some(&mods_path_text),
            },
        )
        .await
        .expect("mod seed");
    }
    std::fs::rename(&alice_old, &alice).expect("rename Alice mod");
    std::fs::rename(&bob_old, &bob).expect("rename Bob mod");

    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();
    let operation_lock = OperationLock::new();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let operation_guard = operation_lock.acquire().await.expect("operation lock");

    let mut alice_request = DiskReconcileRequest::watcher_batch(
        "game-1".to_string(),
        vec![
            alice_old.to_string_lossy().to_string(),
            alice.to_string_lossy().to_string(),
        ],
        &[ModWatchEvent::Renamed {
            from: alice_old.to_string_lossy().to_string(),
            to: alice.to_string_lossy().to_string(),
        }],
    );
    alice_request.path_hints = vec![DiskReconcilePathHint {
        old_path: alice_old.to_string_lossy().to_string(),
        new_path: alice.to_string_lossy().to_string(),
        target_object_id: "alice-object".to_string(),
    }];
    let mut bob_request = DiskReconcileRequest::watcher_batch(
        "game-1".to_string(),
        vec![
            bob_old.to_string_lossy().to_string(),
            bob.to_string_lossy().to_string(),
        ],
        &[ModWatchEvent::Renamed {
            from: bob_old.to_string_lossy().to_string(),
            to: bob.to_string_lossy().to_string(),
        }],
    );
    bob_request.path_hints = vec![DiskReconcilePathHint {
        old_path: bob_old.to_string_lossy().to_string(),
        new_path: bob.to_string_lossy().to_string(),
        target_object_id: "bob-object".to_string(),
    }];

    let first = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: suppressor.clone(),
            operation_lock: &operation_lock,
            progress_reporter: None,
        },
        alice_request,
    );
    tokio::pin!(first);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut first)
            .await
            .is_err(),
        "the first request should hold the game lease while operation lock is busy"
    );

    let second = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: suppressor,
            operation_lock: &operation_lock,
            progress_reporter: None,
        },
        bob_request,
    );
    drop(operation_guard);
    let (alice_result, bob_result) = tokio::join!(first, second);
    let alice_result = alice_result.expect("Alice request");
    let bob_result = bob_result.expect("Bob request");

    assert_eq!(alice_result.changed_roots, vec!["Alice".to_string()]);
    assert_eq!(bob_result.changed_roots, vec!["Bob".to_string()]);
    assert_eq!(alice_result.path_updates.len(), 1);
    assert!(alice_result.path_updates[0].from.contains("Alice"));
    assert!(alice_result.path_updates[0].to.contains("Alice"));
    assert_eq!(bob_result.path_updates.len(), 1);
    assert!(bob_result.path_updates[0].from.contains("Bob"));
    assert!(bob_result.path_updates[0].to.contains("Bob"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn dropped_blanket_events_upgrade_the_next_scoped_request_to_full_reconcile() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    seed_game_row(&ctx.pool, "game-1", &mods_path).await;
    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "stale-bob",
            game_id: "game-1",
            name: "Bob",
            folder_path: "Bob",
            object_type: "Character",
        },
    )
    .await
    .expect("stale object");
    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();
    let operation_lock = OperationLock::new();
    let watcher = WatcherState::new();
    let watcher_session = watcher.begin_session(&mods_path);
    let suppressor = watcher.suppressor.clone();
    suppressor.mark_blanket_event_dropped(&watcher_session);

    let result = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: suppressor.clone(),
            operation_lock: &operation_lock,
            progress_reporter: None,
        },
        DiskReconcileRequest::watcher_batch(
            "game-1".to_string(),
            vec![mods_path.join("Alice").to_string_lossy().to_string()],
            &[ModWatchEvent::Modified(
                mods_path.join("Alice").to_string_lossy().to_string(),
            )],
        )
        .for_watcher_session(watcher_session),
    )
    .await
    .expect("repair reconcile");

    let stale_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE id = 'stale-bob'")
            .fetch_one(&ctx.pool)
            .await
            .expect("stale count");
    assert_eq!(result.status, DiskReconcileStatus::Applied);
    assert_eq!(stale_count, 0);
    assert!(!suppressor.has_unrepaired_drops());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconcile_waits_for_an_in_flight_filesystem_mutation() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    seed_game_row(&ctx.pool, "game-1", &mods_path).await;
    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();
    let operation_lock = OperationLock::new();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let mutation_guard = operation_lock.acquire().await.expect("mutation lock");

    let reconcile = reconcile_disk_state(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: suppressor,
            operation_lock: &operation_lock,
            progress_reporter: None,
        },
        DiskReconcileRequest::manual(
            "game-1".to_string(),
            DiskReconcileReason::ManualRepair,
            Vec::new(),
            true,
        ),
    );
    tokio::pin!(reconcile);

    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut reconcile)
            .await
            .is_err(),
        "reconcile must not observe the filesystem mid-mutation"
    );

    drop(mutation_guard);
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), &mut reconcile)
        .await
        .expect("reconcile should resume after mutation")
        .expect("reconcile should succeed");
    assert_eq!(result.status, DiskReconcileStatus::Applied);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconcile_and_collection_apply_do_not_deadlock_on_inverted_locks() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let disabled_mod = mods_path.join("Alice").join("DISABLED Blue Dress");
    std::fs::create_dir_all(&disabled_mod).expect("disabled mod folder");
    std::fs::write(
        disabled_mod.join("mod.ini"),
        "[TextureOverrideAlice]\nhash = abc\n",
    )
    .expect("ini");
    seed_game_row(&ctx.pool, "game-1", &mods_path).await;
    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "object-1",
            game_id: "game-1",
            name: "Alice",
            folder_path: "Alice",
            object_type: "Character",
        },
    )
    .await
    .expect("object seed");
    let mods_path_text = mods_path.to_string_lossy().to_string();
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-1",
            game_id: "game-1",
            object_id: Some("object-1"),
            actual_name: "Blue Dress",
            folder_path: "Alice/DISABLED Blue Dress",
            status: ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_text),
        },
    )
    .await
    .expect("mod seed");

    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();
    let operation_lock = OperationLock::new();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let game_lock = state.game_lock("game-1");
    let mut apply_context = crate::pipeline::apply_pipeline::ApplyContext::new(
        crate::services::collection_service::ApplyCollectionRequest {
            pool: &ctx.pool,
            game_id: "game-1",
            collection_id: "collection-1",
            capture_last_changes: false,
            mods_path: mods_path.clone(),
            suppressor: suppressor.clone(),
            ignore_missing: false,
            settings: config.get_settings(),
        },
    );
    apply_context.to_enable = vec![crate::common::path_key::folder_path_key(
        "Alice/Blue Dress",
        Some(&mods_path_text),
    )];

    // Match the collection command boundary: it owns the operation lease
    // before the pipeline reaches its inline per-game reconcile.
    let operation_guard = operation_lock.acquire().await.expect("operation lock");
    let start = Arc::new(tokio::sync::Barrier::new(2));
    let reconcile_start = start.clone();
    let reconcile = async {
        reconcile_start.wait().await;
        reconcile_disk_state(
            DiskReconcileContext {
                pool: &ctx.pool,
                config: &config,
                state: &state,
                watcher_suppressor: suppressor,
                operation_lock: &operation_lock,
                progress_reporter: None,
            },
            DiskReconcileRequest::manual(
                "game-1".to_string(),
                DiskReconcileReason::ManualRepair,
                Vec::new(),
                true,
            ),
        )
        .await
    };
    let apply = async {
        start.wait().await;

        // Do not start the inline apply reconcile until the public reconcile
        // demonstrably owns the game lease and is waiting on our operation
        // lease. Yielding is only scheduler cooperation, not timing input.
        while let Ok(guard) = game_lock.try_lock() {
            drop(guard);
            tokio::task::yield_now().await;
        }

        let result = crate::pipeline::steps::batch_rename::rename(&mut apply_context).await;
        drop(operation_guard);
        result
    };

    let (reconcile_result, apply_result) =
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            tokio::join!(reconcile, apply)
        })
        .await
        .expect("reconcile/apply lock inversion must not deadlock");

    reconcile_result.expect("public reconcile should complete");
    apply_result.expect("collection apply rename should complete");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconcile_under_existing_locks_keeps_the_operation_lease_owned_by_the_caller() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    seed_game_row(&ctx.pool, "game-1", &mods_path).await;
    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();
    let operation_lock = OperationLock::new();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let game_lock = state.game_lock("game-1");
    let game_guard = game_lock.lock().await;
    let operation_guard = operation_lock.acquire().await.expect("operation lock");

    let result = reconcile_disk_state_under_locks(
        DiskReconcileContext {
            pool: &ctx.pool,
            config: &config,
            state: &state,
            watcher_suppressor: suppressor,
            operation_lock: &operation_lock,
            progress_reporter: None,
        },
        DiskReconcileRequest::manual(
            "game-1".to_string(),
            DiskReconcileReason::ManualRepair,
            Vec::new(),
            true,
        ),
        &game_guard,
        &operation_guard,
    )
    .await
    .expect("reconcile should succeed under caller-owned locks");

    assert_eq!(result.status, DiskReconcileStatus::Applied);
    assert!(
        operation_lock.acquire().await.is_err(),
        "the caller must still own the operation lease after reconcile"
    );

    drop(operation_guard);
    drop(game_guard);
    operation_lock
        .acquire()
        .await
        .expect("operation lease should be released by the caller");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconcile_under_mutation_lease_does_not_reacquire_or_release_either_lock() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    seed_game_row(&ctx.pool, "game-lease", &mods_path).await;
    let config = ConfigService::new_for_test(ctx.pool.clone());
    let state = DiskReconcileState::new();
    let operation_lock = OperationLock::new();
    let suppressor = Arc::new(WatcherSuppressor::new(false));
    let lease = state
        .acquire_mutation_lease("game-lease", &operation_lock)
        .await
        .expect("mutation lease");

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        super::entry::reconcile_disk_state_under_lease(
            DiskReconcileContext {
                pool: &ctx.pool,
                config: &config,
                state: &state,
                watcher_suppressor: suppressor,
                operation_lock: &operation_lock,
                progress_reporter: None,
            },
            DiskReconcileRequest::manual(
                "game-lease".to_string(),
                DiskReconcileReason::ManualRepair,
                Vec::new(),
                true,
            ),
            &lease,
        ),
    )
    .await
    .expect("under-lease reconcile must not deadlock")
    .expect("under-lease reconcile should succeed");

    assert_eq!(result.status, DiskReconcileStatus::Applied);
    assert!(
        operation_lock.acquire().await.is_err(),
        "terminal reconcile must leave the caller's mutation lease held"
    );
    drop(lease);
    operation_lock
        .acquire()
        .await
        .expect("dropping the lease should release the operation lock");
}
