use super::{
    begin_batch_commit, create_batch, get_batch, mark_batch_review_started,
    recover_interrupted_batch_states, rename_planned_item, restore_item_after_rollback,
    store_decision, store_match_suggestions, transition_item_status,
    CreateImportBatchRecord, NewImportItemRecord,
};
use crate::modules::ingestion::application::import_batch::types::{
    CanonicalSuggestion, ConfidenceTier, DestinationKind, DestinationMatchMethod,
    DestinationSuggestion, ImportDecision, ImportFlow, ImportItemStatus, ImportSourceKind,
    SetImportItemDecisionInput, TargetMode,
};
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture,
};

type ClearedPlanFields = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

#[tokio::test]
async fn batch_and_items_round_trip_without_frontend_owned_scoring() {
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

    create_batch(
        &context.pool,
        &CreateImportBatchRecord {
            id: "batch-1".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[NewImportItemRecord {
            id: "item-1".to_string(),
            source_kind: ImportSourceKind::Folder,
            source_path: "C:/Downloads/DISABLED ayaka-12319mods".to_string(),
            staging_path: None,
            planned_name: "DISABLED ayaka-12319mods".to_string(),
        }],
    )
    .await
    .unwrap();

    let batch = get_batch(&context.pool, "batch-1").await.unwrap().unwrap();
    assert_eq!(batch.flow, ImportFlow::AutoImport);
    assert_eq!(batch.items.len(), 1);
    assert_eq!(batch.items[0].status, ImportItemStatus::Discovered);
    assert!(batch.items[0].category_suggestions.is_empty());
    assert!(batch.items[0].destination_suggestions.is_empty());
}

#[tokio::test]
async fn stores_only_scored_destinations_and_uses_the_default_destination_score() {
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
    create_batch(
        &context.pool,
        &CreateImportBatchRecord {
            id: "batch-scores".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[NewImportItemRecord {
            id: "item-scores".to_string(),
            source_kind: ImportSourceKind::Folder,
            source_path: "C:/Downloads/Ayaka".to_string(),
            staging_path: None,
            planned_name: "Ayaka".to_string(),
        }],
    )
    .await
    .unwrap();
    sqlx::query("UPDATE import_jobs SET status = 'awaiting_destination' WHERE id = 'item-scores'")
        .execute(&context.pool)
        .await
        .unwrap();

    let canonical = [CanonicalSuggestion {
        entry_key: "ayaka".to_string(),
        name: "Ayaka".to_string(),
        matched_alias: None,
        confidence_percentage: 92,
        confidence_tier: ConfidenceTier::High,
        evidence: Vec::new(),
    }];
    let destinations = [
        DestinationSuggestion {
            kind: DestinationKind::ExistingObject,
            object_id: Some("object-ayaka".to_string()),
            canonical_entry_key: Some("ayaka".to_string()),
            folder_name: "Ayaka".to_string(),
            target_path: "C:/Games/Genshin/Mods/Ayaka".to_string(),
            confidence_percentage: 75,
            confidence_tier: ConfidenceTier::High,
            match_method: DestinationMatchMethod::NameSubstring,
            warning: None,
        },
        DestinationSuggestion {
            kind: DestinationKind::ExistingObject,
            object_id: Some("object-zero".to_string()),
            canonical_entry_key: None,
            folder_name: "Unrelated".to_string(),
            target_path: "C:/Games/Genshin/Mods/Unrelated".to_string(),
            confidence_percentage: 0,
            confidence_tier: ConfidenceTier::NoMatch,
            match_method: DestinationMatchMethod::NoNameMatch,
            warning: None,
        },
    ];
    assert!(store_match_suggestions(
        &context.pool,
        "item-scores",
        &canonical,
        &destinations,
        &[],
    )
    .await
    .unwrap());

    let batch = get_batch(&context.pool, "batch-scores").await.unwrap().unwrap();
    let item = &batch.items[0];
    assert_eq!(item.confidence_percentage, 75);
    assert_eq!(item.confidence_tier, ConfidenceTier::High);
    assert_eq!(item.destination_suggestions.len(), 1);
    assert_eq!(item.destination_suggestions[0].object_id.as_deref(), Some("object-ayaka"));
}

#[tokio::test]
async fn batch_commit_requires_a_durable_review_marker() {
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
    create_batch(
        &context.pool,
        &CreateImportBatchRecord {
            id: "batch-review".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[NewImportItemRecord {
            id: "item-review".to_string(),
            source_kind: ImportSourceKind::Folder,
            source_path: "C:/Downloads/Ayaka".to_string(),
            staging_path: None,
            planned_name: "Ayaka".to_string(),
        }],
    )
    .await
    .unwrap();
    sqlx::query("UPDATE import_batches SET status = 'awaiting_review' WHERE id = 'batch-review'")
        .execute(&context.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE import_jobs SET status = 'ready' WHERE id = 'item-review'")
        .execute(&context.pool)
        .await
        .unwrap();

    let item_ids = vec!["item-review".to_string()];
    assert!(!begin_batch_commit(&context.pool, "batch-review", &item_ids)
        .await
        .unwrap());
    assert!(mark_batch_review_started(&context.pool, "batch-review")
        .await
        .unwrap());
    assert!(begin_batch_commit(&context.pool, "batch-review", &item_ids)
        .await
        .unwrap());
}

#[tokio::test]
async fn transition_is_compare_and_swap_and_rejects_stale_state() {
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
    create_batch(
        &context.pool,
        &CreateImportBatchRecord {
            id: "batch-1".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::ReadyToMove,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[NewImportItemRecord {
            id: "item-1".to_string(),
            source_kind: ImportSourceKind::ReadyToMove,
            source_path: "C:/Downloads/mods/Genshin/Ayaka".to_string(),
            staging_path: None,
            planned_name: "Ayaka".to_string(),
        }],
    )
    .await
    .unwrap();

    assert!(transition_item_status(
        &context.pool,
        "item-1",
        ImportItemStatus::Discovered,
        ImportItemStatus::Staged,
    )
    .await
    .unwrap());
    assert!(!transition_item_status(
        &context.pool,
        "item-1",
        ImportItemStatus::Discovered,
        ImportItemStatus::Staged,
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn replacing_a_decision_clears_stale_canonical_identity() {
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
    insert_test_object(
        &context.pool,
        &TestObjectFixture {
            id: "object-ayaka",
            game_id: "gimi",
            name: "Ayaka",
            folder_path: "Ayaka",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    create_batch(
        &context.pool,
        &CreateImportBatchRecord {
            id: "batch-decision".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[NewImportItemRecord {
            id: "item-decision".to_string(),
            source_kind: ImportSourceKind::Folder,
            source_path: "C:/Downloads/Ayaka".to_string(),
            staging_path: None,
            planned_name: "Ayaka".to_string(),
        }],
    )
    .await
    .unwrap();
    sqlx::query("UPDATE import_jobs SET status = 'awaiting_destination' WHERE id = ?")
        .bind("item-decision")
        .execute(&context.pool)
        .await
        .unwrap();

    assert!(store_decision(
        &context.pool,
        &SetImportItemDecisionInput {
            item_id: "item-decision".to_string(),
            decision: ImportDecision::CreateCanonical,
            destination_object_id: None,
            destination_path: Some("C:/Games/Genshin/Mods/Ayaka".to_string()),
            canonical_entry_key: Some("ayaka".to_string()),
            matched_alias: Some("Kamisato Ayaka".to_string()),
        },
        75,
        ConfidenceTier::High,
    )
    .await
    .unwrap());
    assert!(store_decision(
        &context.pool,
        &SetImportItemDecisionInput {
            item_id: "item-decision".to_string(),
            decision: ImportDecision::Skip,
            destination_object_id: None,
            destination_path: None,
            canonical_entry_key: None,
            matched_alias: None,
        },
        0,
        ConfidenceTier::NoMatch,
    )
    .await
    .unwrap());

    let stored: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT match_entry_key, match_alias_name FROM import_jobs WHERE id = ?")
            .bind("item-decision")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    assert_eq!(stored, (None, None));

    sqlx::query(
        "UPDATE import_jobs SET match_entry_key = 'ayaka', match_alias_name = 'Ayaka',
         destination_object_id = 'object-ayaka', match_object_id = 'object-ayaka',
         destination_path = 'C:/Mods/Ayaka', placed_path = 'C:/Mods/Ayaka/old-name'
         WHERE id = ?",
    )
    .bind("item-decision")
    .execute(&context.pool)
    .await
    .unwrap();
    assert!(
        rename_planned_item(&context.pool, "item-decision", "new-name")
            .await
            .unwrap()
    );
    let renamed: ClearedPlanFields = sqlx::query_as(
        "SELECT match_entry_key, match_alias_name, destination_object_id, match_object_id,
                destination_path, placed_path FROM import_jobs WHERE id = ?",
    )
    .bind("item-decision")
    .fetch_one(&context.pool)
    .await
    .unwrap();
    assert_eq!(renamed, (None, None, None, None, None, None));
}

#[tokio::test]
async fn refreshing_suggestions_invalidates_a_ready_decision() {
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
    insert_test_object(
        &context.pool,
        &TestObjectFixture {
            id: "old-object",
            game_id: "gimi",
            name: "Old",
            folder_path: "Old",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    create_batch(
        &context.pool,
        &CreateImportBatchRecord {
            id: "batch-refresh".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[NewImportItemRecord {
            id: "item-refresh".to_string(),
            source_kind: ImportSourceKind::Folder,
            source_path: "C:/Downloads/Ayaka".to_string(),
            staging_path: None,
            planned_name: "DISABLED Ayaka".to_string(),
        }],
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE import_jobs SET status = 'ready', decision = 'existing_target',
         destination_object_id = 'old-object', destination_path = 'C:/Mods/Old',
         match_entry_key = 'old-entry', match_alias_name = 'Old Alias',
         match_object_id = 'old-object', placed_path = 'C:/Mods/Old/DISABLED Ayaka',
         result = 'planned' WHERE id = 'item-refresh'",
    )
    .execute(&context.pool)
    .await
    .unwrap();

    assert!(
        store_match_suggestions(&context.pool, "item-refresh", &[], &[], &[])
            .await
            .unwrap()
    );
    let refreshed = get_batch(&context.pool, "batch-refresh")
        .await
        .unwrap()
        .unwrap();
    let item = &refreshed.items[0];
    assert_eq!(item.status, ImportItemStatus::AwaitingDestination);
    assert_eq!(item.decision, ImportDecision::Pending);
    assert!(item.destination_object_id.is_none());
    assert!(item.destination_path.is_none());
    assert!(item.selected_entry_key.is_none());
    let placed_path: Option<String> =
        sqlx::query_scalar("SELECT placed_path FROM import_jobs WHERE id = 'item-refresh'")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    assert!(placed_path.is_none());
}

#[tokio::test]
async fn successful_filesystem_rollback_restores_a_commit_item_to_ready() {
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
    create_batch(
        &context.pool,
        &CreateImportBatchRecord {
            id: "batch-rollback".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[NewImportItemRecord {
            id: "item-rollback".to_string(),
            source_kind: ImportSourceKind::Folder,
            source_path: "C:/Downloads/Ayaka".to_string(),
            staging_path: None,
            planned_name: "DISABLED Ayaka".to_string(),
        }],
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE import_jobs SET status = 'committing', decision = 'confirm',
         destination_path = 'C:/Mods/Ayaka', placed_path = 'C:/Mods/Ayaka/DISABLED Ayaka'
         WHERE id = 'item-rollback'",
    )
    .execute(&context.pool)
    .await
    .unwrap();

    restore_item_after_rollback(&context.pool, "item-rollback", "injected move failure")
        .await
        .unwrap();
    let item = get_batch(&context.pool, "batch-rollback")
        .await
        .unwrap()
        .unwrap()
        .items
        .remove(0);
    assert_eq!(item.status, ImportItemStatus::Ready);
    assert_eq!(item.destination_path.as_deref(), Some("C:/Mods/Ayaka"));
    assert_eq!(item.result.as_deref(), Some("rolled_back"));
    let placed_path: Option<String> =
        sqlx::query_scalar("SELECT placed_path FROM import_jobs WHERE id = 'item-rollback'")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    assert!(placed_path.is_none());
}

#[tokio::test]
async fn interrupted_metadata_finalization_becomes_resumable_on_startup() {
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
    create_batch(
        &context.pool,
        &CreateImportBatchRecord {
            id: "batch-recovery".to_string(),
            game_id: "gimi".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: None,
            source_archive_path: None,
        },
        &[NewImportItemRecord {
            id: "item-recovery".to_string(),
            source_kind: ImportSourceKind::Folder,
            source_path: "C:/Downloads/Ayaka".to_string(),
            staging_path: None,
            planned_name: "DISABLED Ayaka".to_string(),
        }],
    )
    .await
    .unwrap();
    sqlx::raw_sql(
        "UPDATE import_batches SET status = 'committing' WHERE id = 'batch-recovery';
         UPDATE import_jobs SET status = 'finalizing_metadata' WHERE id = 'item-recovery';",
    )
    .execute(&context.pool)
    .await
    .unwrap();

    assert!(
        recover_interrupted_batch_states(&context.pool)
            .await
            .unwrap()
            >= 1
    );
    let recovered = get_batch(&context.pool, "batch-recovery")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        recovered.status,
        crate::modules::ingestion::application::import_batch::types::ImportBatchStatus::Partial
    );
    assert_eq!(recovered.items[0].status, ImportItemStatus::MetadataPending);
}

#[tokio::test]
async fn startup_recovers_analysis_completed_commits_and_pending_ready_to_move_archives() {
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
    for (batch_id, item_id, flow, staging_path) in [
        (
            "batch-analyzing",
            "item-analyzing",
            ImportFlow::AutoImport,
            None,
        ),
        (
            "batch-complete",
            "item-complete",
            ImportFlow::AutoImport,
            None,
        ),
        (
            "batch-archive",
            "item-archive",
            ImportFlow::ReadyToMove,
            Some("C:/App/import-staging/batch-archive/item-archive/extracted".to_string()),
        ),
        (
            "batch-direct-inbox",
            "item-direct-inbox",
            ImportFlow::ReadyToMove,
            None,
        ),
    ] {
        create_batch(
            &context.pool,
            &CreateImportBatchRecord {
                id: batch_id.to_string(),
                game_id: "gimi".to_string(),
                flow,
                target_mode: TargetMode::Auto,
                target_object_id: None,
                target_subpath: None,
                source_archive_path: None,
            },
            &[NewImportItemRecord {
                id: item_id.to_string(),
                source_kind: if flow == ImportFlow::ReadyToMove {
                    ImportSourceKind::ReadyToMove
                } else {
                    ImportSourceKind::Folder
                },
                source_path: format!("C:/Downloads/{item_id}.zip"),
                staging_path,
                planned_name: "DISABLED Ayaka".to_string(),
            }],
        )
        .await
        .unwrap();
    }
    sqlx::raw_sql(
        "UPDATE import_batches SET status = 'analyzing' WHERE id = 'batch-analyzing';
         UPDATE import_batches SET status = 'committing'
             WHERE id IN ('batch-complete', 'batch-archive', 'batch-direct-inbox');
         UPDATE import_jobs SET status = 'done'
             WHERE id IN ('item-complete', 'item-archive', 'item-direct-inbox');",
    )
    .execute(&context.pool)
    .await
    .unwrap();

    assert!(
        recover_interrupted_batch_states(&context.pool)
            .await
            .unwrap()
            >= 3
    );
    let analyzing = get_batch(&context.pool, "batch-analyzing")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        analyzing.status,
        crate::modules::ingestion::application::import_batch::types::ImportBatchStatus::Partial
    );
    let complete = get_batch(&context.pool, "batch-complete")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        complete.status,
        crate::modules::ingestion::application::import_batch::types::ImportBatchStatus::Done
    );
    let archive = get_batch(&context.pool, "batch-archive")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        archive.status,
        crate::modules::ingestion::application::import_batch::types::ImportBatchStatus::Partial
    );
    assert_eq!(archive.items[0].status, ImportItemStatus::Partial);
    assert_eq!(archive.items[0].result.as_deref(), Some("archive_pending"));
    let direct = get_batch(&context.pool, "batch-direct-inbox")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        direct.status,
        crate::modules::ingestion::application::import_batch::types::ImportBatchStatus::Partial
    );
    assert_eq!(direct.items[0].status, ImportItemStatus::Partial);
    assert_eq!(direct.items[0].result.as_deref(), Some("archive_pending"));
}
