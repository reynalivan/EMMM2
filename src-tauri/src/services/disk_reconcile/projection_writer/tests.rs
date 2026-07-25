use std::path::{Path, PathBuf};

use crate::common::corridor_constants::CORRIDOR_SOURCE_UNKNOWN;
use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::models::{GameType, ItemStatus};
use crate::services::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::services::disk_reconcile::disk_snapshot::collect_disk_projection;
use crate::services::disk_reconcile::helpers::generate_stable_mod_id;
use crate::services::disk_reconcile::types::{DiskReconcilePathKind, DiskReconcilePathUpdate};
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
    TestModFixture, TestObjectFixture,
};

use super::{reconcile_projection_in_tx, ProjectionWriteRequest};

struct WriterRun {
    objects_changed: bool,
    folders_changed: bool,
    path_updates: Vec<DiskReconcilePathUpdate>,
    change_summary: crate::services::disk_reconcile::types::DiskReconcileChangeSummary,
}

async fn run_writer(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &Path,
    changed_roots: &[String],
    force_full: bool,
) -> WriterRun {
    let projection = collect_disk_projection(mods_path, changed_roots, false)
        .expect("disk projection should be collected");
    let mut path_updates = Vec::new();
    let mut impact = CollectionReferenceImpact::default();
    let mut change_summary = ChangeSummaryBuilder::default();

    let mut tx = pool.begin().await.expect("tx should begin");
    let (objects_changed, folders_changed) = reconcile_projection_in_tx(
        &mut tx,
        ProjectionWriteRequest {
            game_id,
            mods_path,
            safe_mode_keywords: &[],
            projection: &projection,
            changed_roots,
            force_full,
            path_updates: &mut path_updates,
            collection_reference_impact: &mut impact,
            change_summary: &mut change_summary,
        },
    )
    .await
    .expect("projection write should succeed");
    tx.commit().await.expect("tx should commit");

    WriterRun {
        objects_changed,
        folders_changed,
        path_updates,
        change_summary: change_summary.build(),
    }
}

fn create_terminal_mod(mods_path: &Path, object: &str, mod_folder: &str) -> PathBuf {
    let terminal = mods_path.join(object).join(mod_folder);
    std::fs::create_dir_all(&terminal).expect("mod folder should be created");
    std::fs::write(
        terminal.join("mod.ini"),
        "[TextureOverrideAlice]\nhash = abc\n",
    )
    .expect("ini should be written");
    std::fs::write(terminal.join("mesh.buf"), "mesh").expect("asset should be written");
    terminal
}

async fn seed_game(pool: &sqlx::SqlitePool, game_id: &str, mods_path: &Path) -> String {
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        pool,
        &TestGameFixture {
            id: game_id,
            name: "Game",
            game_type: GameType::GIMI,
            path: mods_path.parent().unwrap().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game should be inserted");
    mods_path_string
}

#[tokio::test]
async fn inserts_new_objects_and_mods_discovered_on_disk() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "Blue Dress");
    seed_game(&ctx.pool, "game-1", &mods_path).await;

    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    assert!(run.objects_changed);
    assert!(run.folders_changed);
    assert!(run.path_updates.is_empty());

    let object_row: (String, String, i64) =
        sqlx::query_as("SELECT folder_path, object_type, status FROM objects WHERE game_id = ?")
            .bind("game-1")
            .fetch_one(&ctx.pool)
            .await
            .expect("object row");
    assert_eq!(object_row.0, "Alice");
    assert_eq!(object_row.1, "Other");
    assert_eq!(object_row.2, ItemStatus::Enabled as i64);

    let expected_rel = PathBuf::from("Alice")
        .join("Blue Dress")
        .to_string_lossy()
        .to_string();
    let mod_row: (String, String, i64, String) = sqlx::query_as(
        "SELECT id, folder_path, status, corridor_source FROM mods WHERE game_id = ?",
    )
    .bind("game-1")
    .fetch_one(&ctx.pool)
    .await
    .expect("mod row");
    assert_eq!(mod_row.0, generate_stable_mod_id("game-1", &expected_rel));
    assert_eq!(mod_row.1, expected_rel);
    assert_eq!(mod_row.2, ItemStatus::Enabled as i64);
    assert_eq!(mod_row.3, CORRIDOR_SOURCE_UNKNOWN);

    assert_eq!(run.change_summary.object_changes.added, 1);
    assert_eq!(run.change_summary.mod_changes.added, 1);
    assert!(run.change_summary.has_user_visible_changes);
}

#[tokio::test]
async fn heals_disabled_prefix_rename_by_runtime_key_and_records_path_update() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    create_terminal_mod(&mods_path, "Alice", "DISABLED Blue Dress");
    let mods_path_string = seed_game(&ctx.pool, "game-1", &mods_path).await;

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "obj-alice",
            game_id: "game-1",
            name: "Alice",
            folder_path: "Alice",
            object_type: "Character",
        },
    )
    .await
    .expect("object seed");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-blue",
            game_id: "game-1",
            object_id: Some("obj-alice"),
            actual_name: "Blue Dress",
            folder_path: "Alice/Blue Dress",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("mod seed");

    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    assert!(run.folders_changed);
    let expected_rel = PathBuf::from("Alice")
        .join("DISABLED Blue Dress")
        .to_string_lossy()
        .to_string();
    let mod_row: (String, String, i64) =
        sqlx::query_as("SELECT id, folder_path, status FROM mods WHERE game_id = ?")
            .bind("game-1")
            .fetch_one(&ctx.pool)
            .await
            .expect("mod row");
    assert_eq!(mod_row.0, generate_stable_mod_id("game-1", &expected_rel));
    assert_eq!(mod_row.1, expected_rel);
    assert_eq!(mod_row.2, ItemStatus::Disabled as i64);

    assert_eq!(run.path_updates.len(), 1);
    assert_eq!(run.path_updates[0].kind, DiskReconcilePathKind::Mod);
    assert_eq!(run.path_updates[0].from, "Alice/Blue Dress");
    assert_eq!(run.path_updates[0].to, expected_rel);
    assert_eq!(run.change_summary.mod_changes.renamed, 1);
}

#[tokio::test]
async fn force_full_deletes_db_rows_missing_from_disk() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    let mods_path_string = seed_game(&ctx.pool, "game-1", &mods_path).await;

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "obj-bob",
            game_id: "game-1",
            name: "Bob",
            folder_path: "Bob",
            object_type: "Character",
        },
    )
    .await
    .expect("object seed");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-red",
            game_id: "game-1",
            object_id: Some("obj-bob"),
            actual_name: "Red",
            folder_path: "Bob/Red",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("mod seed");

    let run = run_writer(&ctx.pool, "game-1", &mods_path, &[], true).await;

    assert!(run.objects_changed);
    assert!(run.folders_changed);
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
    assert_eq!(object_count, 0);
    assert_eq!(mod_count, 0);

    assert_eq!(run.change_summary.object_changes.removed, 1);
    // Mods removed through their parent object cascade are NOT counted
    // in the mod change summary (current behavior).
    assert_eq!(run.change_summary.mod_changes.removed, 0);
}

#[tokio::test]
async fn scoped_run_leaves_out_of_scope_rows_untouched() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    let mods_path_string = seed_game(&ctx.pool, "game-1", &mods_path).await;

    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "obj-bob",
            game_id: "game-1",
            name: "Bob",
            folder_path: "Bob",
            object_type: "Character",
        },
    )
    .await
    .expect("object seed");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-red",
            game_id: "game-1",
            object_id: Some("obj-bob"),
            actual_name: "Red",
            folder_path: "Bob/Red",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("mod seed");

    // Scope only covers "Alice"; "Bob" is stale on disk but must survive.
    let run = run_writer(
        &ctx.pool,
        "game-1",
        &mods_path,
        &["Alice".to_string()],
        false,
    )
    .await;

    assert!(!run.objects_changed);
    assert!(!run.folders_changed);
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
    assert!(!run.change_summary.has_user_visible_changes);
}
