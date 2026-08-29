use crate::services::objects::classification::{
    apply_object_classification, CanonicalClassificationMatch, ObjectClassificationInput,
};

async fn setup_classification_fixture() -> sqlx::SqlitePool {
    let pool = crate::test_utils::init_test_db().await.pool;
    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Game",
            game_type: crate::domain::models::GameType::GIMI,
            path: "/",
            mods_path: Some("/Mods"),
        },
    )
    .await
    .expect("game seed");
    crate::test_utils::insert_test_object(
        &pool,
        &crate::test_utils::TestObjectFixture {
            id: "o1",
            game_id: "g1",
            name: "Raiden",
            folder_path: "Raiden",
            object_type: "Character",
        },
    )
    .await
    .expect("object seed");
    crate::test_utils::insert_test_mod(
        &pool,
        &crate::test_utils::TestModFixture {
            id: "m1",
            game_id: "g1",
            object_id: Some("o1"),
            actual_name: "DISABLED raiden32114",
            folder_path: "Raiden/DISABLED raiden32114",
            status: crate::domain::models::ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("/Mods"),
        },
    )
    .await
    .expect("mod seed");
    pool
}

fn classification_input(alias: Option<&str>) -> ObjectClassificationInput {
    ObjectClassificationInput {
        game_id: "g1".to_string(),
        object_id: "o1".to_string(),
        category: "Weapon".to_string(),
        subcategory: Some("Light Cone".to_string()),
        metadata: serde_json::json!({"path": "Nihility", "rarity": 5}),
        canonical_match: Some(CanonicalClassificationMatch {
            entry_key: "raiden-shogun".to_string(),
            alias_name: Some("Raiden Shogun".to_string()),
            confidence: Some(0.91),
            reason: Some("folder name".to_string()),
            source: "match_wizard".to_string(),
        }),
        confirmed_source_alias: alias.map(str::to_string),
    }
}

#[tokio::test]
async fn classification_writer_commits_metadata_children_projection_and_user_alias_together() {
    let pool = setup_classification_fixture().await;
    sqlx::query(
        r#"UPDATE objects SET custom_skins = '[{"name":"Bundled","aliases":["Shogun"],"thumbnail_skin_path":null,"rarity":null},{"name":"User","aliases":["Old Alias"],"thumbnail_skin_path":null,"rarity":null}]' WHERE id = 'o1'"#,
    )
    .execute(&pool)
    .await
    .expect("alias seed");

    let result = apply_object_classification(&pool, classification_input(Some("raiden32114")))
        .await
        .expect("classification write");

    assert_eq!(result.object_id, "o1");
    assert_eq!(result.child_mods_updated, 1);
    assert!(result.aliases_changed);

    let object: (String, Option<String>, String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT object_type, sub_category, metadata, matched_entry_key, matched_alias_name FROM objects WHERE id = 'o1'",
    )
    .fetch_one(&pool)
    .await
    .expect("classified object");
    assert_eq!(object.0, "Weapon");
    assert_eq!(object.1.as_deref(), Some("Light Cone"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&object.2).expect("metadata json"),
        serde_json::json!({"path": "Nihility", "rarity": 5})
    );
    assert_eq!(object.3.as_deref(), Some("raiden-shogun"));
    assert_eq!(object.4.as_deref(), Some("Raiden Shogun"));

    let mod_type: String = sqlx::query_scalar("SELECT object_type FROM mods WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .expect("child category");
    assert_eq!(mod_type, "Weapon");

    let projection_type: String = sqlx::query_scalar(
        "SELECT object_type FROM object_runtime_projection WHERE game_id = 'g1' AND object_id = 'o1'",
    )
    .fetch_one(&pool)
    .await
    .expect("runtime projection");
    assert_eq!(projection_type, "Weapon");

    let aliases_json: String =
        sqlx::query_scalar("SELECT custom_skins FROM objects WHERE id = 'o1'")
            .fetch_one(&pool)
            .await
            .expect("custom skins");
    let skins: Vec<crate::services::scanner::deep_matcher::CustomSkin> =
        serde_json::from_str(&aliases_json).expect("custom skins array");
    let user = skins
        .iter()
        .find(|skin| skin.name == "User")
        .expect("User skin group");
    assert_eq!(user.aliases, vec!["Old Alias", "raiden32114"]);
}

#[tokio::test]
async fn classification_writer_is_idempotent() {
    let pool = setup_classification_fixture().await;

    let first = apply_object_classification(&pool, classification_input(Some("RAIDEN32114")))
        .await
        .expect("first classification");
    let second = apply_object_classification(&pool, classification_input(Some("raiden32114")))
        .await
        .expect("second classification");

    assert!(first.aliases_changed);
    assert!(!second.aliases_changed);
    let aliases_json: String =
        sqlx::query_scalar("SELECT custom_skins FROM objects WHERE id = 'o1'")
            .fetch_one(&pool)
            .await
            .expect("custom skins");
    let skins: Vec<crate::services::scanner::deep_matcher::CustomSkin> =
        serde_json::from_str(&aliases_json).expect("normalized custom skins array");
    let learned: Vec<&String> = skins
        .iter()
        .flat_map(|skin| skin.aliases.iter())
        .filter(|alias| alias.eq_ignore_ascii_case("raiden32114"))
        .collect();
    assert_eq!(learned.len(), 1);
}

#[tokio::test]
async fn classification_writer_rejects_unstable_category_without_writes() {
    let pool = setup_classification_fixture().await;
    let mut input = classification_input(None);
    input.category = "Light Cone".to_string();

    let error = apply_object_classification(&pool, input)
        .await
        .expect_err("legacy category must be rejected");

    assert!(error
        .to_string()
        .contains("Character, Weapon, UI, or Other"));
    let object_type: String = sqlx::query_scalar("SELECT object_type FROM objects WHERE id = 'o1'")
        .fetch_one(&pool)
        .await
        .expect("unchanged category");
    assert_eq!(object_type, "Character");
}

#[tokio::test]
async fn classification_writer_rolls_back_every_field_when_projection_refresh_fails() {
    let pool = setup_classification_fixture().await;
    sqlx::query(
        "CREATE TRIGGER fail_classification_projection BEFORE INSERT ON object_runtime_projection BEGIN SELECT RAISE(ABORT, 'injected projection failure'); END",
    )
    .execute(&pool)
    .await
    .expect("projection failure trigger");

    let result =
        apply_object_classification(&pool, classification_input(Some("raiden32114"))).await;

    assert!(result.is_err());
    let object: (String, Option<String>, String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT object_type, sub_category, metadata, matched_entry_key, custom_skins FROM objects WHERE id = 'o1'",
    )
    .fetch_one(&pool)
    .await
    .expect("rolled-back object");
    assert_eq!(object.0, "Character");
    assert_eq!(object.1, None);
    assert_eq!(object.2, "{}");
    assert_eq!(object.3, None);
    assert_eq!(object.4, None);
    let mod_type: String = sqlx::query_scalar("SELECT object_type FROM mods WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .expect("rolled-back mod");
    assert_eq!(mod_type, "Character");
}

#[tokio::test]
async fn classification_writer_requires_canonical_match_before_learning_alias() {
    let pool = setup_classification_fixture().await;
    let mut input = classification_input(Some("raiden32114"));
    input.canonical_match = None;

    let error = apply_object_classification(&pool, input)
        .await
        .expect_err("unbound alias must be rejected");

    assert!(error.to_string().contains("canonical match"));
}

#[tokio::test]
async fn classification_batch_preflights_every_item_before_writing_any_item() {
    use crate::services::import_batch::types::StableCategory;
    use crate::services::match_engine::inspection::{inspect_source, InspectionRequest};
    use crate::services::objects::classification_batch::{
        apply_object_classification_batch, ApplyObjectClassificationBatchInput,
        ApplyObjectClassificationItem,
    };

    let context = crate::test_utils::init_test_db().await;
    let workspace = tempfile::tempdir().unwrap();
    let mods_root = workspace.path().join("Mods");
    for name in ["Ayaka", "Raiden"] {
        let source = mods_root.join(name);
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("merged.ini"), format!("[{name}]\n")).unwrap();
    }
    crate::test_utils::insert_test_game(
        &context.pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Game",
            game_type: crate::domain::models::GameType::GIMI,
            path: mods_root.to_str().unwrap(),
            mods_path: Some(mods_root.to_str().unwrap()),
        },
    )
    .await
    .unwrap();
    for (id, name) in [("o1", "Ayaka"), ("o2", "Raiden")] {
        crate::test_utils::insert_test_object(
            &context.pool,
            &crate::test_utils::TestObjectFixture {
                id,
                game_id: "g1",
                name,
                folder_path: name,
                object_type: "Character",
            },
        )
        .await
        .unwrap();
    }
    let fingerprints = ["Ayaka", "Raiden"].map(|name| {
        inspect_source(&InspectionRequest {
            source_path: mods_root.join(name),
            planned_name: Some(name.to_string()),
            match_extensions: vec!["ini".to_string()],
        })
        .unwrap()
        .fingerprint
    });
    std::fs::write(mods_root.join("Raiden/changed.ini"), "[changed]\n").unwrap();

    let error = apply_object_classification_batch(
        &context.pool,
        ApplyObjectClassificationBatchInput {
            game_id: "g1".to_string(),
            disable_after_apply: false,
            items: [
                ("o1", fingerprints[0].clone()),
                ("o2", fingerprints[1].clone()),
            ]
            .into_iter()
            .map(|(object_id, fingerprint)| ApplyObjectClassificationItem {
                object_id: object_id.to_string(),
                category: StableCategory::Weapon,
                sub_category: None,
                metadata: serde_json::json!({}),
                canonical_entry_key: None,
                canonical_alias: None,
                confidence_percentage: None,
                fingerprint,
            })
            .collect(),
        },
        &crate::services::scanner::deep_matcher::MasterDb::new(Vec::new()),
        &["ini".to_string()],
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("stale_preview"));
    let categories: Vec<String> = sqlx::query_scalar("SELECT object_type FROM objects ORDER BY id")
        .fetch_all(&context.pool)
        .await
        .unwrap();
    assert_eq!(categories, ["Character", "Character"]);
}

#[tokio::test]
async fn classification_batch_revalidates_canonical_identity_against_master_db() {
    use crate::services::import_batch::types::StableCategory;
    use crate::services::match_engine::inspection::{inspect_source, InspectionRequest};
    use crate::services::objects::classification_batch::{
        apply_object_classification_batch, ApplyObjectClassificationBatchInput,
        ApplyObjectClassificationItem,
    };
    use crate::services::scanner::deep_matcher::{DbEntry, MasterDb};

    let context = crate::test_utils::init_test_db().await;
    let workspace = tempfile::tempdir().unwrap();
    let mods_root = workspace.path().join("Mods");
    let source = mods_root.join("Ayaka");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("merged.ini"), "[Ayaka]\n").unwrap();
    crate::test_utils::insert_test_game(
        &context.pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Game",
            game_type: crate::domain::models::GameType::GIMI,
            path: mods_root.to_str().unwrap(),
            mods_path: Some(mods_root.to_str().unwrap()),
        },
    )
    .await
    .unwrap();
    crate::test_utils::insert_test_object(
        &context.pool,
        &crate::test_utils::TestObjectFixture {
            id: "o1",
            game_id: "g1",
            name: "Ayaka",
            folder_path: "Ayaka",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    let fingerprint = inspect_source(&InspectionRequest {
        source_path: source,
        planned_name: Some("Ayaka".to_string()),
        match_extensions: vec!["ini".to_string()],
    })
    .unwrap()
    .fingerprint;
    let entries: Vec<DbEntry> = serde_json::from_value(serde_json::json!([
        {"name": "Ayaka", "object_type": "Character", "entry_kind": "canonical"},
        {"name": "Weapon Taxonomy", "object_type": "Weapon", "entry_kind": "taxonomy"}
    ]))
    .unwrap();
    let master_db = MasterDb::new(entries);

    for entry_key in ["missing-entry", "weapon-taxonomy", "ayaka"] {
        let error = apply_object_classification_batch(
            &context.pool,
            ApplyObjectClassificationBatchInput {
                game_id: "g1".to_string(),
                disable_after_apply: false,
                items: vec![ApplyObjectClassificationItem {
                    object_id: "o1".to_string(),
                    category: StableCategory::Weapon,
                    sub_category: None,
                    metadata: serde_json::json!({}),
                    canonical_entry_key: Some(entry_key.to_string()),
                    canonical_alias: None,
                    confidence_percentage: Some(90),
                    fingerprint: fingerprint.clone(),
                }],
            },
            &master_db,
            &["ini".to_string()],
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            crate::domain::errors::AppError::Validation(_)
        ));
    }
    let persisted: (String, Option<String>) =
        sqlx::query_as("SELECT object_type, matched_entry_key FROM objects WHERE id = 'o1'")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    assert_eq!(persisted, ("Character".to_string(), None));

    let valid = apply_object_classification_batch(
        &context.pool,
        ApplyObjectClassificationBatchInput {
            game_id: "g1".to_string(),
            disable_after_apply: false,
            items: vec![ApplyObjectClassificationItem {
                object_id: "o1".to_string(),
                category: StableCategory::Character,
                sub_category: None,
                metadata: serde_json::json!({}),
                canonical_entry_key: Some("ayaka".to_string()),
                canonical_alias: None,
                confidence_percentage: Some(90),
                fingerprint,
            }],
        },
        &master_db,
        &["ini".to_string()],
    )
    .await
    .unwrap();
    assert_eq!(valid.applied, 1);
    let matched_key: Option<String> =
        sqlx::query_scalar("SELECT matched_entry_key FROM objects WHERE id = 'o1'")
            .fetch_one(&context.pool)
            .await
            .unwrap();
    assert_eq!(matched_key.as_deref(), Some("ayaka"));
}
