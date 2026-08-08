use super::*;
use std::path::PathBuf;

use crate::domain::models::{GameType, ItemStatus};
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object, TestGameFixture,
    TestModFixture, TestObjectFixture,
};

async fn seed_game_object_mod(
    pool: &sqlx::SqlitePool,
    mods_path: &Path,
    mod_folder_path: &str,
) -> String {
    let mods_path_string = mods_path.to_string_lossy().to_string();
    insert_test_game(
        pool,
        &TestGameFixture {
            id: "game-1",
            name: "Game",
            game_type: GameType::GIMI,
            path: mods_path.parent().unwrap().to_string_lossy().as_ref(),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("game seed");
    insert_test_object(
        pool,
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
        pool,
        &TestModFixture {
            id: "mod-old",
            game_id: "game-1",
            object_id: Some("obj-alice"),
            actual_name: "Old Mod",
            folder_path: mod_folder_path,
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path_string),
        },
    )
    .await
    .expect("mod seed");
    mods_path_string
}

struct HealerRun {
    path_updates: Vec<DiskReconcilePathUpdate>,
}

async fn run_healer(
    pool: &sqlx::SqlitePool,
    mods_path: &Path,
    events: &[ModWatchEvent],
) -> HealerRun {
    let mut path_updates = Vec::new();
    let mut impact = CollectionReferenceImpact::default();
    let mut change_summary = ChangeSummaryBuilder::default();

    let mut tx = pool.begin().await.expect("tx begin");
    apply_watcher_rename_hints(WatcherRenameHintsApplyRequest {
        conn: &mut tx,
        game_id: "game-1",
        mods_path,
        safe_mode_keywords: &[],
        watcher_events: events,
        path_updates: &mut path_updates,
        collection_reference_impact: &mut impact,
        change_summary: &mut change_summary,
    })
    .await
    .expect("healer should succeed");
    tx.commit().await.expect("tx commit");

    HealerRun { path_updates }
}

#[tokio::test]
async fn mod_rename_hint_rewrites_row_identity_and_path() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    seed_game_object_mod(&ctx.pool, &mods_path, "Alice/Old Mod").await;

    let events = vec![ModWatchEvent::Renamed {
        from: mods_path
            .join("Alice")
            .join("Old Mod")
            .to_string_lossy()
            .to_string(),
        to: mods_path
            .join("Alice")
            .join("New Mod")
            .to_string_lossy()
            .to_string(),
    }];
    let run = run_healer(&ctx.pool, &mods_path, &events).await;

    let expected_rel = PathBuf::from("Alice")
        .join("New Mod")
        .to_string_lossy()
        .to_string();
    let mod_row: (String, String, String, Option<String>, i64) = sqlx::query_as(
        "SELECT id, folder_path, actual_name, object_id, status FROM mods WHERE game_id = ?",
    )
    .bind("game-1")
    .fetch_one(&ctx.pool)
    .await
    .expect("mod row");
    assert_eq!(mod_row.0, generate_stable_id("game-1", &expected_rel));
    assert_eq!(mod_row.1, expected_rel);
    assert_eq!(mod_row.2, "New Mod");
    assert_eq!(mod_row.3.as_deref(), Some("obj-alice"));
    assert_eq!(mod_row.4, ItemStatus::Enabled as i64);

    assert_eq!(run.path_updates.len(), 1);
    assert_eq!(run.path_updates[0].kind, DiskReconcilePathKind::Mod);
    assert_eq!(run.path_updates[0].to, expected_rel);
}

#[tokio::test]
async fn object_rename_hint_updates_object_status_and_child_mod_paths() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    seed_game_object_mod(&ctx.pool, &mods_path, "Alice/Blue").await;

    let events = vec![ModWatchEvent::Renamed {
        from: mods_path.join("Alice").to_string_lossy().to_string(),
        to: mods_path
            .join("DISABLED Alice")
            .to_string_lossy()
            .to_string(),
    }];
    let run = run_healer(&ctx.pool, &mods_path, &events).await;

    let object_row: (String, i64) =
        sqlx::query_as("SELECT folder_path, status FROM objects WHERE game_id = ?")
            .bind("game-1")
            .fetch_one(&ctx.pool)
            .await
            .expect("object row");
    assert_eq!(object_row.0, "DISABLED Alice");
    assert_eq!(object_row.1, ItemStatus::Disabled as i64);

    let mod_folder_path: String =
        sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ?")
            .bind("game-1")
            .fetch_one(&ctx.pool)
            .await
            .expect("mod row");
    assert_eq!(mod_folder_path, "DISABLED Alice/Blue");

    assert_eq!(run.path_updates.len(), 1);
    assert_eq!(run.path_updates[0].kind, DiskReconcilePathKind::Object);
    assert_eq!(run.path_updates[0].from, "Alice");
    assert_eq!(run.path_updates[0].to, "DISABLED Alice");
}

#[tokio::test]
async fn events_without_rename_hints_are_a_no_op() {
    let ctx = init_test_db().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_path = temp.path().join("Mods");
    std::fs::create_dir_all(&mods_path).expect("mods root");
    seed_game_object_mod(&ctx.pool, &mods_path, "Alice/Blue").await;

    let outside_root = temp.path().join("Elsewhere");
    let events = vec![
        ModWatchEvent::Created(
            mods_path
                .join("Alice")
                .join("Fresh")
                .to_string_lossy()
                .to_string(),
        ),
        // Rename outside the mods root must be ignored.
        ModWatchEvent::Renamed {
            from: outside_root.join("Alice").to_string_lossy().to_string(),
            to: outside_root.join("Bob").to_string_lossy().to_string(),
        },
    ];
    let run = run_healer(&ctx.pool, &mods_path, &events).await;

    assert!(run.path_updates.is_empty());
    let mod_folder_path: String =
        sqlx::query_scalar("SELECT folder_path FROM mods WHERE game_id = ?")
            .bind("game-1")
            .fetch_one(&ctx.pool)
            .await
            .expect("mod row");
    assert_eq!(mod_folder_path, "Alice/Blue");
}
