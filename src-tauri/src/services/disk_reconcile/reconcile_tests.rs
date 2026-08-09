//! Pins the scoped projection-refresh branch: a scoped reconcile must refresh
//! only the touched objects' runtime projection rows and leave the rest alone.

use std::fs;
use std::path::Path;

use crate::domain::models::GameType;
use crate::services::disk_reconcile::reconcile::{
    reconcile_disk_projection, ReconcileDiskProjectionRequest,
};
use crate::services::disk_reconcile::types::DiskReconcileReason;
use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};

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
) {
    reconcile_disk_projection(ReconcileDiskProjectionRequest {
        pool,
        game_id,
        mods_path,
        safe_mode_keywords: &[],
        reason: &reason,
        changed_paths,
        force_full,
        watcher_events: None,
    })
    .await
    .expect("reconcile should succeed");
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
