//! Pins the scoped projection-refresh branch: a scoped reconcile must refresh
//! only the touched objects' runtime projection rows and leave the rest alone.

use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::modules::games::domain::models::GameType;
use crate::modules::reconciliation::application::disk_reconcile::reconcile::ReconcileOutcome;
use crate::modules::reconciliation::application::disk_reconcile::reconcile::{
    begin_projection_write_transaction, reconcile_disk_projection, ReconcileDiskProjectionRequest,
};
use crate::modules::reconciliation::application::disk_reconcile::types::{
    DiskReconcileReason, DiskReconcileScanScope, DiskReconcileStatus,
};
use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
struct PersistedModSnapshot {
    id: String,
    object_id: Option<String>,
    folder_path: String,
    folder_path_key: String,
    actual_name: String,
    status: i64,
    object_type: Option<String>,
    is_safe: i64,
    safety_source: String,
    content_hash: Option<String>,
    size_bytes: i64,
    filesystem_identity: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

fn create_terminal_mod(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::write(path.join("mod.ini"), "[TextureOverride]\nhash = abc\n").unwrap();
    fs::write(path.join("mesh.buf"), "mesh").unwrap();
}

async fn run_reconcile(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    reason: DiskReconcileReason,
    changed_paths: &[String],
    force_full: bool,
) -> ReconcileOutcome {
    reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool,
        game_id,
        mods_path,
        safe_mode_keywords: &[],
        reason: &reason,
        changed_paths,
        force_full,
        watcher_events: None,
        path_hints: &[],
        trusted_mutation_scope: false,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .expect("reconcile should succeed")
}

async fn projection_row(pool: &sqlx::SqlitePool, game_id: &str, folder_path: &str) -> Option<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT p.is_object_disabled
         FROM object_runtime_projection p
         JOIN objects o ON o.id = p.object_id
         WHERE o.game_id = ? AND o.folder_path = ?",
    )
    .bind(game_id)
    .bind(folder_path)
    .fetch_optional(pool)
    .await
    .expect("projection query should succeed")
}

async fn mod_row(pool: &sqlx::SqlitePool, game_id: &str) -> Option<(String, i64)> {
    sqlx::query_as(
        "SELECT folder_path, status FROM mods WHERE game_id = ? AND actual_name = 'Blue'",
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .expect("mod row should load")
}

async fn mod_row_named(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    actual_name: &str,
) -> Option<String> {
    sqlx::query_scalar("SELECT id FROM mods WHERE game_id = ? AND actual_name = ?")
        .bind(game_id)
        .bind(actual_name)
        .fetch_optional(pool)
        .await
        .expect("mod row should load")
}

async fn persisted_mod_snapshot(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    actual_name: &str,
) -> PersistedModSnapshot {
    sqlx::query_as(
        "SELECT id, object_id, folder_path, folder_path_key, actual_name, status,
                object_type, is_safe, safety_source, content_hash, size_bytes,
                filesystem_identity, created_at, updated_at
         FROM mods
         WHERE game_id = ? AND actual_name = ?",
    )
    .bind(game_id)
    .bind(actual_name)
    .fetch_one(pool)
    .await
    .expect("persisted mod snapshot should load")
}

async fn runtime_signature(pool: &sqlx::SqlitePool, game_id: &str) -> String {
    crate::modules::collections::application::runtime::get_collection_runtime_state(pool, game_id)
        .await
        .expect("runtime state should load")
        .current_signature
}

#[tokio::test]
async fn pending_toggle_burst_converges_after_a_scoped_sqlite_retry() {
    use crate::modules::library::api::mods::core_ops::plan_toggle_rename;
    use crate::modules::mutation::api::{MutationCoordinator, OperationPlan, PlannedStep};
    use crate::modules::mutation::journal::{OperationJournal, OperationStatus};
    use crate::modules::reconciliation::application::toggle_projection::{
        proven_projection_events, trusted_projection_paths, validate_projected_toggle_rows,
    };
    use crate::platform::fs::file_utils::filesystem_identity;
    use crate::platform::fs::operation_lock::OperationLock;

    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let blue = root.join("Alice").join("Blue");
    let red = root.join("Bob").join("DISABLED Red");
    create_terminal_mod(&blue);
    create_terminal_mod(&red);
    create_terminal_mod(&root.join("Charlie").join("Untouched"));
    let game_id = "g_pending_toggle_burst";
    let root_text = root.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&root_text),
        },
    )
    .await
    .unwrap();
    run_reconcile(
        &pool,
        game_id,
        &root,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    let original_blue = persisted_mod_snapshot(&pool, game_id, "Blue").await;
    let original_red = persisted_mod_snapshot(&pool, game_id, "Red").await;
    let untouched = persisted_mod_snapshot(&pool, game_id, "Untouched").await;
    let epoch = filesystem_identity(&root).unwrap();
    let journal = std::sync::Arc::new(
        OperationJournal::open(temp.path().join("burst-journal.json"), 16).unwrap(),
    );
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());

    // Exercise the real storage/journal boundary while projection has not run.
    for (path, enable) in [
        (blue.clone(), false),
        (blue.with_file_name("DISABLED Blue"), true),
        (blue.clone(), false),
        (red.clone(), true),
    ] {
        let rename = plan_toggle_rename(&path, enable).unwrap().unwrap();
        let operation = coordinator
            .acquire_operation(
                OperationPlan::new(
                    "workspace-switch",
                    game_id,
                    vec![PlannedStep::rename(
                        0,
                        rename.old_path().to_path_buf(),
                        rename.new_path().to_path_buf(),
                    )
                    .with_expected_identity(Some(rename.expected_identity().to_owned()))],
                )
                .with_source_epoch(epoch.clone()),
            )
            .await
            .unwrap();
        rename.apply("mod").unwrap();
        operation.mark_step_applied(0).unwrap();
        operation.mark_disk_committed().unwrap();
    }
    assert_eq!(
        persisted_mod_snapshot(&pool, game_id, "Blue").await,
        original_blue
    );
    assert_eq!(
        persisted_mod_snapshot(&pool, game_id, "Red").await,
        original_red
    );
    let pending = coordinator.pending_disk_commits().unwrap();
    assert_eq!(pending.len(), 4);
    assert!(pending
        .iter()
        .all(|operation| operation.status == OperationStatus::DiskCommitted));
    assert!(
        validate_projected_toggle_rows(&pool, game_id, &pending, &root)
            .await
            .is_err()
    );
    let events = proven_projection_events(&pending, &root).unwrap();
    let paths = trusted_projection_paths(&pending).unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_burst_projection BEFORE INSERT ON object_runtime_projection
         BEGIN SELECT RAISE(FAIL, 'transient burst projection failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let failed = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id,
        mods_path: &root,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::InternalMutation,
        changed_paths: &paths,
        force_full: false,
        watcher_events: Some(&events),
        path_hints: &[],
        trusted_mutation_scope: true,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .unwrap_err();
    assert!(failed
        .to_string()
        .contains("transient burst projection failure"));
    assert_eq!(
        persisted_mod_snapshot(&pool, game_id, "Blue").await,
        original_blue
    );
    assert_eq!(
        persisted_mod_snapshot(&pool, game_id, "Red").await,
        original_red
    );
    assert!(blue.with_file_name("DISABLED Blue").is_dir());
    assert!(red.with_file_name("Red").is_dir());
    assert_eq!(coordinator.pending_disk_commits().unwrap().len(), 4);

    let rename = plan_toggle_rename(&blue.with_file_name("DISABLED Blue"), true)
        .unwrap()
        .unwrap();
    let operation = coordinator
        .acquire_operation(
            OperationPlan::new(
                "workspace-switch",
                game_id,
                vec![PlannedStep::rename(
                    0,
                    rename.old_path().to_path_buf(),
                    rename.new_path().to_path_buf(),
                )
                .with_expected_identity(Some(rename.expected_identity().to_owned()))],
            )
            .with_source_epoch(epoch),
        )
        .await
        .unwrap();
    rename.apply("mod").unwrap();
    operation.mark_step_applied(0).unwrap();
    operation.mark_disk_committed().unwrap();
    drop(operation);
    sqlx::query("DROP TRIGGER fail_burst_projection")
        .execute(&pool)
        .await
        .unwrap();
    let pending = coordinator.pending_disk_commits().unwrap();
    assert_eq!(pending.len(), 5);
    let events = proven_projection_events(&pending, &root).unwrap();
    let paths = trusted_projection_paths(&pending).unwrap();
    let outcome = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id,
        mods_path: &root,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::InternalMutation,
        changed_paths: &paths,
        force_full: false,
        watcher_events: Some(&events),
        path_hints: &[],
        trusted_mutation_scope: true,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .unwrap();
    assert_eq!(outcome.scan_scope, DiskReconcileScanScope::Scoped);
    assert_eq!(outcome.status, DiskReconcileStatus::Applied);
    validate_projected_toggle_rows(&pool, game_id, &pending, &root)
        .await
        .unwrap();
    let projected_blue = persisted_mod_snapshot(&pool, game_id, "Blue").await;
    let projected_red = persisted_mod_snapshot(&pool, game_id, "Red").await;
    assert_eq!(projected_blue.id, original_blue.id);
    assert_eq!(projected_red.id, original_red.id);
    assert_eq!(
        projected_blue.filesystem_identity,
        original_blue.filesystem_identity
    );
    assert_eq!(
        projected_red.filesystem_identity,
        original_red.filesystem_identity
    );
    assert_eq!(projected_blue.status, 1);
    assert_eq!(projected_red.status, 1);
    assert_eq!(
        Path::new(&projected_blue.folder_path),
        blue.strip_prefix(&root).unwrap()
    );
    assert_eq!(
        Path::new(&projected_red.folder_path),
        red.with_file_name("Red").strip_prefix(&root).unwrap()
    );
    assert_eq!(
        persisted_mod_snapshot(&pool, game_id, "Untouched").await,
        untouched
    );
    coordinator
        .complete_disk_projection(
            &pending
                .iter()
                .map(|operation| operation.id.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    assert!(coordinator.pending_disk_commits().unwrap().is_empty());
    assert!(journal
        .entries()
        .iter()
        .all(|operation| operation.status == OperationStatus::Completed));
}

#[tokio::test]
async fn disabled_ancestors_agree_across_reconcile_capture_and_apply_preview() {
    use crate::modules::collections::api::testing::domain::collection::CreateCollectionInput;
    use crate::modules::collections::api::{collection, runtime};

    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    create_terminal_mod(&root.join("Alice").join("Blue"));
    create_terminal_mod(&root.join("Alice").join("DISABLED Pack").join("Nested"));
    create_terminal_mod(&root.join("Bob").join("Green"));
    let game_id = "g_disabled_ancestor_capture";
    let root_text = root.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&root_text),
        },
    )
    .await
    .unwrap();
    run_reconcile(
        &pool,
        game_id,
        &root,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    let nested = persisted_mod_snapshot(&pool, game_id, "Nested").await;
    assert_eq!(
        nested.status, 1,
        "the terminal folder itself is locally enabled"
    );
    let before = runtime::get_collection_runtime_state(&pool, game_id)
        .await
        .unwrap();
    assert_eq!(before.current_mods.len(), 2);
    assert!(before
        .current_mods
        .iter()
        .all(|member| member.mod_id.as_ref() != Some(&nested.id)));

    let old = root.join("Alice");
    let disabled = root.join("DISABLED Alice");
    fs::rename(&old, &disabled).unwrap();
    let paths = [
        old.to_string_lossy().to_string(),
        disabled.to_string_lossy().to_string(),
    ];
    let outcome = run_reconcile(
        &pool,
        game_id,
        &root,
        DiskReconcileReason::InternalMutation,
        &paths,
        false,
    )
    .await;
    assert_eq!(outcome.scan_scope, DiskReconcileScanScope::Scoped);
    let blue = persisted_mod_snapshot(&pool, game_id, "Blue").await;
    assert_eq!(
        blue.status, 1,
        "disabling an ancestor does not change a child's local status"
    );
    let current = runtime::get_collection_runtime_state(&pool, game_id)
        .await
        .unwrap();
    let descriptor = runtime::get_collection_runtime_descriptor(&pool, game_id)
        .await
        .unwrap();
    assert_eq!(descriptor.counts.active_mod_count, 1);
    assert_eq!(current.current_mods.len(), 1);
    assert_eq!(
        current.current_mods[0].display_name.as_deref(),
        Some("Green")
    );
    assert_eq!(current.projected_state.summary.active_root_count, 1);
    assert_eq!(current.current_tree_nodes.len(), 1);
    assert!(current.current_tree_nodes[0]
        .children
        .iter()
        .all(|node| node.is_effectively_active));
    assert!(current
        .current_objects
        .iter()
        .any(|object| object.path_key.as_deref() == Some("DISABLED Alice") && !object.is_enabled));

    // This application capture runs only after the real projection is coherent;
    // the AppHandle snapshot guard and native runtime publication are outside this fixture.
    let captured = collection::create_collection(
        &pool,
        CreateCollectionInput {
            game_id: game_id.to_owned(),
            name: "Disabled ancestor state".to_owned(),
            save_mode: None,
            source_collection_id: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(captured.mod_count, 1);
    assert_eq!(
        captured.signature.as_deref(),
        Some(current.current_signature.as_str())
    );
    let preview =
        collection::get_collection_preview(&pool, game_id, &captured.id, Some(&root_text))
            .await
            .unwrap();
    assert_eq!(preview.projected_state.active_roots.len(), 1);
    assert_eq!(
        preview.projected_state.active_roots[0].display_name,
        "Green"
    );
    assert!(preview
        .tree_nodes
        .iter()
        .any(|object| !object.is_enabled && object.children.is_empty()));
    let clean = runtime::get_collection_runtime_state(&pool, game_id)
        .await
        .unwrap();
    assert!(!clean.is_dirty);
    let initial_preview =
        collection::preview_apply(&pool, game_id, &captured.id, Some(&root_text), false)
            .await
            .unwrap();
    assert_eq!(
        initial_preview
            .current_projected_state
            .summary
            .active_root_count,
        1
    );
    assert_eq!(
        initial_preview
            .target_projected_state
            .summary
            .active_root_count,
        1
    );

    fs::rename(&disabled, &old).unwrap();
    run_reconcile(
        &pool,
        game_id,
        &root,
        DiskReconcileReason::WatcherBatch,
        &paths,
        false,
    )
    .await;
    let external = runtime::get_collection_runtime_state(&pool, game_id)
        .await
        .unwrap();
    assert!(external.is_dirty);
    assert_eq!(external.current_mods.len(), 2);
    assert!(external
        .current_mods
        .iter()
        .all(|member| member.mod_id.as_ref() != Some(&nested.id)));
    let preview = collection::preview_apply(&pool, game_id, &captured.id, Some(&root_text), false)
        .await
        .unwrap();
    assert_eq!(preview.current_projected_state.summary.active_root_count, 2);
    assert_eq!(preview.target_projected_state.summary.active_root_count, 1);
    assert_eq!(
        persisted_mod_snapshot(&pool, game_id, "Blue").await.id,
        blue.id
    );
    assert_eq!(
        persisted_mod_snapshot(&pool, game_id, "Nested").await.id,
        nested.id
    );
}

#[tokio::test]
async fn projection_write_transaction_waits_for_an_existing_writer() {
    let temp = tempfile::tempdir().expect("tempdir");
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(temp.path().join("projection-lock.db"))
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .busy_timeout(Duration::from_millis(500)),
        )
        .await
        .expect("pool should connect");

    let blocker = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .expect("writer lock should begin");
    let contending_pool = pool.clone();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let mut contender = tokio::spawn(async move {
        let _ = started_tx.send(());
        begin_projection_write_transaction(&contending_pool)
            .await
            .map(drop)
    });
    started_rx.await.expect("contender should start");

    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut contender)
            .await
            .is_err(),
        "a projection transaction must wait for an existing SQLite writer"
    );

    blocker.commit().await.expect("writer lock should release");
    contender
        .await
        .expect("contender task should finish")
        .expect("projection transaction should start after the writer releases");
}

#[tokio::test]
async fn full_reconcile_marks_every_scanned_root_for_thumbnail_refresh() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    create_terminal_mod(&mods_path.join("Bob").join("Red"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_full_thumbnail_refresh",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");

    let outcome = run_reconcile(
        &pool,
        "g_full_thumbnail_refresh",
        &mods_path,
        DiskReconcileReason::StartupBoot,
        &[],
        true,
    )
    .await;

    assert_eq!(outcome.thumbnail_roots, vec!["Alice", "Bob"]);
}

#[tokio::test]
async fn projection_refresh_failure_rolls_back_core_reconcile_rows() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_atomic_projection",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");

    sqlx::query(
        "CREATE TRIGGER fail_runtime_projection_insert
         BEFORE INSERT ON object_runtime_projection
         BEGIN
             SELECT RAISE(FAIL, 'injected runtime projection failure');
         END",
    )
    .execute(&pool)
    .await
    .expect("failure trigger should install");

    let result = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id: "g_atomic_projection",
        mods_path: &mods_path,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::ManualRepair,
        changed_paths: &[],
        force_full: true,
        watcher_events: None,
        path_hints: &[],
        trusted_mutation_scope: false,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await;

    let error = result.expect_err("projection failure must fail reconcile");
    assert!(
        error
            .to_string()
            .contains("injected runtime projection failure"),
        "{error}"
    );
    let object_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = 'g_atomic_projection'")
            .fetch_one(&pool)
            .await
            .expect("object count should load");
    let mod_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = 'g_atomic_projection'")
            .fetch_one(&pool)
            .await
            .expect("mod count should load");
    assert_eq!(
        (object_count, mod_count),
        (0, 0),
        "core rows must not commit without their runtime projection"
    );
}

#[tokio::test]
async fn scoped_reconcile_refreshes_touched_projection_and_keeps_the_rest() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    create_terminal_mod(&mods_path.join("Bob").join("Red"));

    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_scoped",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");

    run_reconcile(
        &pool,
        "g_scoped",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    assert_eq!(projection_row(&pool, "g_scoped", "Alice").await, Some(0));
    assert_eq!(projection_row(&pool, "g_scoped", "Bob").await, Some(0));

    // Toggle Alice on disk, then reconcile scoped to that root only.
    fs::rename(mods_path.join("Alice"), mods_path.join("DISABLED Alice"))
        .expect("rename should succeed");
    let toggled = run_reconcile(
        &pool,
        "g_scoped",
        &mods_path,
        DiskReconcileReason::InternalMutation,
        &[
            mods_path.join("Alice").to_string_lossy().to_string(),
            mods_path
                .join("DISABLED Alice")
                .to_string_lossy()
                .to_string(),
        ],
        false,
    )
    .await;

    assert_eq!(toggled.status, DiskReconcileStatus::Applied);
    assert!(
        toggled.folder_conflicts.is_empty(),
        "a single disabled root must not be reported as a folder conflict"
    );

    assert_eq!(
        projection_row(&pool, "g_scoped", "DISABLED Alice").await,
        Some(1),
        "toggled object's projection should be refreshed"
    );
    assert_eq!(
        projection_row(&pool, "g_scoped", "Bob").await,
        Some(0),
        "untouched object's projection must survive a scoped refresh"
    );
    assert_eq!(
        mod_row(&pool, "g_scoped").await,
        Some((
            Path::new("DISABLED Alice")
                .join("Blue")
                .to_string_lossy()
                .to_string(),
            1,
        )),
        "a parent prefix changes the mod path while its own status remains enabled"
    );

    // A watcher-driven rename in the other direction must converge to the
    // same enabled path instead of replaying a stale internal-mutation state.
    fs::rename(mods_path.join("DISABLED Alice"), mods_path.join("Alice"))
        .expect("rename should succeed");
    run_reconcile(
        &pool,
        "g_scoped",
        &mods_path,
        DiskReconcileReason::WatcherBatch,
        &[
            mods_path
                .join("DISABLED Alice")
                .to_string_lossy()
                .to_string(),
            mods_path.join("Alice").to_string_lossy().to_string(),
        ],
        false,
    )
    .await;
    assert_eq!(
        mod_row(&pool, "g_scoped").await,
        Some((
            Path::new("Alice")
                .join("Blue")
                .to_string_lossy()
                .to_string(),
            1,
        )),
        "watcher reconcile must converge back to the enabled disk path/status"
    );

    // Exercise the prefix on the mod itself as used by enable/disable actions.
    fs::rename(
        mods_path.join("Alice").join("Blue"),
        mods_path.join("Alice").join("DISABLED Blue"),
    )
    .expect("rename should succeed");
    run_reconcile(
        &pool,
        "g_scoped",
        &mods_path,
        DiskReconcileReason::InternalMutation,
        &[
            mods_path
                .join("Alice")
                .join("Blue")
                .to_string_lossy()
                .to_string(),
            mods_path
                .join("Alice")
                .join("DISABLED Blue")
                .to_string_lossy()
                .to_string(),
        ],
        false,
    )
    .await;
    assert_eq!(
        mod_row(&pool, "g_scoped").await,
        Some((
            Path::new("Alice")
                .join("DISABLED Blue")
                .to_string_lossy()
                .to_string(),
            0,
        )),
        "disable action reconcile must align the DB path/status with disk"
    );

    fs::rename(
        mods_path.join("Alice").join("DISABLED Blue"),
        mods_path.join("Alice").join("Blue"),
    )
    .expect("rename should succeed");
    run_reconcile(
        &pool,
        "g_scoped",
        &mods_path,
        DiskReconcileReason::WatcherBatch,
        &[
            mods_path
                .join("Alice")
                .join("DISABLED Blue")
                .to_string_lossy()
                .to_string(),
            mods_path
                .join("Alice")
                .join("Blue")
                .to_string_lossy()
                .to_string(),
        ],
        false,
    )
    .await;
    assert_eq!(
        mod_row(&pool, "g_scoped").await,
        Some((
            Path::new("Alice")
                .join("Blue")
                .to_string_lossy()
                .to_string(),
            1,
        )),
        "watcher reconcile after enable must align the DB path/status with disk"
    );

    // Delete Bob's folder; a scoped reconcile must drop its projection row.
    fs::remove_dir_all(mods_path.join("Bob")).expect("remove should succeed");
    run_reconcile(
        &pool,
        "g_scoped",
        &mods_path,
        DiskReconcileReason::InternalMutation,
        &[mods_path.join("Bob").to_string_lossy().to_string()],
        false,
    )
    .await;

    assert_eq!(
        projection_row(&pool, "g_scoped", "Bob").await,
        None,
        "deleted object's projection row must be pruned by a scoped refresh"
    );
}

#[tokio::test]
async fn watcher_single_root_reconcile_does_not_classify_an_unrelated_root() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_watcher_scoped_scan",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    run_reconcile(
        &pool,
        "g_watcher_scoped_scan",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;

    create_terminal_mod(&mods_path.join("Bob").join("Broken"));
    fs::write(
        mods_path.join("Bob").join("Broken").join("mod.ini"),
        [0xFF, 0xFE, 0xFD],
    )
    .expect("invalid unrelated ini should be written");
    create_terminal_mod(&mods_path.join("Alice").join("Green"));

    let outcome = run_reconcile(
        &pool,
        "g_watcher_scoped_scan",
        &mods_path,
        DiskReconcileReason::WatcherBatch,
        &[mods_path
            .join("Alice")
            .join("Green")
            .to_string_lossy()
            .to_string()],
        false,
    )
    .await;

    assert_eq!(outcome.status, DiskReconcileStatus::Applied);
    assert!(
        mod_row_named(&pool, "g_watcher_scoped_scan", "Green")
            .await
            .is_some(),
        "the changed root should still converge"
    );
}

#[tokio::test]
async fn internal_single_root_reconcile_does_not_classify_an_unrelated_root() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_internal_scoped_scan",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    run_reconcile(
        &pool,
        "g_internal_scoped_scan",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;

    create_terminal_mod(&mods_path.join("Bob").join("Broken"));
    fs::write(
        mods_path.join("Bob").join("Broken").join("mod.ini"),
        [0xFF, 0xFE, 0xFD],
    )
    .expect("invalid unrelated ini should be written");
    create_terminal_mod(&mods_path.join("Alice").join("Green"));

    let outcome = run_reconcile(
        &pool,
        "g_internal_scoped_scan",
        &mods_path,
        DiskReconcileReason::InternalMutation,
        &[mods_path
            .join("Alice")
            .join("Green")
            .to_string_lossy()
            .to_string()],
        false,
    )
    .await;

    assert_eq!(outcome.status, DiskReconcileStatus::Applied);
    assert!(
        mod_row_named(&pool, "g_internal_scoped_scan", "Green")
            .await
            .is_some(),
        "the changed root should still converge"
    );
}

#[tokio::test]
async fn thumbnail_only_watcher_batch_skips_projection_and_unrelated_classification() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_thumbnail_fast_path",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    run_reconcile(
        &pool,
        "g_thumbnail_fast_path",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;

    create_terminal_mod(&mods_path.join("Bob").join("Broken"));
    fs::write(
        mods_path.join("Bob").join("Broken").join("mod.ini"),
        [0xFF, 0xFE, 0xFD],
    )
    .expect("invalid unrelated ini should be written");
    let thumbnail = mods_path.join("Alice").join("Blue").join("thumb.png");
    fs::write(&thumbnail, "image").expect("thumbnail should be written");

    let outcome = run_reconcile(
        &pool,
        "g_thumbnail_fast_path",
        &mods_path,
        DiskReconcileReason::WatcherBatch,
        &[thumbnail.to_string_lossy().to_string()],
        false,
    )
    .await;

    assert_eq!(outcome.status, DiskReconcileStatus::Applied);
    assert_eq!(
        outcome.scan_scope,
        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileScanScope::None
    );
    assert_eq!(outcome.thumbnail_roots, vec!["Alice"]);
    assert!(!outcome.objects_changed);
    assert!(!outcome.folders_changed);
    assert_eq!(
        mod_row_named(&pool, "g_thumbnail_fast_path", "Blue").await,
        Some(
            mod_row_named(&pool, "g_thumbnail_fast_path", "Blue")
                .await
                .expect("existing projected mod")
        ),
        "thumbnail invalidation must not rewrite the disk projection"
    );
}

#[tokio::test]
async fn watcher_event_for_mods_root_forces_full_source_validation() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_mods_root_event",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    run_reconcile(
        &pool,
        "g_mods_root_event",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    create_terminal_mod(&mods_path.join("Bob").join("Broken"));
    fs::write(
        mods_path.join("Bob").join("Broken").join("mod.ini"),
        [0xFF, 0xFE, 0xFD],
    )
    .expect("invalid ini should be written");

    let error = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id: "g_mods_root_event",
        mods_path: &mods_path,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::WatcherBatch,
        changed_paths: &[mods_path.to_string_lossy().to_string()],
        force_full: false,
        watcher_events: None,
        path_hints: &[],
        trusted_mutation_scope: false,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .expect_err("a Mods-root watcher event must validate every root");

    assert!(error.to_string().contains("Unsupported INI encoding"));
}

#[tokio::test]
async fn brand_new_conflicted_identity_is_overlay_only_and_excluded_from_runtime_state() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Stable"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_conflict",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");

    run_reconcile(
        &pool,
        "g_conflict",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    let signature_before = runtime_signature(&pool, "g_conflict").await;

    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    create_terminal_mod(&mods_path.join("Alice").join("DISABLED Blue"));
    let outcome = run_reconcile(
        &pool,
        "g_conflict",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;

    assert_eq!(
        outcome.status,
        DiskReconcileStatus::AppliedWithFolderConflicts
    );
    assert_eq!(outcome.folder_conflicts.len(), 1);
    assert_eq!(outcome.folder_conflicts[0].candidates.len(), 2);
    assert_eq!(outcome.folder_conflicts[0].identity, "alice/blue");
    assert_eq!(
        outcome.folder_conflicts[0]
            .candidates
            .iter()
            .map(|candidate| candidate.folder_name.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["Blue", "DISABLED Blue"]),
        "the ambiguity overlay must retain every physical candidate"
    );
    assert!(outcome.error_message.is_none());

    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind("g_conflict")
        .fetch_one(&pool)
        .await
        .expect("object count should load");
    let mod_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = ?")
        .bind("g_conflict")
        .fetch_one(&pool)
        .await
        .expect("mod count should load");
    assert_eq!((object_count, mod_count), (1, 1));
    assert!(
        mod_row_named(&pool, "g_conflict", "Blue").await.is_none(),
        "a new ambiguous identity must not gain a writable DB representative"
    );
    assert_eq!(
        runtime_signature(&pool, "g_conflict").await,
        signature_before,
        "an unresolved identity with no prior row must not alter runtime signature"
    );
}

#[tokio::test]
async fn existing_enabled_conflict_state_is_preserved_while_unrelated_roots_converge() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_existing_conflict",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");

    run_reconcile(
        &pool,
        "g_existing_conflict",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    let row_before = persisted_mod_snapshot(&pool, "g_existing_conflict", "Blue").await;
    let signature_before = runtime_signature(&pool, "g_existing_conflict").await;

    create_terminal_mod(&mods_path.join("Alice").join("DISABLED Blue"));
    let conflict_outcome = run_reconcile(
        &pool,
        "g_existing_conflict",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;

    assert_eq!(
        conflict_outcome.status,
        DiskReconcileStatus::AppliedWithFolderConflicts
    );
    assert_eq!(
        persisted_mod_snapshot(&pool, "g_existing_conflict", "Blue").await,
        row_before,
        "the complete persisted row must remain byte-for-byte stable while ambiguous"
    );
    assert_eq!(
        runtime_signature(&pool, "g_existing_conflict").await,
        signature_before,
        "the protected enabled state must retain its runtime signature"
    );

    create_terminal_mod(&mods_path.join("Bob").join("Red"));
    let unrelated_outcome = run_reconcile(
        &pool,
        "g_existing_conflict",
        &mods_path,
        DiskReconcileReason::WatcherBatch,
        &[mods_path.join("Bob").to_string_lossy().to_string()],
        false,
    )
    .await;

    assert_eq!(
        unrelated_outcome.status,
        DiskReconcileStatus::AppliedWithFolderConflicts
    );
    assert!(
        mod_row_named(&pool, "g_existing_conflict", "Red")
            .await
            .is_some(),
        "an unrelated root must converge in the same conflicted reconcile"
    );
    assert_eq!(
        persisted_mod_snapshot(&pool, "g_existing_conflict", "Blue").await,
        row_before,
        "unrelated convergence must not rewrite protected state"
    );
}

#[tokio::test]
async fn folder_conflict_quarantines_only_its_paths_while_unrelated_mods_converge() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    create_terminal_mod(&mods_path.join("Alice").join("DISABLED Blue"));
    create_terminal_mod(&mods_path.join("Bob").join("Red"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_conflict_scope",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");

    let outcome = run_reconcile(
        &pool,
        "g_conflict_scope",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;

    assert_eq!(
        outcome.status,
        DiskReconcileStatus::AppliedWithFolderConflicts
    );
    assert_eq!(outcome.folder_conflicts.len(), 1);
    assert!(
        mod_row_named(&pool, "g_conflict_scope", "Red")
            .await
            .is_some(),
        "unrelated disk paths must project even while Alice is quarantined"
    );
    assert!(
        mod_row_named(&pool, "g_conflict_scope", "Blue")
            .await
            .is_none(),
        "the conflict overlay, not a writable representative row, owns ambiguity"
    );
}

#[tokio::test]
async fn scoped_conflict_preflight_ignores_duplicate_names_under_distinct_roots() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    create_terminal_mod(&mods_path.join("Alice").join("DISABLED Blue"));
    create_terminal_mod(&mods_path.join("Charlie").join("Blue"));
    create_terminal_mod(&mods_path.join("Bob").join("Red"));
    create_terminal_mod(&mods_path.join("Bob").join("DISABLED Red"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_conflict_queue",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");

    let outcome = run_reconcile(
        &pool,
        "g_conflict_queue",
        &mods_path,
        DiskReconcileReason::InternalMutation,
        &[mods_path.join("Alice").to_string_lossy().to_string()],
        false,
    )
    .await;

    assert_eq!(
        outcome.status,
        DiskReconcileStatus::AppliedWithFolderConflicts
    );
    assert_eq!(outcome.folder_conflicts.len(), 2);
    assert!(outcome.folder_conflicts.iter().all(|group| {
        group
            .candidates
            .iter()
            .all(|candidate| !Path::new(&candidate.path).starts_with(mods_path.join("Charlie")))
    }));
    assert_eq!(
        outcome
            .folder_conflicts
            .iter()
            .map(|group| group.candidates.len())
            .max(),
        Some(2)
    );
    let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_count, 0);
}

#[tokio::test]
async fn scoped_single_root_event_keeps_existing_conflict_scope_snapshot() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));

    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_single_root_conflict",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    run_reconcile(
        &pool,
        "g_single_root_conflict",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    let before = mod_row(&pool, "g_single_root_conflict").await;

    create_terminal_mod(&mods_path.join("DISABLED Alice").join("Blue"));
    let outcome = run_reconcile(
        &pool,
        "g_single_root_conflict",
        &mods_path,
        DiskReconcileReason::WatcherBatch,
        &[mods_path
            .join("DISABLED Alice")
            .to_string_lossy()
            .to_string()],
        false,
    )
    .await;

    assert_eq!(
        outcome.status,
        DiskReconcileStatus::AppliedWithFolderConflicts
    );
    assert_eq!(outcome.scan_scope, DiskReconcileScanScope::Full);
    assert_eq!(mod_row(&pool, "g_single_root_conflict").await, before);
    let mod_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = ?")
        .bind("g_single_root_conflict")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        mod_count, 1,
        "blocked scoped reconcile must not drift DB state"
    );
}

#[tokio::test]
async fn trusted_toggle_reconcile_tracks_owned_row_through_a_distinct_prefix_conflict() {
    assert_trusted_toggle_projects_owned_row(false, false).await;
}

#[tokio::test]
async fn trusted_toggle_reconcile_tracks_owned_row_when_old_path_is_reoccupied_before_projection() {
    assert_trusted_toggle_projects_owned_row(true, false).await;
}

#[cfg(windows)]
#[tokio::test]
async fn trusted_toggle_projection_accepts_canonical_journal_with_regular_configured_root() {
    assert_trusted_toggle_projects_owned_row(false, true).await;
}

#[cfg(windows)]
#[test]
fn trusted_toggle_proof_preserves_verbatim_only_folder_names() {
    assert_special_folder_projection_proof("A. ", true);
}

#[cfg(windows)]
#[tokio::test]
async fn trusted_toggle_reconcile_preserves_verbatim_only_ancestor_name() {
    use crate::modules::library::application::mods::core_ops::plan_toggle_rename;
    use crate::modules::reconciliation::application::toggle_projection::{
        proven_projection_events, validate_projected_toggle_rows,
    };
    let db = init_test_db().await;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    std::fs::create_dir(&root).unwrap();
    let canonical_root = std::fs::canonicalize(&root).unwrap();
    let source = canonical_root.join("Alice. ").join("A");
    create_terminal_mod(&source);
    let game_id = "g_verbatim_ancestor";
    insert_test_game(
        &db.pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(root.to_string_lossy().as_ref()),
        },
    )
    .await
    .unwrap();
    let initial = run_reconcile(
        &db.pool,
        game_id,
        &canonical_root,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    assert!(initial.status.applied());
    let original = persisted_mod_snapshot(&db.pool, game_id, "A").await;
    let plan = plan_toggle_rename(&source, false).unwrap().unwrap();
    plan.apply("mod").unwrap();
    let pending = projection_fixture_operation(&plan);
    let events = proven_projection_events(std::slice::from_ref(&pending), &root).unwrap();
    let changes = [
        plan.old_path().to_string_lossy().into_owned(),
        plan.new_path().to_string_lossy().into_owned(),
    ];
    let outcome = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &db.pool,
        game_id,
        mods_path: &root,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::InternalMutation,
        changed_paths: &changes,
        force_full: false,
        watcher_events: Some(&events),
        path_hints: &[],
        trusted_mutation_scope: true,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .unwrap();
    assert!(outcome.status.applied());
    let current = persisted_mod_snapshot(&db.pool, game_id, "A").await;
    assert_eq!(current.id, original.id);
    assert_eq!(current.filesystem_identity, original.filesystem_identity);
    assert_eq!(
        Path::new(&current.folder_path),
        Path::new("Alice. ").join("DISABLED A")
    );
    validate_projected_toggle_rows(&db.pool, game_id, &[pending], &root)
        .await
        .unwrap();
    assert!(plan.new_path().join("mod.ini").is_file());
}

#[cfg(not(windows))]
#[test]
fn trusted_toggle_proof_preserves_native_backslash_folder_names() {
    assert_special_folder_projection_proof(r"A\Name", false);
}

fn assert_special_folder_projection_proof(name: &str, canonical: bool) {
    use crate::modules::library::application::mods::core_ops::plan_toggle_rename;
    use crate::modules::reconciliation::application::toggle_projection::proven_projection_events;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    std::fs::create_dir(&root).unwrap();
    let physical_root = if canonical {
        std::fs::canonicalize(&root).unwrap()
    } else {
        root.clone()
    };
    let source = physical_root.join(name);
    create_terminal_mod(&source);
    let plan = plan_toggle_rename(&source, false).unwrap().unwrap();
    plan.apply("mod").unwrap();
    let pending = projection_fixture_operation(&plan);
    let events = proven_projection_events(&[pending], &root).unwrap();
    assert_eq!(events.len(), 1);
    assert!(plan.new_path().join("mod.ini").is_file());
}

#[tokio::test]
async fn trusted_toggle_reconcile_never_borrows_a_distinct_normalized_sibling_row() {
    use crate::modules::games::domain::models::ItemStatus;
    use crate::modules::library::application::mods::core_ops::plan_toggle_rename;
    use crate::platform::fs::file_utils::FilesystemIdentityProof;

    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().unwrap();
    let mods_path = temp.path().join("Mods");
    let indexed_path = mods_path.join("Alice").join("DISABLED DISABLED A");
    create_terminal_mod(&indexed_path);
    let indexed_proof = FilesystemIdentityProof::capture(&indexed_path).unwrap();
    let mods_path_string = mods_path.to_string_lossy().to_string();
    let game_id = "g_trusted_toggle_foreign_row";
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .unwrap();
    run_reconcile(
        &pool,
        game_id,
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    let original_row = persisted_mod_snapshot(&pool, game_id, "A").await;
    assert_eq!(original_row.status, ItemStatus::Disabled as i64);
    assert_eq!(
        original_row.filesystem_identity.as_deref(),
        Some(indexed_proof.identity())
    );

    let toggled_path = mods_path.join("Alice").join("DISABLED A");
    create_terminal_mod(&toggled_path);
    let toggled_proof = FilesystemIdentityProof::capture(&toggled_path).unwrap();
    assert_ne!(toggled_proof.identity(), indexed_proof.identity());
    let plan = plan_toggle_rename(&toggled_path, true).unwrap().unwrap();
    plan.apply("mod").unwrap();
    toggled_proof.validate(plan.new_path()).unwrap();
    indexed_proof.validate(&indexed_path).unwrap();
    let changed_paths = [
        plan.old_path().to_string_lossy().to_string(),
        plan.new_path().to_string_lossy().to_string(),
    ];
    let pending = projection_fixture_operation(&plan);
    let events =
        crate::modules::reconciliation::application::toggle_projection::proven_projection_events(
            std::slice::from_ref(&pending),
            &mods_path,
        )
        .unwrap();
    let outcome = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id,
        mods_path: &mods_path,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::InternalMutation,
        changed_paths: &changed_paths,
        force_full: false,
        watcher_events: Some(&events),
        path_hints: &[],
        trusted_mutation_scope: true,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .unwrap();
    assert_eq!(
        outcome.status,
        DiskReconcileStatus::AppliedWithFolderConflicts
    );
    let row = persisted_mod_snapshot(&pool, game_id, "A").await;
    assert_eq!(row.id, original_row.id);
    assert_eq!(
        row.folder_path, original_row.folder_path,
        "a normalized-key match must not borrow a distinct sibling's indexed row"
    );
    assert_eq!(row.status, ItemStatus::Disabled as i64);
    assert_eq!(row.filesystem_identity, original_row.filesystem_identity);
    indexed_proof.validate(&indexed_path).unwrap();
    toggled_proof.validate(plan.new_path()).unwrap();
    assert!(crate::modules::reconciliation::application::toggle_projection::validate_projected_toggle_rows(&pool, game_id, &[pending], &mods_path).await.is_err(), "an unindexed ambiguous owned mod must not advance a projection checkpoint");
}

fn projection_fixture_operation(
    plan: &crate::modules::library::application::mods::core_ops::ToggleRenamePlan,
) -> crate::modules::mutation::api::Operation {
    use crate::modules::mutation::journal::{
        DatabaseProjectionStatus, MutationStepKind, Operation, OperationStatus, OperationStep,
        StepStatus,
    };
    Operation {
        id: "proven-native-toggle".to_string(),
        kind: "workspace-switch".to_string(),
        game_id: "fixture".to_string(),
        source_epoch: None,
        created_at: String::new(),
        status: OperationStatus::DiskCommitted,
        steps: vec![OperationStep {
            sequence: 0,
            kind: MutationStepKind::Rename,
            old_path: Some(plan.old_path().to_path_buf()),
            new_path: Some(plan.new_path().to_path_buf()),
            stage_path: None,
            expected_identity: Some(plan.expected_identity().to_string()),
            status: StepStatus::Applied,
        }],
        database_projection_status: DatabaseProjectionStatus::NotStarted,
        last_error: None,
        disk_revision: Some(1),
    }
}

#[tokio::test]
async fn trusted_parent_toggle_projects_its_own_row_and_known_descendants() {
    assert_parent_toggle_descendants(true, false).await;
}

#[tokio::test]
async fn rowless_parent_toggle_cannot_acknowledge_an_unproven_known_descendant() {
    assert_parent_toggle_descendants(false, true).await;
}

#[tokio::test]
async fn trusted_parent_toggle_cannot_acknowledge_a_replaced_descendant() {
    assert_parent_toggle_descendants(true, true).await;
}

async fn assert_parent_toggle_descendants(parent_has_ini: bool, invalidate_child: bool) {
    use crate::modules::library::application::mods::core_ops::plan_toggle_rename;
    use crate::modules::reconciliation::application::toggle_projection::{
        proven_projection_events, validate_projected_toggle_rows,
    };
    use crate::platform::fs::file_utils::FilesystemIdentityProof;

    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let old = root.join("Alice").join("DISABLED Pack");
    if parent_has_ini {
        create_terminal_mod(&old);
    }
    create_terminal_mod(&old.join("Child"));
    let root_text = root.to_string_lossy().to_string();
    let game_id = "g_parent_toggle_descendants";
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&root_text),
        },
    )
    .await
    .unwrap();
    run_reconcile(
        &pool,
        game_id,
        &root,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    if parent_has_ini {
        use crate::modules::games::domain::models::ItemStatus;
        use crate::test_utils::{insert_test_mod, TestModFixture};
        let object_id: String =
            sqlx::query_scalar("SELECT object_id FROM mods WHERE game_id = ? LIMIT 1")
                .bind(game_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let child_id =
            crate::modules::system::adapters::sqlite::utils::stable_ids::generate_stable_id(
                game_id,
                "Alice/DISABLED Pack/Child",
            );
        insert_test_mod(
            &pool,
            &TestModFixture {
                id: &child_id,
                game_id,
                object_id: Some(&object_id),
                actual_name: "Child",
                folder_path: "Alice/DISABLED Pack/Child",
                status: ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Other"),
                mods_path: Some(&root_text),
            },
        )
        .await
        .unwrap();
        sqlx::query("UPDATE mods SET filesystem_identity = ? WHERE id = ?")
            .bind(crate::platform::fs::file_utils::filesystem_identity(
                &old.join("Child"),
            ))
            .bind(&child_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    let child = persisted_mod_snapshot(&pool, game_id, "Child").await;
    let child_proof = FilesystemIdentityProof::capture(&old.join("Child")).unwrap();
    if !parent_has_ini && invalidate_child {
        sqlx::query("UPDATE mods SET filesystem_identity = NULL WHERE id = ?")
            .bind(&child.id)
            .execute(&pool)
            .await
            .unwrap();
    }
    let plan = plan_toggle_rename(&old, true).unwrap().unwrap();
    plan.apply("mod").unwrap();
    create_terminal_mod(&old.join("Child"));
    create_terminal_mod(
        &root
            .join("Alice")
            .join("DISABLED DISABLED Pack")
            .join("Child"),
    );
    let foreign_proof = FilesystemIdentityProof::capture(&old).unwrap();
    if parent_has_ini && invalidate_child {
        fs::rename(
            plan.new_path().join("Child"),
            temp.path().join("original-child"),
        )
        .unwrap();
        create_terminal_mod(&plan.new_path().join("Child"));
    }
    let pending = projection_fixture_operation(&plan);
    let events = proven_projection_events(std::slice::from_ref(&pending), &root).unwrap();
    let paths = [
        old.to_string_lossy().to_string(),
        plan.new_path().to_string_lossy().to_string(),
    ];
    let outcome = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id,
        mods_path: &root,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::InternalMutation,
        changed_paths: &paths,
        force_full: false,
        watcher_events: Some(&events),
        path_hints: &[],
        trusted_mutation_scope: true,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .unwrap();
    assert_eq!(
        outcome.status,
        DiskReconcileStatus::AppliedWithFolderConflicts
    );
    let validation = validate_projected_toggle_rows(&pool, game_id, &[pending], &root).await;
    if invalidate_child {
        assert!(
            validation.is_err(),
            "a parent participant cannot acknowledge an unproven or replaced child"
        );
        let persisted = persisted_mod_snapshot(&pool, game_id, "Child").await;
        assert_eq!(persisted.id, child.id);
    } else {
        validation.unwrap();
        let persisted = persisted_mod_snapshot(&pool, game_id, "Child").await;
        assert_eq!(persisted.id, child.id);
        assert_eq!(
            Path::new(&persisted.folder_path),
            plan.new_path().join("Child").strip_prefix(&root).unwrap()
        );
        assert_eq!(
            persisted.filesystem_identity.as_deref(),
            Some(child_proof.identity())
        );
        child_proof
            .validate(&plan.new_path().join("Child"))
            .unwrap();
        let parent = persisted_mod_snapshot(&pool, game_id, "Pack").await;
        assert_eq!(
            Path::new(&parent.folder_path),
            plan.new_path().strip_prefix(&root).unwrap()
        );
    }
    foreign_proof.validate(&old).unwrap();
}

#[tokio::test]
async fn repaired_projection_uses_a_completed_same_physical_successor_and_latest_revision() {
    use crate::modules::library::application::mods::core_ops::plan_toggle_rename;
    use crate::modules::mutation::api::MutationCoordinator;
    use crate::modules::mutation::journal::{OperationJournal, OperationPlan, PlannedStep};
    use crate::modules::reconciliation::application::toggle_projection::settle_repaired_projection_operations;
    use crate::platform::fs::file_utils::FilesystemIdentityProof;
    use crate::platform::fs::operation_lock::OperationLock;

    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Mods");
    let disabled = root.join("Alice").join("DISABLED A");
    create_terminal_mod(&disabled);
    let root_text = root.to_string_lossy().to_string();
    let game_id = "g_completed_successor_repair";
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&root_text),
        },
    )
    .await
    .unwrap();
    run_reconcile(
        &pool,
        game_id,
        &root,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    let original = persisted_mod_snapshot(&pool, game_id, "A").await;
    let epoch = FilesystemIdentityProof::capture(&root)
        .unwrap()
        .identity()
        .to_string();
    let journal = std::sync::Arc::new(
        OperationJournal::open(temp.path().join("successor-journal.json"), 50).unwrap(),
    );
    let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
    let record = |plan: &crate::modules::library::application::mods::core_ops::ToggleRenamePlan| {
        let id = journal
            .plan_operation(
                OperationPlan::new(
                    "workspace-switch",
                    game_id,
                    vec![PlannedStep::rename(
                        0,
                        plan.old_path().to_path_buf(),
                        plan.new_path().to_path_buf(),
                    )
                    .with_expected_identity(Some(plan.expected_identity().to_string()))],
                )
                .with_source_epoch(epoch.clone()),
            )
            .unwrap();
        journal.mark_applying(&id).unwrap();
        journal.mark_step_applied(&id, 0).unwrap();
        journal.mark_disk_committed(&id).unwrap();
        id
    };
    let first = plan_toggle_rename(&disabled, true).unwrap().unwrap();
    first.apply("mod").unwrap();
    let first_id = record(&first);
    coordinator
        .isolate_projection_for_repair(&first_id, "stale owned row")
        .unwrap();
    assert!(settle_repaired_projection_operations(
        &pool,
        &coordinator,
        game_id,
        &epoch,
        &root,
        &[]
    )
    .await
    .is_err());

    let successor = plan_toggle_rename(first.new_path(), false)
        .unwrap()
        .unwrap();
    successor.apply("mod").unwrap();
    let successor_id = record(&successor);
    coordinator
        .complete_disk_projection(&[successor_id])
        .unwrap();
    let latest = coordinator
        .latest_toggle_disk_revision_in_epoch(game_id, Some(&epoch))
        .unwrap();
    assert!(
        latest
            > journal
                .entries()
                .iter()
                .find(|operation| operation.id == first_id)
                .unwrap()
                .disk_revision
                .unwrap()
    );
    assert!(coordinator
        .earliest_toggle_projection_repair(game_id, &epoch)
        .unwrap()
        .is_some());
    settle_repaired_projection_operations(&pool, &coordinator, game_id, &epoch, &root, &[])
        .await
        .unwrap();
    assert!(coordinator
        .earliest_toggle_projection_repair(game_id, &epoch)
        .unwrap()
        .is_none());
    assert_eq!(
        coordinator
            .latest_toggle_disk_revision_in_epoch(game_id, Some(&epoch))
            .unwrap(),
        latest,
        "after clearing the hole, the barrier must include the already completed successor"
    );
    let current = persisted_mod_snapshot(&pool, game_id, "A").await;
    assert_eq!(current.id, original.id);
    assert_eq!(current.folder_path, original.folder_path);
    assert_eq!(current.filesystem_identity, original.filesystem_identity);
}

async fn assert_trusted_toggle_projects_owned_row(
    reoccupy_old_path: bool,
    canonical_journal: bool,
) {
    use crate::modules::games::domain::models::ItemStatus;
    use crate::modules::library::application::mods::core_ops::plan_toggle_rename;
    use crate::platform::fs::file_utils::FilesystemIdentityProof;

    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().unwrap();
    let mods_path = temp.path().join("Mods");
    let disabled_path = mods_path.join("Alice").join("DISABLED A");
    create_terminal_mod(&disabled_path);
    let mods_path_string = mods_path.to_string_lossy().to_string();
    let game_id = "g_trusted_toggle_conflict";
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .unwrap();
    let indexed = run_reconcile(
        &pool,
        game_id,
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    assert_eq!(indexed.status, DiskReconcileStatus::Applied);
    let original_row = persisted_mod_snapshot(&pool, game_id, "A").await;
    assert_eq!(original_row.status, ItemStatus::Disabled as i64);
    let owned_proof = FilesystemIdentityProof::capture(&disabled_path).unwrap();

    let foreign_path = mods_path.join("Alice").join("DISABLED DISABLED A");
    create_terminal_mod(&foreign_path);
    let foreign_proof = FilesystemIdentityProof::capture(&foreign_path).unwrap();
    assert_ne!(owned_proof.identity(), foreign_proof.identity());

    let mut current_path = disabled_path;
    for enable in [true, false, true, false] {
        let plan = plan_toggle_rename(&current_path, enable).unwrap().unwrap();
        plan.apply("mod").unwrap();
        let changed_paths = [
            plan.old_path().to_string_lossy().to_string(),
            plan.new_path().to_string_lossy().to_string(),
        ];
        assert!(!plan.old_path().exists());
        owned_proof.validate(plan.new_path()).unwrap();
        foreign_proof.validate(&foreign_path).unwrap();

        let reoccupied_proof = if reoccupy_old_path {
            create_terminal_mod(plan.old_path());
            let proof = FilesystemIdentityProof::capture(plan.old_path()).unwrap();
            assert_ne!(proof.identity(), owned_proof.identity());
            Some(proof)
        } else {
            None
        };
        let mut pending = projection_fixture_operation(&plan);
        if canonical_journal {
            let canonical_root = std::fs::canonicalize(&mods_path).unwrap();
            crate::modules::reconciliation::application::toggle_projection::proven_projection_events(
                &[projection_fixture_operation(&plan)],
                &canonical_root,
            ).expect("regular journal paths must match a canonical root too");
            for step in &mut pending.steps {
                for endpoint in [&mut step.old_path, &mut step.new_path]
                    .into_iter()
                    .flatten()
                {
                    *endpoint = canonical_root.join(endpoint.strip_prefix(&mods_path).unwrap());
                }
            }
        }
        assert!(crate::modules::reconciliation::application::toggle_projection::proven_projection_events(
            std::slice::from_ref(&pending),
            &mods_path.with_file_name("ModsOther"),
        ).is_err(), "a genuinely different parent root must remain rejected");
        let events = crate::modules::reconciliation::application::toggle_projection::proven_projection_events(std::slice::from_ref(&pending), &mods_path).unwrap();

        let repair = if reoccupy_old_path {
            use crate::modules::mutation::api::MutationCoordinator;
            use crate::modules::mutation::journal::{OperationJournal, OperationPlan, PlannedStep};
            use crate::platform::fs::operation_lock::OperationLock;
            let epoch = FilesystemIdentityProof::capture(&mods_path)
                .unwrap()
                .identity()
                .to_string();
            let journal = std::sync::Arc::new(
                OperationJournal::open(temp.path().join("repair-journal.json"), 50).unwrap(),
            );
            let coordinator = MutationCoordinator::with_lock(OperationLock::new(), journal.clone());
            let id = journal
                .plan_operation(
                    OperationPlan::new(
                        "workspace-switch",
                        game_id,
                        vec![PlannedStep::rename(
                            0,
                            plan.old_path().to_path_buf(),
                            plan.new_path().to_path_buf(),
                        )
                        .with_expected_identity(Some(plan.expected_identity().to_string()))],
                    )
                    .with_source_epoch(epoch.clone()),
                )
                .unwrap();
            journal.mark_applying(&id).unwrap();
            journal.mark_step_applied(&id, 0).unwrap();
            journal.mark_disk_committed(&id).unwrap();
            coordinator
                .isolate_projection_for_repair(&id, "owned row has not projected yet")
                .unwrap();
            assert!(crate::modules::reconciliation::application::toggle_projection::settle_repaired_projection_operations(&pool, &coordinator, game_id, &epoch, &mods_path, &[]).await.is_err());
            assert!(coordinator
                .earliest_toggle_projection_repair(game_id, &epoch)
                .unwrap()
                .is_some());
            Some((coordinator, epoch))
        } else {
            None
        };

        let outcome = reconcile_disk_projection(ReconcileDiskProjectionRequest {
            pool: &pool,
            game_id,
            mods_path: &mods_path,
            safe_mode_keywords: &[],
            reason: &DiskReconcileReason::InternalMutation,
            changed_paths: &changed_paths,
            force_full: false,
            watcher_events: Some(&events),
            path_hints: &[],
            trusted_mutation_scope: true,
            progress_reporter: None,
            precomputed_discovery: None,
        })
        .await
        .unwrap();
        assert_eq!(
            outcome.status,
            DiskReconcileStatus::AppliedWithFolderConflicts
        );
        assert!(outcome.status.applied());
        assert_eq!(outcome.folder_conflicts.len(), 1);
        let row = persisted_mod_snapshot(&pool, game_id, "A").await;
        assert_eq!(
            row.id, original_row.id,
            "the indexed mod must retain its DB identity"
        );
        assert_eq!(
            Path::new(&row.folder_path),
            plan.new_path().strip_prefix(&mods_path).unwrap(),
            "an applied result must project the actual toggle path"
        );
        assert_eq!(row.status, ItemStatus::from_is_disabled(!enable) as i64);
        assert_eq!(
            row.filesystem_identity.as_deref(),
            Some(owned_proof.identity())
        );
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = ?")
            .bind(game_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            rows, 1,
            "the distinct conflict candidate must not collapse into the owned row"
        );
        foreign_proof.validate(&foreign_path).unwrap();
        assert!(foreign_path.join("mod.ini").is_file());
        crate::modules::reconciliation::application::toggle_projection::validate_projected_toggle_rows(&pool, game_id, &[pending], &mods_path).await.unwrap();
        if let Some((coordinator, epoch)) = repair {
            crate::modules::reconciliation::application::toggle_projection::settle_repaired_projection_operations(&pool, &coordinator, game_id, &epoch, &mods_path, &[]).await.unwrap();
            assert!(
                coordinator
                    .earliest_toggle_projection_repair(game_id, &epoch)
                    .unwrap()
                    .is_none(),
                "a repair hole clears only after the real owned DB row and disk identity agree"
            );
        }
        if let Some(proof) = reoccupied_proof {
            proof.validate(plan.old_path()).unwrap();
            assert!(plan.old_path().join("mod.ini").is_file());
            return;
        }
        current_path = plan.new_path().to_path_buf();
    }
}

#[tokio::test]
async fn conflicting_enabled_and_disabled_object_roots_block_even_when_mod_names_differ() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    create_terminal_mod(&mods_path.join("DISABLED Alice").join("Red"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_object_conflict",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");

    let outcome = run_reconcile(
        &pool,
        "g_object_conflict",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;

    assert_eq!(
        outcome.status,
        DiskReconcileStatus::AppliedWithFolderConflicts
    );
    assert_eq!(outcome.folder_conflicts.len(), 1);
    assert_eq!(outcome.folder_conflicts[0].identity, "alice");
    assert_eq!(outcome.folder_conflicts[0].candidates.len(), 2);
    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects")
        .fetch_one(&pool)
        .await
        .unwrap();
    let mod_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((object_count, mod_count), (0, 0));
}

#[tokio::test]
async fn offline_semantic_rename_without_identity_requires_confirmation_and_keeps_db_snapshot() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let old_path = mods_path.join("Alice").join("Old");
    let new_path = mods_path.join("Alice").join("New");
    create_terminal_mod(&old_path);
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_missing_identity",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");

    run_reconcile(
        &pool,
        "g_missing_identity",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    sqlx::query("UPDATE mods SET filesystem_identity = NULL WHERE game_id = ?")
        .bind("g_missing_identity")
        .execute(&pool)
        .await
        .expect("identity should be cleared");
    fs::rename(&old_path, &new_path).expect("offline rename");

    let outcome = run_reconcile(
        &pool,
        "g_missing_identity",
        &mods_path,
        DiskReconcileReason::StartupBoot,
        &[],
        true,
    )
    .await;

    assert_eq!(outcome.status, DiskReconcileStatus::NeedsRenameConfirmation);
    assert_eq!(outcome.rename_confirmations.len(), 1);
    assert_eq!(
        outcome.rename_confirmations[0].previous_paths,
        [Path::new("Alice").join("Old").to_string_lossy().to_string()]
    );
    assert_eq!(
        outcome.rename_confirmations[0].current_paths,
        [Path::new("Alice").join("New").to_string_lossy().to_string()]
    );
    let stored_path: String = sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ?")
        .bind("g_missing_identity")
        .fetch_one(&pool)
        .await
        .expect("old row should remain");
    assert!(stored_path.ends_with("Alice\\Old") || stored_path.ends_with("Alice/Old"));

    let resolution_event =
        crate::modules::workspace::application::scanner::watcher::ModWatchEvent::RenameResolution {
            group_id: outcome.rename_confirmations[0].group_id.clone(),
            from: Some(old_path.to_string_lossy().to_string()),
            to: Some(new_path.to_string_lossy().to_string()),
            apply_as_rename: true,
        };
    let resolved = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id: "g_missing_identity",
        mods_path: &mods_path,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::ManualRepair,
        changed_paths: &[],
        force_full: true,
        watcher_events: Some(std::slice::from_ref(&resolution_event)),
        path_hints: &[],
        trusted_mutation_scope: false,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .expect("confirmed rename should reconcile");
    assert_eq!(resolved.status, DiskReconcileStatus::Applied);
    let stored_path: String = sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ?")
        .bind("g_missing_identity")
        .fetch_one(&pool)
        .await
        .expect("renamed row should remain");
    assert!(stored_path.ends_with("Alice\\New") || stored_path.ends_with("Alice/New"));
}

#[tokio::test]
async fn ambiguous_rename_protects_only_its_object_scope_while_unrelated_root_projects() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let old_path = mods_path.join("Alice").join("Old");
    let new_path = mods_path.join("Alice").join("New");
    create_terminal_mod(&old_path);
    create_terminal_mod(&mods_path.join("Bob").join("Stable"));
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_scoped_rename_confirmation",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    run_reconcile(
        &pool,
        "g_scoped_rename_confirmation",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    sqlx::query(
        "UPDATE mods SET filesystem_identity = NULL
         WHERE game_id = ? AND actual_name = 'Old'",
    )
    .bind("g_scoped_rename_confirmation")
    .execute(&pool)
    .await
    .expect("ambiguous source identity should be cleared");

    fs::remove_dir_all(&old_path).expect("old folder should be removed");
    create_terminal_mod(&new_path);
    create_terminal_mod(&mods_path.join("Charlie").join("Fresh"));

    let outcome = run_reconcile(
        &pool,
        "g_scoped_rename_confirmation",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;

    assert_eq!(outcome.status, DiskReconcileStatus::NeedsRenameConfirmation);
    assert_eq!(outcome.rename_confirmations.len(), 1);
    assert_eq!(
        outcome.rename_confirmations[0].previous_paths,
        [Path::new("Alice").join("Old").to_string_lossy().to_string()]
    );
    assert_eq!(
        outcome.rename_confirmations[0].current_paths,
        [Path::new("Alice").join("New").to_string_lossy().to_string()]
    );
    assert!(
        mod_row_named(&pool, "g_scoped_rename_confirmation", "Fresh")
            .await
            .is_some(),
        "unrelated Charlie scope must project during Alice ambiguity"
    );
    let projected_names: Vec<String> =
        sqlx::query_scalar("SELECT actual_name FROM mods WHERE game_id = ? ORDER BY actual_name")
            .bind("g_scoped_rename_confirmation")
            .fetch_all(&pool)
            .await
            .expect("projected names should load");
    assert!(
        projected_names.iter().any(|name| name == "Old"),
        "Alice's previous row remains protected until explicit resolution: {projected_names:?}"
    );
    assert!(
        projected_names.iter().all(|name| name != "New"),
        "Alice's current candidate remains overlay-only until explicit resolution: {projected_names:?}"
    );
}

#[tokio::test]
async fn confirmed_separate_change_prunes_old_runtime_row_and_adds_current_folder() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let old_path = mods_path.join("Alice").join("Old");
    let new_path = mods_path.join("Alice").join("New");
    create_terminal_mod(&old_path);
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_separate_change",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    run_reconcile(
        &pool,
        "g_separate_change",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    let old_id: String = sqlx::query_scalar("SELECT id FROM mods WHERE game_id = ?")
        .bind("g_separate_change")
        .fetch_one(&pool)
        .await
        .expect("old id should load");
    sqlx::query("UPDATE mods SET filesystem_identity = NULL WHERE game_id = ?")
        .bind("g_separate_change")
        .execute(&pool)
        .await
        .expect("identity should be cleared");
    fs::rename(&old_path, &new_path).expect("external replacement");
    let blocked = run_reconcile(
        &pool,
        "g_separate_change",
        &mods_path,
        DiskReconcileReason::StartupBoot,
        &[],
        true,
    )
    .await;
    let resolution_event =
        crate::modules::workspace::application::scanner::watcher::ModWatchEvent::RenameResolution {
            group_id: blocked.rename_confirmations[0].group_id.clone(),
            from: None,
            to: None,
            apply_as_rename: false,
        };
    let resolved = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id: "g_separate_change",
        mods_path: &mods_path,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::ManualRepair,
        changed_paths: &[],
        force_full: true,
        watcher_events: Some(std::slice::from_ref(&resolution_event)),
        path_hints: &[],
        trusted_mutation_scope: false,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .expect("separate changes should reconcile");

    assert_eq!(resolved.status, DiskReconcileStatus::Applied);
    let (new_id, stored_path): (String, String) =
        sqlx::query_as("SELECT id, folder_path FROM mods WHERE game_id = ?")
            .bind("g_separate_change")
            .fetch_one(&pool)
            .await
            .expect("current row should load");
    assert_ne!(new_id, old_id);
    assert!(stored_path.ends_with("Alice\\New") || stored_path.ends_with("Alice/New"));
}

#[tokio::test]
async fn watcher_rename_event_is_sufficient_evidence_when_filesystem_identity_is_missing() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    let old_path = mods_path.join("Alice").join("Variants").join("Old");
    let new_path = mods_path.join("Alice").join("Variants").join("New");
    create_terminal_mod(&old_path);
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g_watcher_evidence",
            name: "Game",
            game_type: GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    run_reconcile(
        &pool,
        "g_watcher_evidence",
        &mods_path,
        DiskReconcileReason::ManualRepair,
        &[],
        true,
    )
    .await;
    sqlx::query("UPDATE mods SET filesystem_identity = NULL WHERE game_id = ?")
        .bind("g_watcher_evidence")
        .execute(&pool)
        .await
        .expect("identity should be cleared");
    fs::rename(&old_path, &new_path).expect("watcher rename");
    let event = crate::modules::workspace::application::scanner::watcher::ModWatchEvent::Renamed {
        from: old_path.to_string_lossy().to_string(),
        to: new_path.to_string_lossy().to_string(),
    };
    let outcome = reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool: &pool,
        game_id: "g_watcher_evidence",
        mods_path: &mods_path,
        safe_mode_keywords: &[],
        reason: &DiskReconcileReason::WatcherBatch,
        changed_paths: &[
            old_path.to_string_lossy().to_string(),
            new_path.to_string_lossy().to_string(),
        ],
        force_full: false,
        watcher_events: Some(std::slice::from_ref(&event)),
        path_hints: &[],
        trusted_mutation_scope: false,
        progress_reporter: None,
        precomputed_discovery: None,
    })
    .await
    .expect("watcher evidence should reconcile");

    assert_eq!(outcome.status, DiskReconcileStatus::Applied);
    assert!(outcome.rename_confirmations.is_empty());
    let stored_path: String = sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ?")
        .bind("g_watcher_evidence")
        .fetch_one(&pool)
        .await
        .expect("renamed row should load");
    assert!(
        stored_path.ends_with("Alice\\Variants\\New")
            || stored_path.ends_with("Alice/Variants/New")
    );
}
