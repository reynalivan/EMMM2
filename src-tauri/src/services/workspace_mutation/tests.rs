use super::import_commit::{
    finalize_ready_to_move_archives, fingerprint_matches, rollback_move_journal, MoveJournalEntry,
};
use crate::services::import_batch::types::{
    ImportBatchStatus, ImportFlow, ImportSourceKind, TargetMode,
};
use crate::services::match_engine::inspection::{inspect_source, InspectionRequest};
use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};

#[test]
fn stale_fingerprint_detects_source_content_change() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("mod");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("mod.ini"), "[TextureOverrideBody]").unwrap();
    let before = inspect_source(&InspectionRequest {
        source_path: source.clone(),
        planned_name: None,
        match_extensions: vec!["ini".to_string()],
    })
    .unwrap()
    .fingerprint;
    std::fs::write(source.join("body.dds"), "new").unwrap();
    let after = inspect_source(&InspectionRequest {
        source_path: source,
        planned_name: None,
        match_extensions: vec!["ini".to_string(), "dds".to_string()],
    })
    .unwrap()
    .fingerprint;
    assert!(!fingerprint_matches(&before, &after));
}

#[test]
fn fingerprint_accepts_canonical_equivalent_source_path() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("extracted-mod");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("mod.ini"), "[TextureOverrideBody]").unwrap();
    let expected = inspect_source(&InspectionRequest {
        source_path: source.clone(),
        planned_name: None,
        match_extensions: vec!["ini".to_string()],
    })
    .unwrap()
    .fingerprint;
    let current = inspect_source(&InspectionRequest {
        source_path: source.canonicalize().unwrap(),
        planned_name: None,
        match_extensions: vec!["ini".to_string()],
    })
    .unwrap()
    .fingerprint;

    assert!(
        fingerprint_matches(&expected, &current),
        "canonical and non-canonical representations must identify the same staged source"
    );
}

#[test]
fn move_journal_rolls_back_in_reverse_order() {
    let root = tempfile::tempdir().unwrap();
    let first_source = root.path().join("first-source");
    let first_target = root.path().join("first-target");
    let second_source = root.path().join("second-source");
    let second_target = root.path().join("second-target");
    std::fs::create_dir(&first_target).unwrap();
    std::fs::create_dir(&second_target).unwrap();

    rollback_move_journal(&[
        MoveJournalEntry::new(first_source.clone(), first_target.clone()),
        MoveJournalEntry::new(second_source.clone(), second_target.clone()),
    ])
    .unwrap();

    assert!(first_source.exists());
    assert!(second_source.exists());
    assert!(!first_target.exists());
    assert!(!second_target.exists());
}

#[test]
fn move_journal_removes_owned_target_when_source_still_exists() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let target = root.path().join("partial-target");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&target).unwrap();
    std::fs::write(source.join("mod.ini"), "source").unwrap();
    std::fs::write(target.join("mod.ini"), "partial").unwrap();

    rollback_move_journal(&[MoveJournalEntry::new(source.clone(), target.clone())]).unwrap();

    assert!(source.join("mod.ini").is_file());
    assert!(!target.exists());
}

#[tokio::test]
async fn ready_to_move_archive_failure_stays_resumable_until_processed_move_succeeds() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("Ayaka.zip");
    std::fs::write(&archive, b"archive").unwrap();
    crate::repo::import_batch_repo::create_batch(
        &context.pool,
        &crate::repo::import_batch_repo::CreateImportBatchRecord {
            id: "batch-archive".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::ReadyToMove,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[crate::repo::import_batch_repo::NewImportItemRecord {
            id: "item-archive".to_string(),
            source_kind: ImportSourceKind::ReadyToMove,
            source_path: archive.to_string_lossy().into_owned(),
            staging_path: Some(
                root.path()
                    .join("staged/Ayaka")
                    .to_string_lossy()
                    .into_owned(),
            ),
            planned_name: "DISABLED Ayaka".to_string(),
        }],
    )
    .await
    .unwrap();
    sqlx::raw_sql(
        "UPDATE import_batches SET status = 'committing' WHERE id = 'batch-archive';
         UPDATE import_jobs SET status = 'done', result = 'done' WHERE id = 'item-archive';",
    )
    .execute(&context.pool)
    .await
    .unwrap();
    std::fs::write(root.path().join("Processed"), b"blocks directory creation").unwrap();

    let error = finalize_ready_to_move_archives(&context.pool, "batch-archive")
        .await
        .unwrap_err();
    crate::repo::import_batch_repo::mark_ready_to_move_archive_pending(
        &context.pool,
        "batch-archive",
        &error.to_string(),
    )
    .await
    .unwrap();
    let pending = crate::repo::import_batch_repo::get_batch(&context.pool, "batch-archive")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.status, ImportBatchStatus::Partial);
    assert_eq!(pending.items[0].result.as_deref(), Some("archive_pending"));
    assert!(archive.exists());

    std::fs::remove_file(root.path().join("Processed")).unwrap();
    finalize_ready_to_move_archives(&context.pool, "batch-archive")
        .await
        .unwrap();
    crate::repo::import_batch_repo::complete_ready_to_move_archive_pending(
        &context.pool,
        "batch-archive",
    )
    .await
    .unwrap();
    assert_eq!(
        crate::repo::import_batch_repo::finish_batch_from_items(&context.pool, "batch-archive",)
            .await
            .unwrap(),
        ImportBatchStatus::Done
    );
    assert!(!archive.exists());
    assert!(root.path().join("Processed/Ayaka.zip").is_file());
    let history: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT processed_source_path, source_processed_at FROM import_jobs
         WHERE id = 'item-archive'",
    )
    .fetch_one(&context.pool)
    .await
    .unwrap();
    assert_eq!(
        history.0.map(std::path::PathBuf::from),
        Some(root.path().join("Processed/Ayaka.zip"))
    );
    assert!(history.1.is_some());
    finalize_ready_to_move_archives(&context.pool, "batch-archive")
        .await
        .expect("processed finalization must be idempotent after a crash/retry");
}

#[tokio::test]
async fn ready_to_move_folder_pack_is_retained_in_processed_with_history() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let pack = root.path().join("Character Pack");
    std::fs::create_dir(&pack).unwrap();
    crate::repo::import_batch_repo::create_batch(
        &context.pool,
        &crate::repo::import_batch_repo::CreateImportBatchRecord {
            id: "batch-pack".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::ReadyToMove,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[crate::repo::import_batch_repo::NewImportItemRecord {
            id: "item-pack".to_string(),
            source_kind: ImportSourceKind::ReadyToMove,
            source_path: pack.to_string_lossy().into_owned(),
            staging_path: Some(
                root.path()
                    .join("staged/Ayaka")
                    .to_string_lossy()
                    .into_owned(),
            ),
            planned_name: "DISABLED Ayaka".to_string(),
        }],
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE import_jobs SET status = 'done', result = 'done',
         placed_path = 'C:/Games/Genshin/Mods/Ayaka/DISABLED Ayaka'
         WHERE id = 'item-pack'",
    )
    .execute(&context.pool)
    .await
    .unwrap();

    finalize_ready_to_move_archives(&context.pool, "batch-pack")
        .await
        .unwrap();

    assert!(!pack.exists());
    assert!(root.path().join("Processed/Character Pack").is_dir());
    let processed: Option<String> =
        sqlx::query_scalar("SELECT processed_source_path FROM import_jobs WHERE id = 'item-pack'")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    assert_eq!(
        processed.map(std::path::PathBuf::from),
        Some(root.path().join("Processed/Character Pack"))
    );
}

#[tokio::test]
async fn ready_to_move_finalization_recovers_a_crash_after_the_source_move() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("Crash Pack");
    let processed = root.path().join("Processed");
    let target = processed.join("Crash Pack");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&processed).unwrap();
    crate::repo::import_batch_repo::create_batch(
        &context.pool,
        &crate::repo::import_batch_repo::CreateImportBatchRecord {
            id: "batch-crash".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::ReadyToMove,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[crate::repo::import_batch_repo::NewImportItemRecord {
            id: "item-crash".to_string(),
            source_kind: ImportSourceKind::ReadyToMove,
            source_path: source.to_string_lossy().into_owned(),
            staging_path: Some(
                root.path()
                    .join("staged/mod")
                    .to_string_lossy()
                    .into_owned(),
            ),
            planned_name: "DISABLED Crash".to_string(),
        }],
    )
    .await
    .unwrap();
    sqlx::query("UPDATE import_jobs SET status = 'done', result = 'done' WHERE id = 'item-crash'")
        .execute(&context.pool)
        .await
        .unwrap();
    crate::repo::import_batch_repo::plan_mod_inbox_source_processing(
        &context.pool,
        "batch-crash",
        &source.to_string_lossy(),
        &target.to_string_lossy(),
    )
    .await
    .unwrap();
    std::fs::rename(&source, &target).unwrap();

    finalize_ready_to_move_archives(&context.pool, "batch-crash")
        .await
        .unwrap();

    assert!(!source.exists());
    assert!(target.is_dir());
    let completed: Option<String> =
        sqlx::query_scalar("SELECT source_processed_at FROM import_jobs WHERE id = 'item-crash'")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    assert!(completed.is_some());
}
