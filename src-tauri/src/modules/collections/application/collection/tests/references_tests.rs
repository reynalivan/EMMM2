use super::*;

async fn save_live_preview_member(
    pool: &sqlx::SqlitePool,
    mods_path: &str,
    member_path: &str,
    preview_path: &str,
) {
    seed_game(pool, "game-1", Some(mods_path)).await;
    seed_ainoz_object(pool, "object-1", "game-1").await;
    collection::create(pool, "saved", "game-1", "Saved", true, false)
        .await
        .expect("collection");
    let mut member = test_collection_mod("saved", member_path, "Saved mod");
    member.preview_path = Some(preview_path.to_string());
    let object = test_collection_object("saved");
    let state = projected_state::build_projected_state(
        std::slice::from_ref(&member),
        std::slice::from_ref(&object),
        Some(mods_path),
    );
    persist_projected_state(pool, "saved", &[member], &[object], &state)
        .await
        .expect("save live-like absolute preview metadata");
}

#[tokio::test]
async fn absolute_preview_follows_mod_rename_through_apply_path_validation() {
    let db = init_test_db().await;
    let root = tempfile::tempdir().expect("Mods root");
    let mods_path = root.path().to_string_lossy().to_string();
    create_flat_mod_folder(root.path(), "AINOZ/旧名");
    let old_path = root.path().join("AINOZ/旧名");
    save_live_preview_member(
        &db.pool,
        &mods_path,
        "AINOZ/旧名",
        &old_path.to_string_lossy(),
    )
    .await;
    let new_path = root.path().join("AINOZ/新名");
    std::fs::rename(&old_path, &new_path).expect("rename physical mod");
    handle_mod_moved_or_renamed(&db.pool, "game-1", "AINOZ/旧名", "AINOZ/新名", None)
        .await
        .expect("reconcile rename");
    let preview = get_collection_preview(&db.pool, "game-1", "saved", Some(&mods_path))
        .await
        .expect("preview");
    let mut apply =
        crate::modules::collections::application::apply::apply_pipeline::ApplyContext::new(
            ApplyCollectionRequest {
                pool: &db.pool,
                game_id: "game-1",
                collection_id: "saved",
                mods_path: root.path().to_path_buf(),
                suppressor: Arc::new(WatcherSuppressor::new(false)),
                ignore_missing: false,
                settings: AppSettings::default(),
                capture_last_changes: false,
            },
        );
    apply.target_mods =
        projected_state::mods_from_projected_state("saved", &preview.projected_state);
    crate::modules::collections::application::apply::steps::validate_paths::validate(&mut apply)
        .await
        .expect("renamed saved preview remains applicable");
    assert_eq!(apply.target_mods[0].mod_path, "AINOZ/新名");
    let members = collection::get_mods(&db.pool, "saved")
        .await
        .expect("members");
    assert_eq!(
        std::path::Path::new(
            members[0]
                .preview_path
                .as_deref()
                .expect("absolute preview")
        ),
        new_path
    );
}

#[tokio::test]
#[cfg(windows)]
async fn absolute_ancestor_preview_follows_object_rename_without_collapsing_pack() {
    let db = init_test_db().await;
    save_live_preview_member(
        &db.pool,
        "E:/Mods",
        "AINOZ/Pack/Variant",
        r"E:\Mods\AINOZ\Pack",
    )
    .await;
    let mut tx = db.pool.begin().await.expect("transaction");
    handle_object_renamed_tx(&mut tx, "game-1", "AINOZ", "日本語")
        .await
        .expect("object rename");
    tx.commit().await.expect("commit");
    let members = collection::get_mods(&db.pool, "saved")
        .await
        .expect("members");
    assert_eq!(members[0].mod_path, "日本語/Pack/Variant");
    assert_eq!(
        members[0].preview_path.as_deref(),
        Some(r"E:\Mods\日本語\Pack")
    );
}

#[tokio::test]
#[cfg(windows)]
async fn staged_identity_swap_preserves_absolute_ancestor_previews() {
    use crate::modules::collections::domain::collection::CollectionPathRewrite;
    let db = init_test_db().await;
    save_live_preview_member(
        &db.pool,
        "E:/Mods",
        "AINOZ/Pack/Variant",
        "E:/Mods/AINOZ/Pack",
    )
    .await;
    sqlx::query("INSERT INTO collection_mods (collection_id, mod_path, object_ref_key, object_id, preview_path, node_type) VALUES ('saved', 'Alice/Other', 'alice', 'object-1', 'E:/Mods/Alice/Other', 'FlatModRoot')")
        .execute(&db.pool).await.expect("second swap member");
    let mut tx = db.pool.begin().await.expect("transaction");
    let staged = super::super::references::stage_identity_path_transitions_tx(
        &mut tx,
        "game-1",
        &[
            CollectionPathRewrite {
                from: "AINOZ".to_string(),
                to: "Alice".to_string(),
            },
            CollectionPathRewrite {
                from: "Alice".to_string(),
                to: "AINOZ".to_string(),
            },
        ],
        &[CollectionPathRewrite {
            from: "AINOZ/Pack/Variant".to_string(),
            to: "Alice/Pack/Renamed".to_string(),
        }],
    )
    .await
    .expect("identity swap");
    super::super::references::refresh_collection_signatures_tx(
        &mut tx,
        &staged.affected_collection_ids,
    )
    .await
    .expect("refresh signatures");
    tx.commit().await.expect("commit");
    let members = collection::get_mods(&db.pool, "saved")
        .await
        .expect("members");
    let previews = members
        .iter()
        .map(|m| (m.mod_path.as_str(), m.preview_path.as_deref()))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        previews.get("Alice/Pack/Renamed"),
        Some(&Some("E:/Mods/Alice/Pack"))
    );
    assert_eq!(
        previews.get("AINOZ/Other"),
        Some(&Some("E:/Mods/AINOZ/Other"))
    );
}

#[tokio::test]
async fn preview_rewrite_preserves_ancestors_boundaries_and_other_game_ownership() {
    let db = init_test_db().await;
    seed_game(&db.pool, "game-1", Some("E:/Mods")).await;
    seed_game(&db.pool, "game-2", Some("E:/OtherMods")).await;
    let fixtures = [
        ("outside", "game-1", "AINOZ/Blue", "E:/Else/Mods/AINOZ/Blue"),
        (
            "outside-disabled-root",
            "game-1",
            "AINOZ/Blue",
            "E:/DISABLED Mods/AINOZ/Blue",
        ),
        (
            "outside-sibling-root",
            "game-1",
            "AINOZ/Blue",
            "E:/Mods2/AINOZ/Blue",
        ),
        ("sibling", "game-1", "AINOZ/Blue", "E:/Mods/AINOZX/Blue"),
        (
            "ancestor",
            "game-1",
            "AINOZ/Pack/Variant",
            "E:/Mods/AINOZ/Pack",
        ),
        (
            "other-game",
            "game-2",
            "AINOZ/Blue",
            "E:/OtherMods/AINOZ/Blue",
        ),
    ];
    for (id, game, member, preview) in fixtures {
        collection::create(&db.pool, id, game, id, true, false)
            .await
            .expect("collection");
        sqlx::query("INSERT INTO collection_mods (collection_id, mod_path, object_ref_key, preview_path, node_type) VALUES (?, ?, 'ainoz', ?, 'FlatModRoot')")
            .bind(id).bind(member).bind(preview).execute(&db.pool).await.expect("member");
    }
    handle_mod_moved_or_renamed(&db.pool, "game-1", "AINOZ/Blue", "AINOZ/Green", None)
        .await
        .expect("mod rename");
    handle_mod_moved_or_renamed(
        &db.pool,
        "game-1",
        "AINOZ/Pack/Variant",
        "AINOZ/Pack/New",
        None,
    )
    .await
    .expect("child rename");
    for (id, _, _, preview) in fixtures {
        let members = collection::get_mods(&db.pool, id).await.expect("members");
        assert_eq!(members[0].preview_path.as_deref(), Some(preview));
    }
    assert_eq!(
        collection::get_mods(&db.pool, "other-game")
            .await
            .expect("other game")[0]
            .mod_path,
        "AINOZ/Blue"
    );
}

#[tokio::test]
#[cfg(windows)]
async fn absolute_disabled_alias_preview_follows_semantic_mod_rename() {
    let db = init_test_db().await;
    save_live_preview_member(
        &db.pool,
        "E:/Mods",
        "AINOZ/Blue",
        "E:/Mods/DISABLED AINOZ/DISABLED Blue/Child",
    )
    .await;
    handle_mod_moved_or_renamed(
        &db.pool,
        "game-1",
        "AINOZ/DISABLED Blue",
        "AINOZ/Green",
        None,
    )
    .await
    .expect("semantic rename from runtime spelling");
    let members = collection::get_mods(&db.pool, "saved")
        .await
        .expect("members");
    assert_eq!(
        members[0].preview_path.as_deref(),
        Some("E:/Mods/AINOZ/Green/Child")
    );
}

#[tokio::test]
#[cfg(windows)]
async fn staged_preview_prefers_deeper_mod_transition_over_long_disabled_object_alias() {
    use crate::modules::collections::domain::collection::CollectionPathRewrite;
    let db = init_test_db().await;
    save_live_preview_member(&db.pool, "E:/Mods", "AINOZ/X", "E:/Mods/AINOZ/X").await;
    let mut tx = db.pool.begin().await.expect("transaction");
    super::super::references::stage_identity_path_transitions_tx(
        &mut tx,
        "game-1",
        &[CollectionPathRewrite {
            from: "DISABLED AINOZ".to_string(),
            to: "Alice".to_string(),
        }],
        &[CollectionPathRewrite {
            from: "AINOZ/X".to_string(),
            to: "Alice/Y".to_string(),
        }],
    )
    .await
    .expect("combined transition");
    tx.commit().await.expect("commit");
    let members = collection::get_mods(&db.pool, "saved")
        .await
        .expect("members");
    assert_eq!(members[0].mod_path, "Alice/Y");
    assert_eq!(members[0].preview_path.as_deref(), Some("E:/Mods/Alice/Y"));
}

#[tokio::test]
async fn auto_heal_rebuilds_snapshot_roots_signature_and_path_keys() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");
    let old_mod = CollectionMod {
        kind: MemberKind::Mod,
        collection_id: collection.id.clone(),
        mod_id: None,
        mod_path: "AINOZ/Old Mod".to_string(),
        mod_path_key: Some(crate::shared::path_key::folder_path_key(
            "AINOZ/Old Mod",
            None,
        )),
        object_id: "object-1".to_string(),
        display_name: Some("Old Mod".to_string()),
        preview_path: Some("AINOZ/Old Mod".to_string()),
        node_type: None,
        warnings: Vec::new(),
        is_enabled: true,
        is_safe: true,
        safety_source: Some("manual".to_string()),
    };
    let object = CollectionObject {
        kind: MemberKind::Object,
        collection_id: collection.id.clone(),
        object_id: "object-1".to_string(),
        is_enabled: true,
        display_name: Some("AINOZ".to_string()),
        path_key: Some("AINOZ".to_string()),
    };
    let old_state = projected_state::build_projected_state(
        std::slice::from_ref(&old_mod),
        std::slice::from_ref(&object),
        Some("E:/Mods"),
    );
    persist_projected_state(&ctx.pool, &collection.id, &[old_mod], &[object], &old_state)
        .await
        .expect("persist old state");
    let persisted_object_ref_key: String =
        sqlx::query_scalar("SELECT object_ref_key FROM collection_objects WHERE collection_id = ?")
            .bind(&collection.id)
            .fetch_one(&ctx.pool)
            .await
            .expect("load persisted object reference key");
    assert_eq!(
        persisted_object_ref_key,
        crate::shared::path_key::folder_path_key("AINOZ", None)
    );

    handle_mod_moved_or_renamed(&ctx.pool, "game-1", "AINOZ/Old Mod", "AINOZ/New Mod", None)
        .await
        .expect("auto heal path");

    let healed = collection::get_by_id(&ctx.pool, &collection.id)
        .await
        .expect("load collection")
        .expect("collection exists");
    let healed_state = projected_state::parse_snapshot_json(
        healed.snapshot_json.as_deref().expect("snapshot json"),
    )
    .expect("parse healed snapshot");
    let healed_mods = collection::get_mods(&ctx.pool, &collection.id)
        .await
        .expect("load healed mods");
    let expected_key = crate::shared::path_key::folder_path_key("AINOZ/New Mod", None);

    assert_eq!(
        healed_mods
            .first()
            .and_then(|mod_row| mod_row.mod_path_key.as_deref()),
        Some(expected_key.as_str())
    );
    assert_eq!(
        healed_state
            .active_roots
            .first()
            .map(|root| root.source_path.as_str()),
        Some("AINOZ/New Mod")
    );
    assert_eq!(
        healed.display_mod_count,
        healed_state.summary.active_root_count as i32
    );
}

#[tokio::test]
async fn auto_heal_returns_collection_reference_impact() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");
    let old_mod = test_collection_mod(&collection.id, "AINOZ/Old Mod", "Old Mod");
    let object = test_collection_object(&collection.id);
    let old_state = projected_state::build_projected_state(
        std::slice::from_ref(&old_mod),
        std::slice::from_ref(&object),
        None,
    );
    persist_projected_state(&ctx.pool, &collection.id, &[old_mod], &[object], &old_state)
        .await
        .expect("persist old state");

    let impact =
        handle_mod_moved_or_renamed(&ctx.pool, "game-1", "AINOZ/Old Mod", "AINOZ/New Mod", None)
            .await
            .expect("auto heal path");

    assert_eq!(impact.affected_collection_count, 1);
    assert_eq!(impact.affected_collection_names, vec!["Preset"]);
    assert_eq!(impact.rewritten_paths.len(), 1);
    assert_eq!(impact.rewritten_paths[0].from, "AINOZ/Old Mod");
    assert_eq!(impact.rewritten_paths[0].to, "AINOZ/New Mod");
    assert!(impact.missing_paths.is_empty());
}

#[tokio::test]
async fn runtime_prefix_toggle_does_not_rewrite_saved_collection_references() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");
    let mod_member = test_collection_mod(&collection.id, "AINOZ/Blue", "Blue");
    let object = test_collection_object(&collection.id);
    let projected_state = projected_state::build_projected_state(
        std::slice::from_ref(&mod_member),
        std::slice::from_ref(&object),
        None,
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        &[mod_member],
        &[object],
        &projected_state,
    )
    .await
    .expect("persist collection state");

    let impact = handle_mod_moved_or_renamed(
        &ctx.pool,
        "game-1",
        "AINOZ/Blue",
        "AINOZ/DISABLED Blue",
        None,
    )
    .await
    .expect("classify runtime prefix transition");
    let collection_mods = collection::get_mods(&ctx.pool, &collection.id)
        .await
        .expect("load collection mods");

    assert_eq!(impact.affected_collection_count, 0);
    assert!(impact.rewritten_paths.is_empty());
    assert_eq!(collection_mods[0].mod_path, "AINOZ/Blue");
}

#[tokio::test]
async fn object_runtime_prefix_toggle_does_not_rewrite_saved_collection_references() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");
    let mod_member = test_collection_mod(&collection.id, "AINOZ/Blue", "Blue");
    let object = test_collection_object(&collection.id);
    let projected_state = projected_state::build_projected_state(
        std::slice::from_ref(&mod_member),
        std::slice::from_ref(&object),
        None,
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        &[mod_member],
        &[object],
        &projected_state,
    )
    .await
    .expect("persist collection state");

    let mut tx = ctx.pool.begin().await.expect("begin tx");
    let impact = handle_object_renamed_tx(&mut tx, "game-1", "AINOZ", "DISABLED AINOZ")
        .await
        .expect("classify object runtime prefix transition");
    tx.commit().await.expect("commit tx");
    let collection_mods = collection::get_mods(&ctx.pool, &collection.id)
        .await
        .expect("load collection mods");

    assert_eq!(impact.affected_collection_count, 0);
    assert_eq!(collection_mods[0].mod_path, "AINOZ/Blue");
}

#[tokio::test]
async fn object_rename_rewrites_legacy_display_path_reference() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");
    sqlx::query(
        "INSERT INTO collection_objects (collection_id, object_ref_key, object_id, object_display_name, is_enabled) VALUES (?, 'AINOZ', 'object-1', 'AINOZ', 1)",
    )
    .bind(&collection.id)
    .execute(&ctx.pool)
    .await
    .expect("seed legacy collection reference");

    let mut tx = ctx.pool.begin().await.expect("begin transaction");
    let impact = handle_object_renamed_tx(&mut tx, "game-1", "AINOZ", "Alice")
        .await
        .expect("rewrite legacy object reference");
    tx.commit().await.expect("commit transaction");

    let rewritten: (String, String) = sqlx::query_as(
        "SELECT object_ref_key, object_display_name FROM collection_objects WHERE collection_id = ?",
    )
    .bind(&collection.id)
    .fetch_one(&ctx.pool)
    .await
    .expect("load rewritten reference");
    assert_eq!(
        rewritten.0,
        crate::shared::path_key::folder_path_key("Alice", None)
    );
    assert_eq!(rewritten.1, "Alice");
    assert_eq!(impact.affected_collection_count, 1);
    assert_eq!(impact.affected_collection_names, vec!["Preset"]);
}

#[tokio::test]
async fn missing_collection_member_is_preserved_and_reported_as_missing() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    create_flat_mod_folder(mods_root.path(), "AINOZ/Blue");

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");
    let mod_member = test_collection_mod(&collection.id, "AINOZ/Blue", "Blue");
    let object = test_collection_object(&collection.id);
    let projected_state = projected_state::build_projected_state(
        std::slice::from_ref(&mod_member),
        std::slice::from_ref(&object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        &[mod_member],
        &[object],
        &projected_state,
    )
    .await
    .expect("persist collection state");

    std::fs::remove_dir_all(mods_root.path().join("AINOZ/Blue")).expect("remove mod folder");

    let impact = handle_mod_missing(&ctx.pool, "game-1", "AINOZ/Blue")
        .await
        .expect("mark missing impact");
    let preview = get_collection_preview(&ctx.pool, "game-1", &collection.id, Some(&mods_path))
        .await
        .expect("load preview");
    let collection_mods = collection::get_mods(&ctx.pool, &collection.id)
        .await
        .expect("load collection mods");

    assert_eq!(impact.affected_collection_count, 1);
    assert_eq!(impact.affected_collection_names, vec!["Preset"]);
    assert_eq!(impact.missing_paths, vec!["AINOZ/Blue"]);
    assert_eq!(collection_mods.len(), 1);
    assert_eq!(collection_mods[0].mod_path, "AINOZ/Blue");
    assert_eq!(preview.projected_state.summary.missing_root_count, 1);
    assert_eq!(
        preview.tree_nodes[0].children[0].status_kind.as_deref(),
        Some("missing")
    );
}

#[tokio::test]
async fn path_rewrite_never_changes_a_collection_owned_by_another_game() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods/Game1")).await;
    seed_game(&ctx.pool, "game-2", Some("E:/Mods/Game2")).await;

    for (game_id, collection_id) in [("game-1", "collection-1"), ("game-2", "collection-2")] {
        collection::create(
            &ctx.pool,
            collection_id,
            game_id,
            &format!("Preset {game_id}"),
            true,
            false,
        )
        .await
        .expect("create collection");
        sqlx::query(
            "INSERT INTO collection_mods (collection_id, mod_path, mod_path_key, object_ref_key, object_id, node_type) VALUES (?, ?, ?, ?, NULL, ?)",
        )
        .bind(collection_id)
        .bind("AINOZ/Old Mod")
        .bind(crate::shared::path_key::folder_path_key("AINOZ/Old Mod", None))
        .bind("ainoz")
        .bind("FlatModRoot")
        .execute(&ctx.pool)
        .await
        .expect("seed collection member");
    }

    handle_mod_moved_or_renamed(&ctx.pool, "game-1", "AINOZ/Old Mod", "AINOZ/New Mod", None)
        .await
        .expect("rewrite game-1 path");

    let game_1_path: String = sqlx::query_scalar(
        "SELECT cm.mod_path FROM collection_mods cm JOIN collections c ON c.id = cm.collection_id WHERE c.game_id = 'game-1'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("game-1 path");
    let game_2_path: String = sqlx::query_scalar(
        "SELECT cm.mod_path FROM collection_mods cm JOIN collections c ON c.id = cm.collection_id WHERE c.game_id = 'game-2'",
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("game-2 path");

    assert_eq!(game_1_path, "AINOZ/New Mod");
    assert_eq!(game_2_path, "AINOZ/Old Mod");
}

#[tokio::test]
async fn object_prefix_rewrite_treats_sql_wildcards_as_literal_folder_names() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");
    for path in ["A_B/One", "AXB/Two", "100%/Three", "1000/Other"] {
        sqlx::query(
            "INSERT INTO collection_mods (collection_id, mod_path, mod_path_key, object_ref_key, object_id, node_type) VALUES ('collection-1', ?, ?, ?, NULL, 'FlatModRoot')",
        )
        .bind(path)
        .bind(crate::shared::path_key::folder_path_key(path, None))
        .bind(crate::shared::path_key::folder_path_key(path, None).split('/').next().unwrap())
        .execute(&ctx.pool)
        .await
        .expect("seed member");
    }

    let mut tx = ctx.pool.begin().await.expect("begin");
    handle_object_renamed_tx(&mut tx, "game-1", "A_B", "Under Score")
        .await
        .expect("rewrite underscore folder");
    handle_object_renamed_tx(&mut tx, "game-1", "100%", "Percent")
        .await
        .expect("rewrite percent folder");
    tx.commit().await.expect("commit");

    let paths: Vec<String> = sqlx::query_scalar(
        "SELECT mod_path FROM collection_mods WHERE collection_id = 'collection-1' ORDER BY mod_path",
    )
    .fetch_all(&ctx.pool)
    .await
    .expect("paths");
    assert_eq!(
        paths,
        vec!["1000/Other", "AXB/Two", "Percent/Three", "Under Score/One"]
    );
}
