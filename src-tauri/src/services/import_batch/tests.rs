use super::coordinator::{
    create_import_batch, rename_import_item_plan, set_import_item_classification,
};
use super::types::{ConfidenceTier, ImportItemStatus};
use super::types::{
    CreateImportBatchInput, ImportFlow, ImportSourceInput, ImportSourceKind, RenameImportItemInput,
    SetImportItemClassificationInput, StableCategory, TargetMode,
};
use crate::test_utils::{init_test_db, insert_test_game, TestGameFixture};
use std::io::Write;

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
            game_type: crate::domain::models::GameType::GIMI,
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
                    crate::domain::models::GameType::GIMI
                } else {
                    crate::domain::models::GameType::SRMI
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
        crate::domain::errors::AppError::Security(_)
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
            game_type: crate::domain::models::GameType::GIMI,
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

    assert!(crate::repo::import_batch::transition_item_status(
        &context.pool,
        item_id,
        ImportItemStatus::Discovered,
        ImportItemStatus::Staged,
    )
    .await
    .unwrap());
    assert!(crate::repo::import_batch::transition_item_status(
        &context.pool,
        item_id,
        ImportItemStatus::Staged,
        ImportItemStatus::AwaitingCategory,
    )
    .await
    .unwrap());

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
            game_type: crate::domain::models::GameType::GIMI,
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
async fn failed_archive_staging_can_be_retried_after_the_source_is_repaired() {
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
async fn ready_to_move_nested_folder_pack_stages_one_item_per_detected_root() {
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
            game_type: crate::domain::models::GameType::GIMI,
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
            game_type: crate::domain::models::GameType::GIMI,
            path: "C:/Games/Genshin",
            mods_path: Some("C:/Games/Genshin/Mods"),
        },
    )
    .await
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("pack.zip");
    std::fs::write(&original, b"archive").unwrap();
    crate::repo::import_batch::create_batch(
        &context.pool,
        &crate::repo::import_batch::CreateImportBatchRecord {
            id: "batch-pack-history".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::ReadyToMove,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[crate::repo::import_batch::NewImportItemRecord {
            id: "source-pack-history".to_string(),
            source_kind: ImportSourceKind::ReadyToMove,
            source_path: original.to_string_lossy().into_owned(),
            staging_path: None,
            planned_name: "DISABLED pack".to_string(),
        }],
    )
    .await
    .unwrap();
    crate::repo::import_batch::replace_archive_item_with_roots(
        &context.pool,
        "source-pack-history",
        &[
            crate::repo::import_batch::StagedRootRecord {
                id: "child-ayaka".to_string(),
                staging_path: "C:/Staging/Ayaka".to_string(),
                planned_name: "DISABLED Ayaka".to_string(),
            },
            crate::repo::import_batch::StagedRootRecord {
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
    crate::repo::import_batch::mark_mod_inbox_source_processed(
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
            game_type: crate::domain::models::GameType::GIMI,
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
        crate::repo::import_batch::create_batch(
            &context.pool,
            &crate::repo::import_batch::CreateImportBatchRecord {
                id: batch_id.to_string(),
                game_id: "gimi".to_string(),
                flow: ImportFlow::ReadyToMove,
                target_mode: TargetMode::Auto,
                target_object_id: None,
                target_subpath: None,
                source_archive_path: None,
            },
            &[crate::repo::import_batch::NewImportItemRecord {
                id: item_id.to_string(),
                source_kind: ImportSourceKind::ReadyToMove,
                source_path: root
                    .path()
                    .join(format!("{item_id}.zip"))
                    .to_string_lossy()
                    .into(),
                staging_path: Some(format!("C:/Staging/{item_id}")),
                planned_name: format!("DISABLED {item_id}"),
            }],
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
        crate::domain::errors::AppError::Security(_)
    ));
    assert!(
        valid.exists(),
        "valid source must remain when any preflight fails"
    );
    assert!(escaped.exists());
}
