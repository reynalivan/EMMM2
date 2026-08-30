use super::*;

#[tokio::test]
async fn fresh_database_uses_only_the_canonical_collection_schema() {
    let ctx = init_test_db().await;

    let legacy_objects: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM sqlite_master
           WHERE name IN (
             'corridor_state', 'corridor_runtime_cache', 'pin_config',
             'collection_nested_items', 'collection_roots', 'collection_signatures'
           )"#,
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("inspect schema objects");
    assert_eq!(legacy_objects, 0, "legacy schema objects must not exist");

    let collection_columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('collections')")
            .fetch_all(&ctx.pool)
            .await
            .expect("inspect collection columns");
    for legacy_column in ["is_unsaved", "is_last_unsaved", "last_active", "root_count"] {
        assert!(
            !collection_columns
                .iter()
                .any(|column| column == legacy_column),
            "legacy collection column {legacy_column} must not exist"
        );
    }
}

#[tokio::test]
async fn list_collections_hides_only_the_runtime_draft_pointer() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-pointer-list", Some("E:/Mods")).await;
    collection::create(
        &ctx.pool,
        "named-collection",
        "game-pointer-list",
        "Named",
        true,
        false,
    )
    .await
    .expect("create named collection");
    collection::create(
        &ctx.pool,
        "draft-collection",
        "game-pointer-list",
        "Internal draft row",
        true,
        false,
    )
    .await
    .expect("create row used as draft");
    let mut tx = ctx.pool.begin().await.expect("begin runtime update");
    crate::modules::collections::adapters::sqlite::runtime::set_draft_tx(
        &mut tx,
        "game-pointer-list",
        "draft-collection",
        Some("named-collection"),
    )
    .await
    .expect("set draft pointer");
    tx.commit().await.expect("commit runtime update");

    let collections = list_collections(&ctx.pool, "game-pointer-list")
        .await
        .expect("list named collections");
    let ids = collections
        .iter()
        .map(|collection| collection.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["named-collection"]);
}

#[tokio::test]
async fn canonical_collection_list_uses_game_and_runtime_indexes_without_loading_snapshots() {
    let ctx = init_test_db().await;
    let plan_rows: Vec<(i64, i64, i64, String)> = sqlx::query_as(
        r#"EXPLAIN QUERY PLAN
           SELECT c.id, c.game_id, c.name, c.name_key, c.is_safe,
                  0 AS is_draft, c.signature, c.display_mod_count,
                  c.created_at, c.updated_at
           FROM collections c
           LEFT JOIN collection_runtime_state runtime ON runtime.game_id = c.game_id
           WHERE c.game_id = ?
             AND (runtime.draft_collection_id IS NULL OR runtime.draft_collection_id != c.id)
           ORDER BY c.name ASC"#,
    )
    .bind("game-query-plan")
    .fetch_all(&ctx.pool)
    .await
    .expect("explain collection list");
    let plan_text = plan_rows
        .into_iter()
        .map(|(_, _, _, detail)| detail)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        plan_text.contains("idx_collections_named_name_key_game"),
        "collection game predicate must use the canonical game/name index: {plan_text}"
    );
    assert!(
        plan_text.contains("collection_runtime_state") && plan_text.contains("game_id"),
        "draft-pointer join must use the runtime primary key: {plan_text}"
    );
}

#[tokio::test]
async fn save_current_state_becomes_the_game_runtime_baseline() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-save-no-pointer", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-save-no-pointer", "game-save-no-pointer").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-save-no-pointer",
            game_id: "game-save-no-pointer",
            object_id: Some("object-save-no-pointer"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert mod");
    let saved = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-save-no-pointer".to_string(),
            name: "Saved Snapshot".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("save current state");
    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&ctx.pool, "game-save-no-pointer")
        .await
        .expect("load runtime")
        .expect("runtime row exists");

    assert!(saved.is_active);
    assert_eq!(
        runtime.active_collection_id.as_deref(),
        Some(saved.id.as_str())
    );
    assert!(runtime.draft_collection_id.is_none());
    let draft_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM collection_runtime_state WHERE game_id = ? AND draft_collection_id IS NOT NULL",
    )
    .bind("game-save-no-pointer")
    .fetch_one(&ctx.pool)
    .await
    .expect("count drafts");
    assert_eq!(draft_count, 0);
}

#[tokio::test]
async fn list_collections_returns_all_named_presets_across_safety_flags() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-list-all", Some("E:/Mods")).await;
    collection::create(
        &ctx.pool,
        "safe-collection",
        "game-list-all",
        "Safe",
        true,
        false,
    )
    .await
    .expect("create safe collection");
    collection::create(
        &ctx.pool,
        "unsafe-collection",
        "game-list-all",
        "Unsafe",
        false,
        false,
    )
    .await
    .expect("create unsafe collection");
    collection::create(
        &ctx.pool,
        "unsaved-collection",
        "game-list-all",
        "Unsaved",
        false,
        true,
    )
    .await
    .expect("create legacy unsaved collection");

    let collections = list_collections(&ctx.pool, "game-list-all")
        .await
        .expect("list collections");
    let ids = collections
        .iter()
        .map(|collection| collection.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["safe-collection", "unsafe-collection"]);
}

#[tokio::test]
async fn collection_safety_summary_tracks_current_member_classification_without_dirtying_state() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-1",
            game_id: "game-1",
            object_id: Some("object-1"),
            actual_name: "Private",
            folder_path: "AINOZ/Private",
            status: ItemStatus::Enabled,
            is_safe: false,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert mod");
    sqlx::query("UPDATE mods SET safety_source = 'manual' WHERE id = 'mod-1'")
        .execute(&ctx.pool)
        .await
        .expect("classify mod");

    let saved = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Preset".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("save collection");
    assert!(!saved.is_safe);

    sqlx::query("UPDATE mods SET is_safe = 1 WHERE id = 'mod-1'")
        .execute(&ctx.pool)
        .await
        .expect("reclassify mod");
    let summaries = list_collections(&ctx.pool, "game-1")
        .await
        .expect("list collections");

    assert!(summaries[0].is_safe);
    let runtime = crate::modules::collections::application::runtime::get_collection_runtime_state(
        &ctx.pool, "game-1",
    )
    .await
    .expect("load runtime");
    assert_eq!(
        runtime.runtime_status,
        crate::modules::workspace::domain::runtime_state::RuntimeStatus::Clean
    );
}

#[tokio::test]
async fn missing_member_keeps_its_own_snapshot_safety_in_mixed_collection() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    for (id, name, is_safe) in [
        ("mod-safe", "Public", true),
        ("mod-unsafe", "Private", false),
    ] {
        insert_test_mod(
            &ctx.pool,
            &TestModFixture {
                id,
                game_id: "game-1",
                object_id: Some("object-1"),
                actual_name: name,
                folder_path: &format!("AINOZ/{name}"),
                status: ItemStatus::Enabled,
                is_safe,
                object_type: Some("Character"),
                mods_path: Some("E:/Mods"),
            },
        )
        .await
        .expect("insert mod");
    }
    sqlx::query("UPDATE mods SET safety_source = 'manual' WHERE game_id = 'game-1'")
        .execute(&ctx.pool)
        .await
        .expect("classify mods");

    create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Mixed".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("save mixed collection");
    sqlx::query("DELETE FROM mods WHERE id = 'mod-unsafe'")
        .execute(&ctx.pool)
        .await
        .expect("remove unsafe mod from live projection");

    let summaries = list_collections(&ctx.pool, "game-1")
        .await
        .expect("list collections");
    assert!(!summaries[0].is_safe);
    assert!(summaries[0].is_safety_classified);
}

#[tokio::test]
async fn unclassified_collection_is_not_reported_as_classified_safe() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-unknown",
            game_id: "game-1",
            object_id: Some("object-1"),
            actual_name: "Unknown",
            folder_path: "AINOZ/Unknown",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert unclassified mod");
    create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Unknown".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("save collection");

    let summaries = list_collections(&ctx.pool, "game-1")
        .await
        .expect("list collections");
    assert!(summaries[0].is_safe);
    assert!(!summaries[0].is_safety_classified);
}

#[tokio::test]
async fn clone_snapshot_does_not_change_the_runtime_baseline() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;

    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-1",
            game_id: "game-1",
            object_id: Some("object-1"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert mod");

    let source = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Source Preset".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("create source snapshot");
    let cloned = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Cloned Preset".to_string(),
            save_mode: Some(CreateCollectionMode::CloneSnapshot),
            source_collection_id: Some(source.id.clone()),
        },
    )
    .await
    .expect("clone source snapshot");

    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&ctx.pool, "game-1")
        .await
        .expect("load runtime")
        .expect("runtime exists");

    assert_eq!(
        runtime.active_collection_id.as_deref(),
        Some(source.id.as_str())
    );
    assert!(!cloned.is_active);
}

#[tokio::test]
async fn update_collection_returns_preview_tree_mod_count() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;

    let collection =
        collection::create(&ctx.pool, "collection-1", "game-1", "Original", true, false)
            .await
            .expect("create collection");

    let snapshot = ProjectedCollectionState {
        object_states: Vec::new(),
        active_roots: Vec::new(),
        summary: ProjectedStateSummary {
            object_count: 0,
            enabled_object_count: 0,
            active_root_count: 7,
            missing_root_count: 0,
        },
    };
    let snapshot_json =
        projected_state::serialize_snapshot_json(&snapshot).expect("serialize snapshot");

    sqlx::query(
        "UPDATE collections SET snapshot_json = ?, signature = ?, display_mod_count = ? WHERE id = ?",
    )
    .bind(snapshot_json)
    .bind("sig-1")
    .bind(7_i32)
    .bind(&collection.id)
    .execute(&ctx.pool)
    .await
    .expect("update snapshot");

    let updated = update_collection(
        &ctx.pool,
        UpdateCollectionInput {
            id: collection.id.clone(),
            game_id: "game-1".to_string(),
            name: Some("Renamed".to_string()),
        },
    )
    .await
    .expect("update collection");

    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.mod_count, 7);
}

#[tokio::test]
async fn clean_runtime_does_not_create_or_replace_last_changes() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-clean-draft", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-clean-draft", "game-clean-draft").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-clean-draft",
            game_id: "game-clean-draft",
            object_id: Some("object-clean-draft"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert mod");

    create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-clean-draft".to_string(),
            name: "Baseline".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect("create baseline");

    let captured = capture_last_changes_if_needed(&ctx.pool, "game-clean-draft")
        .await
        .expect("capture decision");

    assert!(captured.is_none());
    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&ctx.pool, "game-clean-draft")
        .await
        .expect("runtime state")
        .expect("runtime row");
    assert!(runtime.draft_collection_id.is_none());
}

#[tokio::test]
async fn unsaved_runtime_creates_one_last_changes_draft_without_baseline() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-unsaved-draft", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-unsaved-draft", "game-unsaved-draft").await;
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-unsaved-draft",
            game_id: "game-unsaved-draft",
            object_id: Some("object-unsaved-draft"),
            actual_name: "Blue",
            folder_path: "AINOZ/Blue",
            status: ItemStatus::Enabled,
            is_safe: false,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert mod");

    let draft_id = capture_last_changes_if_needed(&ctx.pool, "game-unsaved-draft")
        .await
        .expect("capture draft")
        .expect("draft id");
    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&ctx.pool, "game-unsaved-draft")
        .await
        .expect("runtime state")
        .expect("runtime row");

    assert_eq!(
        runtime.draft_collection_id.as_deref(),
        Some(draft_id.as_str())
    );
    assert!(runtime.draft_base_collection_id.is_none());
    assert!(
        collection::get_by_id(&ctx.pool, &draft_id)
            .await
            .expect("load draft")
            .expect("draft row")
            .is_draft
    );
}

#[tokio::test]
async fn all_disabled_unsaved_runtime_still_captures_rollback_snapshot() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-disabled-draft", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-disabled-draft", "game-disabled-draft").await;
    sqlx::query("UPDATE objects SET status = ?, folder_path = ?, folder_path_key = ? WHERE id = ?")
        .bind(ItemStatus::Disabled as i64)
        .bind("DISABLED AINOZ")
        .bind(crate::shared::path_key::folder_path_key(
            "DISABLED AINOZ",
            Some("E:/Mods"),
        ))
        .bind("object-disabled-draft")
        .execute(&ctx.pool)
        .await
        .expect("disable object");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "mod-disabled-draft",
            game_id: "game-disabled-draft",
            object_id: Some("object-disabled-draft"),
            actual_name: "Blue",
            folder_path: "DISABLED AINOZ/DISABLED Blue",
            status: ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert disabled mod");

    let draft_id = capture_last_changes_if_needed(&ctx.pool, "game-disabled-draft")
        .await
        .expect("capture draft")
        .expect("disabled-only runtime needs rollback draft");
    let draft = collection::get_by_id(&ctx.pool, &draft_id)
        .await
        .expect("load draft")
        .expect("draft exists");
    let projected = projected_state::parse_snapshot_json(
        draft.snapshot_json.as_deref().expect("draft snapshot"),
    )
    .expect("parse draft snapshot");

    assert_eq!(projected.summary.active_root_count, 0);
    assert_eq!(projected.summary.object_count, 1);
    assert_eq!(projected.summary.enabled_object_count, 0);
}

#[tokio::test]
async fn clear_last_changes_rejects_a_draft_referenced_by_an_open_apply() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-clear-protected", Some("E:/Mods")).await;
    let draft = collection::create(
        &ctx.pool,
        "draft-protected",
        "game-clear-protected",
        "Last changes",
        true,
        true,
    )
    .await
    .expect("create draft");
    crate::modules::collections::adapters::sqlite::runtime::set_draft_tx(
        &mut ctx.pool.acquire().await.expect("runtime connection"),
        "game-clear-protected",
        &draft.id,
        None,
    )
    .await
    .expect("set draft pointer");
    crate::modules::workspace::adapters::sqlite::task::create_task_with_rollback_intent(
        &ctx.pool,
        "open-rollback",
        "game-clear-protected",
        "apply_collection",
        Some("target"),
        Some(&draft.id),
        None,
    )
    .await
    .expect("create open task");

    let error =
        crate::modules::collections::application::collection::clear_last_changes(&ctx.pool, "game-clear-protected")
            .await
            .expect_err("open recovery task must protect its rollback draft");

    assert!(format!("{error}").contains("recovery"));
    let runtime = crate::modules::collections::adapters::sqlite::runtime::get(&ctx.pool, "game-clear-protected")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert_eq!(
        runtime.draft_collection_id.as_deref(),
        Some(draft.id.as_str())
    );
    assert!(collection::get_by_id(&ctx.pool, &draft.id)
        .await
        .expect("load protected draft")
        .is_some());
}
