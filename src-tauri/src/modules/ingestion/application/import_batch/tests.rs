use super::coordinator::{
    can_keep_separate, create_import_batch, rename_import_item_plan,
    set_import_item_classification, set_import_item_decision,
};
use super::types::{ConfidenceTier, ImportItemStatus};
use super::types::{
    CreateImportBatchInput, ImportDecision, ImportFlow, ImportSourceInput, ImportSourceKind,
    RenameImportItemInput, SetImportItemClassificationInput, SetImportItemDecisionInput,
    StableCategory, TargetComparisonOutcome, TargetMode,
};
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture,
};
use std::io::Write;

#[tokio::test]
async fn cancel_and_wait_blocks_until_lease_completion_before_cleanup() {
    let state = std::sync::Arc::new(super::extraction_state::ImportExtractionState::default());
    let lease = state.acquire("batch-under-test").unwrap();
    let cancel_token = lease.cancel_token();
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("active-staging");
    std::fs::create_dir(&staging).unwrap();
    std::fs::write(staging.join("partial.ini"), "still writing").unwrap();
    let staging_for_cancel = staging.clone();
    let state_for_cancel = state.clone();

    let cancel_task = tokio::spawn(async move {
        let cancellation = state_for_cancel.cancel_and_wait("batch-under-test").await;
        assert!(staging_for_cancel.exists());
        std::fs::remove_dir_all(&staging_for_cancel).unwrap();
        drop(cancellation);
    });

    while !cancel_token.load(std::sync::atomic::Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }
    assert!(!cancel_task.is_finished());
    assert!(staging.exists());

    drop(lease);
    cancel_task.await.unwrap();
    assert!(!staging.exists());
}

#[test]
fn confidence_tiers_follow_the_match_wizard_contract() {
    assert_eq!(ConfidenceTier::from_percentage(100), ConfidenceTier::High);
    assert_eq!(ConfidenceTier::from_percentage(75), ConfidenceTier::High);
    assert_eq!(ConfidenceTier::from_percentage(74), ConfidenceTier::Medium);
    assert_eq!(ConfidenceTier::from_percentage(45), ConfidenceTier::Medium);
    assert_eq!(ConfidenceTier::from_percentage(44), ConfidenceTier::Low);
    assert_eq!(ConfidenceTier::from_percentage(15), ConfidenceTier::Low);
    assert_eq!(ConfidenceTier::from_percentage(14), ConfidenceTier::NoMatch);
    assert_eq!(ConfidenceTier::from_percentage(0), ConfidenceTier::NoMatch);
}

#[test]
fn only_content_collisions_allow_keep_separate() {
    assert!(can_keep_separate(
        TargetComparisonOutcome::TargetHasAdditionalFiles
    ));
    assert!(can_keep_separate(
        TargetComparisonOutcome::SameNameDifferentContent
    ));
    assert!(!can_keep_separate(
        TargetComparisonOutcome::AlreadyInstalled
    ));
    assert!(!can_keep_separate(TargetComparisonOutcome::Incomplete));
}

#[test]
fn item_state_machine_rejects_matching_before_category_decision() {
    assert!(!ImportItemStatus::AwaitingCategory.can_refresh_object_suggestions());
    assert!(ImportItemStatus::AwaitingCategory.can_transition_to(ImportItemStatus::Skipped));
    assert!(
        ImportItemStatus::AwaitingCategory.can_transition_to(ImportItemStatus::AwaitingDestination)
    );
    assert!(ImportItemStatus::AwaitingDestination.can_refresh_object_suggestions());
}

#[test]
fn item_state_machine_supports_commit_recovery_states() {
    assert!(ImportItemStatus::Ready.can_transition_to(ImportItemStatus::Committing));
    assert!(ImportItemStatus::Committing.can_transition_to(ImportItemStatus::Reconciling));
    assert!(ImportItemStatus::Committing.can_transition_to(ImportItemStatus::Ready));
    assert!(ImportItemStatus::Reconciling.can_transition_to(ImportItemStatus::FinalizingMetadata));
    assert!(
        ImportItemStatus::FinalizingMetadata.can_transition_to(ImportItemStatus::MetadataPending)
    );
    assert!(
        ImportItemStatus::MetadataPending.can_transition_to(ImportItemStatus::FinalizingMetadata)
    );
    assert!(!ImportItemStatus::Done.can_transition_to(ImportItemStatus::Committing));
}

#[tokio::test]
async fn create_batch_is_read_only_and_rename_changes_only_the_plan() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("DISABLED ayaka-12319mods");
    std::fs::create_dir(&source).unwrap();

    let batch = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: source.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::Folder),
            }],
        },
    )
    .await
    .unwrap();
    let item_id = batch.items[0].id.clone();

    let renamed = rename_import_item_plan(
        &context.pool,
        RenameImportItemInput {
            item_id,
            planned_name: "Ayaka Spring".to_string(),
        },
    )
    .await
    .unwrap();

    assert_eq!(renamed.planned_name, "DISABLED Ayaka Spring");
    assert!(source.exists());
    assert!(!root.path().join("DISABLED Ayaka Spring").exists());
}

#[tokio::test]
async fn import_rejects_sources_inside_any_configured_game_workspace() {
    let context = init_test_db().await;
    let root = tempfile::tempdir().unwrap();
    let first_mods = root.path().join("game-a/Mods");
    let second_mods = root.path().join("game-b/Mods");
    let source = first_mods.join("Ayaka/source-mod");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&second_mods).unwrap();
    for (id, name, mods_path) in [
        ("gimi", "Genshin", &first_mods),
        ("srmi", "Star Rail", &second_mods),
    ] {
        insert_test_game(
            &context.pool,
            &TestGameFixture {
                id,
                name,
                game_type: if id == "gimi" {
                    crate::modules::games::domain::models::GameType::GIMI
                } else {
                    crate::modules::games::domain::models::GameType::SRMI
                },
                path: mods_path.to_str().unwrap(),
                mods_path: Some(mods_path.to_str().unwrap()),
            },
        )
        .await
        .unwrap();
    }

    let error = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "srmi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: source.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::Folder),
            }],
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        crate::shared::errors::AppError::Security(_)
    ));
    assert!(source.exists());
}

#[tokio::test]
async fn classification_requires_awaiting_category_and_keeps_user_metadata_authoritative() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("mod");
    std::fs::create_dir(&source).unwrap();
    let batch = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: source.to_string_lossy().into_owned(),
                source_kind: None,
            }],
        },
    )
    .await
    .unwrap();
    let item_id = &batch.items[0].id;
    assert_eq!(batch.items[0].planned_name, "DISABLED mod");

    let error = set_import_item_classification(
        &context.pool,
        SetImportItemClassificationInput {
            item_id: item_id.clone(),
            category: StableCategory::Character,
            sub_category: None,
            metadata: serde_json::json!({"element": "Cryo"}),
        },
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("awaiting_category"));

    assert!(
        crate::modules::ingestion::adapters::sqlite::import_batch::transition_item_status(
            &context.pool,
            item_id,
            ImportItemStatus::Discovered,
            ImportItemStatus::Staged,
        )
        .await
        .unwrap()
    );
    assert!(
        crate::modules::ingestion::adapters::sqlite::import_batch::transition_item_status(
            &context.pool,
            item_id,
            ImportItemStatus::Staged,
            ImportItemStatus::AwaitingCategory,
        )
        .await
        .unwrap()
    );

    let classified = set_import_item_classification(
        &context.pool,
        SetImportItemClassificationInput {
            item_id: item_id.clone(),
            category: StableCategory::Character,
            sub_category: None,
            metadata: serde_json::json!({"element": "Cryo"}),
        },
    )
    .await
    .unwrap();
    assert_eq!(classified.status, ImportItemStatus::AwaitingDestination);
    assert_eq!(classified.classification_metadata["element"], "Cryo");
}

#[tokio::test]
async fn multi_root_archive_becomes_one_batch_item_per_mod_root_without_moving_archive() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("pack.zip");
    let file = std::fs::File::create(&archive).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    for mod_name in ["Ayaka", "Raiden", "Hutao"] {
        writer
            .start_file(format!("{mod_name}/merged.ini"), options)
            .unwrap();
        writer
            .write_all(b"[TextureOverrideBody]\nhash = abc\n")
            .unwrap();
    }
    writer.finish().unwrap();
    let batch = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: archive.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::ArchiveRoot),
            }],
        },
    )
    .await
    .unwrap();

    let staged = super::staging::stage_import_batch_sources(
        &context.pool,
        &batch.id,
        &root.path().join("app-staging"),
    )
    .await
    .unwrap();

    assert_eq!(staged.items.len(), 3);
    assert!(staged
        .items
        .iter()
        .all(|item| item.status == ImportItemStatus::Staged));
    assert_eq!(
        staged
            .items
            .iter()
            .map(|item| item.planned_name.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["DISABLED Ayaka", "DISABLED Hutao", "DISABLED Raiden",])
    );
    assert!(archive.exists());
}

#[tokio::test]
async fn identical_archives_stage_one_representative_and_keep_the_duplicate_source() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi-archive-dedup",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods/character"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let first_archive = root.path().join("Ayaka blue.zip");
    let file = std::fs::File::create(&first_archive).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("Ayaka/merged.ini", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .write_all(b"[TextureOverrideBody]\nhash = abc\n")
        .unwrap();
    writer.finish().unwrap();
    let duplicate_archive = root.path().join("Ayaka blue (download 2).zip");
    std::fs::copy(&first_archive, &duplicate_archive).unwrap();

    let batch = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "gimi-archive-dedup".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![
                ImportSourceInput {
                    path: first_archive.to_string_lossy().into_owned(),
                    source_kind: Some(ImportSourceKind::ArchiveRoot),
                },
                ImportSourceInput {
                    path: duplicate_archive.to_string_lossy().into_owned(),
                    source_kind: Some(ImportSourceKind::ArchiveRoot),
                },
            ],
        },
    )
    .await
    .unwrap();

    let staged = super::staging::stage_import_batch_sources(
        &context.pool,
        &batch.id,
        &root.path().join("app-staging"),
    )
    .await
    .unwrap();
    assert_eq!(staged.items.len(), 2);
    let representative = staged
        .items
        .iter()
        .find(|item| {
            std::path::Path::new(&item.source_path)
                .file_name()
                .is_some_and(|name| name == "Ayaka blue.zip")
        })
        .unwrap_or_else(|| panic!("representative missing: {:#?}", staged.items));
    let duplicate = staged
        .items
        .iter()
        .find(|item| {
            std::path::Path::new(&item.source_path)
                .file_name()
                .is_some_and(|name| name == "Ayaka blue (download 2).zip")
        })
        .unwrap_or_else(|| panic!("duplicate missing: {:#?}", staged.items));
    assert_eq!(representative.status, ImportItemStatus::Staged);
    assert!(representative.archive_sha256.is_some());
    assert_eq!(duplicate.status, ImportItemStatus::Skipped);
    assert_eq!(duplicate.result.as_deref(), Some("duplicate_archive"));
    assert_eq!(
        duplicate.duplicate_of_item_id.as_deref(),
        Some(representative.id.as_str())
    );
    assert!(first_archive.exists());
    assert!(duplicate_archive.exists());
}

#[tokio::test]
async fn target_with_extra_files_requires_review_then_keeps_a_separate_name() {
    let context = init_test_db().await;
    let root = tempfile::tempdir().unwrap();
    let mods_root = root.path().join("Mods/character");
    let target = mods_root.join("Ayaka/DISABLED Spring Skin");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(
        target.join("merged.ini"),
        b"[TextureOverride]\nhash = abc\n",
    )
    .unwrap();
    std::fs::write(target.join("author-notes.txt"), b"extra target file").unwrap();
    let source = root.path().join("downloads/Spring Skin");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("merged.ini"),
        b"[TextureOverride]\nhash = abc\n",
    )
    .unwrap();
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi-target-review",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: root.path().to_str().unwrap(),
            mods_path: Some(mods_root.to_str().unwrap()),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &context.pool,
        &TestObjectFixture {
            id: "object-ayaka-target-review",
            game_id: "gimi-target-review",
            name: "Ayaka",
            folder_path: "Ayaka",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    let batch = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "gimi-target-review".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: source.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::Folder),
            }],
        },
    )
    .await
    .unwrap();
    let item_id = batch.items[0].id.clone();
    assert!(
        crate::modules::ingestion::adapters::sqlite::import_batch::transition_item_status(
            &context.pool,
            &item_id,
            ImportItemStatus::Discovered,
            ImportItemStatus::Staged,
        )
        .await
        .unwrap()
    );
    let manifest = super::payload_manifest::build_payload_manifest(&source, None).unwrap();
    assert!(
        crate::modules::ingestion::adapters::sqlite::import_batch::store_payload_manifest(
            &context.pool,
            &item_id,
            &manifest,
        )
        .await
        .unwrap()
    );
    assert!(
        crate::modules::ingestion::adapters::sqlite::import_batch::transition_item_status(
            &context.pool,
            &item_id,
            ImportItemStatus::Staged,
            ImportItemStatus::AwaitingCategory,
        )
        .await
        .unwrap()
    );
    set_import_item_classification(
        &context.pool,
        SetImportItemClassificationInput {
            item_id: item_id.clone(),
            category: StableCategory::Character,
            sub_category: None,
            metadata: serde_json::json!({}),
        },
    )
    .await
    .unwrap();
    let select_target = SetImportItemDecisionInput {
        item_id: item_id.clone(),
        decision: ImportDecision::Reallocate,
        destination_object_id: Some("object-ayaka-target-review".to_string()),
        destination_path: None,
        canonical_entry_key: None,
        matched_alias: None,
    };
    let reviewed = set_import_item_decision(&context.pool, select_target.clone())
        .await
        .unwrap();
    assert_eq!(reviewed.status, ImportItemStatus::AwaitingDestination);
    assert_eq!(reviewed.decision, ImportDecision::Pending);
    assert_eq!(
        reviewed
            .target_comparison
            .as_ref()
            .map(|value| value.outcome),
        Some(TargetComparisonOutcome::TargetHasAdditionalFiles)
    );
    assert_eq!(
        reviewed
            .target_comparison
            .as_ref()
            .and_then(|value| value.suggested_separate_name.as_deref()),
        Some("Spring Skin (2)")
    );

    let kept_separately = set_import_item_decision(
        &context.pool,
        SetImportItemDecisionInput {
            decision: ImportDecision::KeepSeparate,
            ..select_target
        },
    )
    .await
    .unwrap();
    assert_eq!(kept_separately.status, ImportItemStatus::Ready);
    assert_eq!(kept_separately.decision, ImportDecision::KeepSeparate);
    assert_eq!(kept_separately.planned_name, "Spring Skin (2)");
}

#[tokio::test]
async fn failed_archive_staging_can_be_retried_after_the_source_is_repaired() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("retry.zip");
    std::fs::write(&archive, b"not a zip archive").unwrap();
    let batch = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: archive.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::ArchiveRoot),
            }],
        },
    )
    .await
    .unwrap();
    let staging_root = root.path().join("app-staging");
    assert!(
        super::staging::stage_import_batch_sources(&context.pool, &batch.id, &staging_root,)
            .await
            .is_err()
    );

    let file = std::fs::File::create(&archive).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("Ayaka/merged.ini", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .write_all(b"[TextureOverrideBody]\nhash = abc\n")
        .unwrap();
    writer.finish().unwrap();

    let retried =
        super::staging::stage_import_batch_sources(&context.pool, &batch.id, &staging_root)
            .await
            .unwrap();
    assert_eq!(retried.items.len(), 1);
    assert_eq!(retried.items[0].status, ImportItemStatus::Staged);
    assert_eq!(retried.items[0].planned_name, "DISABLED Ayaka");
}

#[tokio::test]
async fn recovered_analysis_retry_removes_crash_staging_before_new_uuid_tree() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("recovered.zip");
    let file = std::fs::File::create(&archive).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("Ayaka/merged.ini", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .write_all(b"[TextureOverrideBody]\nhash = abc\n")
        .unwrap();
    writer.finish().unwrap();
    let batch = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: archive.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::ArchiveRoot),
            }],
        },
    )
    .await
    .unwrap();
    let staging_root = root.path().join("app-staging");
    let stale_uuid = uuid::Uuid::new_v4().to_string();
    let stale_tree = staging_root
        .join(&batch.id)
        .join(&batch.items[0].id)
        .join(stale_uuid);
    std::fs::create_dir_all(stale_tree.join("extracted")).unwrap();
    std::fs::write(stale_tree.join("extracted/partial.bin"), b"partial").unwrap();
    sqlx::query("UPDATE import_batches SET status = 'analyzing' WHERE id = ?")
        .bind(&batch.id)
        .execute(&context.pool)
        .await
        .unwrap();

    crate::modules::ingestion::adapters::sqlite::import_batch::recover_interrupted_batch_states(
        &context.pool,
    )
    .await
    .unwrap();
    let retried =
        super::staging::stage_import_batch_sources(&context.pool, &batch.id, &staging_root)
            .await
            .unwrap();

    assert_eq!(retried.items[0].status, ImportItemStatus::Staged);
    assert!(!stale_tree.exists());
}

#[tokio::test]
async fn cancelled_staging_does_not_mark_the_item_failed_or_batch_partial() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("cancelled.zip");
    let file = std::fs::File::create(&archive).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("Ayaka/merged.ini", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .write_all(b"[TextureOverrideBody]\nhash = abc\n")
        .unwrap();
    writer.finish().unwrap();
    let batch = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: archive.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::ArchiveRoot),
            }],
        },
    )
    .await
    .unwrap();
    let cancel_token = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

    let error = super::staging::stage_import_batch_sources_with_options(
        &context.pool,
        &batch.id,
        &root.path().join("app-staging"),
        &crate::modules::library::application::mods::archive::StagingExtractOptions {
            cancel_token: Some(cancel_token),
            ..Default::default()
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(error, crate::shared::errors::AppError::Cancelled));
    let reloaded = crate::modules::ingestion::adapters::sqlite::import_batch::get_batch(
        &context.pool,
        &batch.id,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(reloaded.status, super::types::ImportBatchStatus::Analyzing);
    assert_eq!(reloaded.items[0].status, ImportItemStatus::Discovered);
}

#[tokio::test]
async fn ready_to_move_nested_folder_pack_stages_one_item_per_detected_root() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let pack = root.path().join("Downloaded Pack");
    for mod_name in ["Ayaka", "Raiden"] {
        let mod_root = pack.join("wrapped").join(mod_name);
        std::fs::create_dir_all(&mod_root).unwrap();
        std::fs::write(
            mod_root.join("merged.ini"),
            "[TextureOverrideBody]\nhash = abc\n",
        )
        .unwrap();
    }
    let batch = create_import_batch(
        &context.pool,
        CreateImportBatchInput {
            game_id: "gimi".to_string(),
            flow: ImportFlow::ReadyToMove,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: pack.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::ReadyToMove),
            }],
        },
    )
    .await
    .unwrap();

    let staged = super::staging::stage_import_batch_sources(
        &context.pool,
        &batch.id,
        &root.path().join("app-staging"),
    )
    .await
    .unwrap();

    assert_eq!(staged.items.len(), 2);
    assert!(staged
        .items
        .iter()
        .all(|item| { item.status == ImportItemStatus::Staged && item.staging_path.is_some() }));
    assert_eq!(
        staged
            .items
            .iter()
            .map(|item| item.planned_name.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["DISABLED Ayaka", "DISABLED Raiden"])
    );
    assert!(pack.exists(), "staging must not mutate the inbox source");
}

#[tokio::test]
async fn mod_inbox_snapshot_reports_missing_root_without_creating_it() {
    let context = init_test_db().await;
    let root = tempfile::tempdir().unwrap().path().join("not-created");

    let snapshot = super::mod_inbox::build_mod_inbox_snapshot(&context.pool, "gimi", &root)
        .await
        .unwrap();

    assert_eq!(
        snapshot.root_state,
        super::types::ModInboxRootState::Missing
    );
    assert!(snapshot.ready_entries.is_empty());
    assert!(!root.exists());
}

#[tokio::test]
async fn mod_inbox_snapshot_classifies_direct_wrapper_and_pack_folders() {
    let context = init_test_db().await;
    let root = tempfile::tempdir().unwrap();
    let direct = root.path().join("Ayaka");
    let wrapper = root.path().join("Wrapped Raiden").join("inner");
    let pack = root.path().join("Character Pack");
    for mod_root in [&direct, &wrapper, &pack.join("Hutao"), &pack.join("Yelan")] {
        std::fs::create_dir_all(mod_root).unwrap();
        std::fs::write(
            mod_root.join("merged.ini"),
            "[TextureOverrideBody]\nhash = abc\n",
        )
        .unwrap();
    }

    let snapshot = super::mod_inbox::build_mod_inbox_snapshot(&context.pool, "gimi", root.path())
        .await
        .unwrap();
    let layouts = snapshot
        .ready_entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry.layout, entry.detected_root_count))
        .collect::<std::collections::BTreeSet<_>>();

    assert!(layouts.contains(&("Ayaka", super::types::ModInboxLayout::DirectMod, 1)));
    assert!(layouts.contains(&("Wrapped Raiden", super::types::ModInboxLayout::Wrapper, 1)));
    assert!(layouts.contains(&(
        "Character Pack",
        super::types::ModInboxLayout::FolderPack,
        2
    )));
}

#[tokio::test]
async fn mod_inbox_selected_batch_reserves_only_selected_entries() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    for name in ["Ayaka", "Raiden"] {
        let folder = root.path().join(name);
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(
            folder.join("merged.ini"),
            "[TextureOverrideBody]\nhash = abc\n",
        )
        .unwrap();
    }

    let batch = super::mod_inbox::create_mod_inbox_batch(
        &context.pool,
        "gimi",
        root.path(),
        &["Ayaka".to_string()],
    )
    .await
    .unwrap();
    assert_eq!(batch.items.len(), 1);
    assert!(batch.items[0].source_path.ends_with("Ayaka"));

    let snapshot = super::mod_inbox::build_mod_inbox_snapshot(&context.pool, "gimi", root.path())
        .await
        .unwrap();
    assert!(snapshot
        .ready_entries
        .iter()
        .find(|entry| entry.name == "Ayaka")
        .unwrap()
        .pending_batch_id
        .is_some());
    assert!(snapshot
        .ready_entries
        .iter()
        .find(|entry| entry.name == "Raiden")
        .unwrap()
        .pending_batch_id
        .is_none());
}

#[tokio::test]
async fn processed_mod_inbox_groups_pack_children_under_one_source_history() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("pack.zip");
    std::fs::write(&original, b"archive").unwrap();
    crate::modules::ingestion::adapters::sqlite::import_batch::create_batch(
        &context.pool,
        &crate::modules::ingestion::adapters::sqlite::import_batch::CreateImportBatchRecord {
            id: "batch-pack-history".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::ReadyToMove,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[
            crate::modules::ingestion::adapters::sqlite::import_batch::NewImportItemRecord {
                id: "source-pack-history".to_string(),
                source_kind: ImportSourceKind::ReadyToMove,
                source_path: original.to_string_lossy().into_owned(),
                staging_path: None,
                planned_name: "DISABLED pack".to_string(),
            },
        ],
    )
    .await
    .unwrap();
    crate::modules::ingestion::adapters::sqlite::import_batch::replace_archive_item_with_roots(
        &context.pool,
        "source-pack-history",
        &[
            crate::modules::ingestion::adapters::sqlite::import_batch::StagedRootRecord {
                id: "child-ayaka".to_string(),
                staging_path: "C:/Staging/Ayaka".to_string(),
                planned_name: "DISABLED Ayaka".to_string(),
            },
            crate::modules::ingestion::adapters::sqlite::import_batch::StagedRootRecord {
                id: "child-raiden".to_string(),
                staging_path: "C:/Staging/Raiden".to_string(),
                planned_name: "DISABLED Raiden".to_string(),
            },
        ],
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE import_jobs SET status = 'done',
         placed_path = CASE id
           WHEN 'source-pack-history' THEN 'C:/Mods/Ayaka/DISABLED Ayaka'
           ELSE 'C:/Mods/Raiden/DISABLED Raiden' END
         WHERE batch_id = 'batch-pack-history'",
    )
    .execute(&context.pool)
    .await
    .unwrap();
    crate::modules::ingestion::adapters::sqlite::import_batch::mark_mod_inbox_source_processed(
        &context.pool,
        "batch-pack-history",
        &original.to_string_lossy(),
        Some(&root.path().join("Processed/pack.zip").to_string_lossy()),
    )
    .await
    .unwrap();

    let snapshot = super::mod_inbox::build_mod_inbox_snapshot(&context.pool, "gimi", root.path())
        .await
        .unwrap();
    assert_eq!(snapshot.processed_sources.len(), 1);
    assert_eq!(snapshot.processed_sources[0].destinations.len(), 2);
    assert_eq!(
        snapshot.processed_sources[0].source_id,
        "source-pack-history"
    );
}

#[tokio::test]
async fn processed_delete_preflights_every_source_before_recycling_anything() {
    let context = init_test_db().await;
    insert_test_game(
        &context.pool,
        &TestGameFixture {
            id: "gimi",
            name: "Genshin",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let processed = root.path().join("Processed");
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir(&processed).unwrap();
    let valid = processed.join("valid.zip");
    let escaped = outside.path().join("escaped.zip");
    std::fs::write(&valid, b"valid").unwrap();
    std::fs::write(&escaped, b"escaped").unwrap();

    for (batch_id, item_id, retained) in [
        ("batch-valid", "source-valid", &valid),
        ("batch-escaped", "source-escaped", &escaped),
    ] {
        crate::modules::ingestion::adapters::sqlite::import_batch::create_batch(
            &context.pool,
            &crate::modules::ingestion::adapters::sqlite::import_batch::CreateImportBatchRecord {
                id: batch_id.to_string(),
                game_id: "gimi".to_string(),
                flow: ImportFlow::ReadyToMove,
                target_mode: TargetMode::Auto,
                target_object_id: None,
                target_subpath: None,
                source_archive_path: None,
            },
            &[
                crate::modules::ingestion::adapters::sqlite::import_batch::NewImportItemRecord {
                    id: item_id.to_string(),
                    source_kind: ImportSourceKind::ReadyToMove,
                    source_path: root
                        .path()
                        .join(format!("{item_id}.zip"))
                        .to_string_lossy()
                        .into(),
                    staging_path: Some(format!("C:/Staging/{item_id}")),
                    planned_name: format!("DISABLED {item_id}"),
                },
            ],
        )
        .await
        .unwrap();
        sqlx::query(
            "UPDATE import_jobs SET status = 'done', placed_path = 'C:/Mods/Object/mod',
             processed_source_path = ?, source_processed_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(retained.to_string_lossy().as_ref())
        .bind(item_id)
        .execute(&context.pool)
        .await
        .unwrap();
    }

    let error = super::mod_inbox::delete_processed_sources(
        &context.pool,
        "gimi",
        root.path(),
        &["source-valid".to_string(), "source-escaped".to_string()],
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        crate::shared::errors::AppError::Security(_)
    ));
    assert!(
        valid.exists(),
        "valid source must remain when any preflight fails"
    );
    assert!(escaped.exists());
}
