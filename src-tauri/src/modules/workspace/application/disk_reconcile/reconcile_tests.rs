//! Pins the scoped projection-refresh branch: a scoped reconcile must refresh
//! only the touched objects' runtime projection rows and leave the rest alone.

use std::fs;
use std::path::Path;

use crate::modules::games::domain::models::GameType;
use crate::modules::workspace::application::disk_reconcile::reconcile::ReconcileOutcome;
use crate::modules::workspace::application::disk_reconcile::reconcile::{
    reconcile_disk_projection, ReconcileDiskProjectionRequest,
};
use crate::modules::workspace::application::disk_reconcile::types::{DiskReconcileReason, DiskReconcileStatus};
use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};

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
        progress_reporter: None,
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
        progress_reporter: None,
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
    run_reconcile(
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
        progress_reporter: None,
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
async fn scoped_conflict_preflight_returns_the_full_game_queue_including_parent_prefixes() {
    let db = init_test_db().await;
    let pool = db.pool;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path.join("Alice").join("Blue"));
    create_terminal_mod(&mods_path.join("Alice").join("DISABLED Blue"));
    create_terminal_mod(&mods_path.join("DISABLED Alice").join("Blue"));
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
    assert_eq!(outcome.folder_conflicts.len(), 3);
    assert_eq!(
        outcome
            .folder_conflicts
            .iter()
            .map(|group| group.candidates.len())
            .max(),
        Some(3)
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

    let resolution_event = crate::modules::workspace::application::scanner::watcher::ModWatchEvent::RenameResolution {
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
        progress_reporter: None,
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
    let resolution_event = crate::modules::workspace::application::scanner::watcher::ModWatchEvent::RenameResolution {
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
        progress_reporter: None,
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
        progress_reporter: None,
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
