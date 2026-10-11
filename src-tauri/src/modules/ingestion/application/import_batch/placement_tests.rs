use super::{coordinator, payload_manifest, types::*};
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture,
};
use std::path::PathBuf;

struct Fixture {
    root: tempfile::TempDir,
    pool: sqlx::SqlitePool,
    item_id: String,
    source: PathBuf,
}

impl Fixture {
    fn mods_root(&self) -> PathBuf {
        self.root.path().join("Mods")
    }
    fn suggestion(&self, id: &str, folder: &str) -> DestinationSuggestion {
        DestinationSuggestion {
            kind: DestinationKind::ExistingObject,
            object_id: Some(id.to_string()),
            canonical_entry_key: Some("ayaka".to_string()),
            folder_name: id.to_string(),
            target_path: self.mods_root().join(folder).to_string_lossy().into_owned(),
            confidence_percentage: 96,
            confidence_tier: ConfidenceTier::High,
            match_method: DestinationMatchMethod::CanonicalIdentity,
            warning: None,
        }
    }
    async fn suggestions(&self) {
        let suggestions = [
            self.suggestion("a", "Characters/A"),
            self.suggestion("b", "Characters/B"),
        ];
        super::super::super::adapters::sqlite::import_batch::store_match_suggestions(
            &self.pool,
            &self.item_id,
            &[],
            &suggestions,
            &[],
            &ReviewGate::default(),
        )
        .await
        .unwrap();
    }
    fn decision(&self, id: &str) -> SetImportItemDecisionInput {
        SetImportItemDecisionInput {
            item_id: self.item_id.clone(),
            decision: ImportDecision::Confirm,
            destination_object_id: Some(id.to_string()),
            canonical_entry_key: Some("ayaka".to_string()),
            destination_path: None,
            matched_alias: None,
        }
    }
}

async fn fixture(subpath: Option<&str>) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let pool = init_test_db().await.pool;
    let mods = root.path().join("Mods");
    for folder in ["Characters/A", "Characters/B"] {
        std::fs::create_dir_all(mods.join(folder)).unwrap();
    }
    let source = root.path().join("Downloads/Skin");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("merged.ini"),
        b"[TextureOverrideSource]\nhash=abc\n",
    )
    .unwrap();
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "game",
            name: "Game",
            game_type: crate::modules::games::domain::models::GameType::Gimi,
            path: root.path().to_str().unwrap(),
            mods_path: Some(mods.to_str().unwrap()),
        },
    )
    .await
    .unwrap();
    for (id, folder) in [("a", "Characters/A"), ("b", "Characters/B")] {
        insert_test_object(
            &pool,
            &TestObjectFixture {
                id,
                game_id: "game",
                name: id,
                folder_path: folder,
                object_type: "Character",
            },
        )
        .await
        .unwrap();
    }
    let batch = coordinator::create_import_batch(
        &pool,
        CreateImportBatchInput {
            game_id: "game".to_string(),
            flow: ImportFlow::AutoImport,
            target_mode: TargetMode::Auto,
            target_object_id: None,
            target_subpath: subpath.map(str::to_string),
            sources: vec![ImportSourceInput {
                path: source.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::Folder),
            }],
        },
    )
    .await
    .unwrap();
    let item_id = batch.items[0].id.clone();
    sqlx::query("UPDATE import_jobs SET status = 'awaiting_destination', match_category = 'Character' WHERE id = ?")
        .bind(&item_id).execute(&pool).await.unwrap();
    let manifest = payload_manifest::build_payload_manifest(&source, None).unwrap();
    crate::modules::ingestion::adapters::sqlite::import_batch::store_payload_manifest(
        &pool, &item_id, &manifest,
    )
    .await
    .unwrap();
    let fixture = Fixture {
        root,
        pool,
        item_id,
        source,
    };
    fixture.suggestions().await;
    fixture
}

#[tokio::test]
async fn exact_object_id_wins_over_a_shared_canonical_key() {
    let fixture = fixture(None).await;
    let chosen = coordinator::set_import_item_decision(&fixture.pool, fixture.decision("b"))
        .await
        .unwrap();
    assert_eq!(chosen.destination_object_id.as_deref(), Some("b"));
    assert_eq!(
        PathBuf::from(chosen.destination_path.unwrap()),
        fixture.mods_root().join("Characters/B")
    );
}

#[tokio::test]
async fn existing_object_selection_rejects_a_conflicting_canonical_key() {
    let fixture = fixture(None).await;
    let mut input = fixture.decision("b");
    input.canonical_entry_key = Some("different-entry".to_string());
    assert!(coordinator::set_import_item_decision(&fixture.pool, input)
        .await
        .is_err());
}

#[tokio::test]
async fn create_canonical_selection_cannot_include_an_existing_object_id() {
    let fixture = fixture(None).await;
    let mut input = fixture.decision("b");
    input.decision = ImportDecision::CreateCanonical;
    assert!(coordinator::set_import_item_decision(&fixture.pool, input)
        .await
        .is_err());
}

#[tokio::test]
async fn comparison_uses_the_full_destination_path_and_batch_subpath() {
    let fixture = fixture(Some("Skins")).await;
    let target = fixture.mods_root().join("Characters/B/Skins/DISABLED Skin");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(
        target.join("merged.ini"),
        b"[TextureOverrideDifferent]\nhash=def\n",
    )
    .unwrap();
    let chosen = coordinator::set_import_item_decision(&fixture.pool, fixture.decision("b"))
        .await
        .unwrap();
    let comparison = chosen
        .target_comparison
        .expect("same-name conflict in selected subfolder");
    assert_eq!(chosen.status, ImportItemStatus::AwaitingDestination);
    assert_eq!(
        comparison.outcome,
        TargetComparisonOutcome::SameNameDifferentContent
    );
    assert_eq!(PathBuf::from(comparison.target_path), target);
    assert!(fixture.source.exists());
}

#[tokio::test]
async fn decision_resolves_current_object_path_instead_of_stored_suggestion_path() {
    let fixture = fixture(None).await;
    std::fs::create_dir_all(fixture.mods_root().join("Characters/Moved")).unwrap();
    sqlx::query("UPDATE objects SET folder_path = 'Characters/Moved' WHERE id = 'b'")
        .execute(&fixture.pool)
        .await
        .unwrap();
    let chosen = coordinator::set_import_item_decision(&fixture.pool, fixture.decision("b"))
        .await
        .unwrap();
    assert_eq!(
        PathBuf::from(chosen.destination_path.unwrap()),
        fixture.mods_root().join("Characters/Moved")
    );
}

#[tokio::test]
async fn batch_creation_rejects_automatic_mode_with_an_object_id() {
    let fixture = fixture(None).await;
    let input = CreateImportBatchInput {
        game_id: "game".to_string(),
        flow: ImportFlow::AutoImport,
        target_mode: TargetMode::Auto,
        target_object_id: Some("a".to_string()),
        target_subpath: None,
        sources: vec![ImportSourceInput {
            path: fixture.source.to_string_lossy().into_owned(),
            source_kind: Some(ImportSourceKind::Folder),
        }],
    };
    assert!(coordinator::create_import_batch(&fixture.pool, input)
        .await
        .is_err());
}

#[tokio::test]
async fn specific_batch_requires_a_target_owned_by_its_game() {
    let fixture = fixture(None).await;
    insert_test_game(
        &fixture.pool,
        &TestGameFixture {
            id: "foreign-game",
            name: "Foreign",
            game_type: crate::modules::games::domain::models::GameType::Zzmi,
            path: fixture.root.path().to_str().unwrap(),
            mods_path: Some("C:/Foreign/Mods"),
        },
    )
    .await
    .unwrap();
    insert_test_object(
        &fixture.pool,
        &TestObjectFixture {
            id: "foreign-object",
            game_id: "foreign-game",
            name: "Foreign",
            folder_path: "Foreign",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    for target in ["foreign-object", "missing-object", " "] {
        let input = CreateImportBatchInput {
            game_id: "game".to_string(),
            flow: ImportFlow::SpecificImport,
            target_mode: TargetMode::Specific,
            target_object_id: Some(target.to_string()),
            target_subpath: None,
            sources: vec![ImportSourceInput {
                path: fixture.source.to_string_lossy().into_owned(),
                source_kind: Some(ImportSourceKind::Folder),
            }],
        };
        assert!(
            coordinator::create_import_batch(&fixture.pool, input)
                .await
                .is_err(),
            "{target}"
        );
    }
}
