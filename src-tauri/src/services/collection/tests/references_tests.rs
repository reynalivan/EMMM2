use super::*;

#[tokio::test]
async fn auto_heal_rebuilds_snapshot_roots_signature_and_path_keys() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection =
        collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");
    let old_mod = CollectionMod {
        kind: MemberKind::Mod,
        collection_id: collection.id.clone(),
        mod_id: None,
        mod_path: "AINOZ/Old Mod".to_string(),
        mod_path_key: Some(crate::common::path_key::folder_path_key(
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
    let expected_key = crate::common::path_key::folder_path_key("AINOZ/New Mod", None);

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

    let collection =
        collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
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

    let collection =
        collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
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

    let collection =
        collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
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
async fn missing_collection_member_is_preserved_and_reported_as_missing() {
    let ctx = init_test_db().await;
    let mods_root = tempfile::tempdir().expect("create mods root");
    let mods_path = mods_root.path().to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;
    create_flat_mod_folder(mods_root.path(), "AINOZ/Blue");

    let collection =
        collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
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
        .bind(crate::common::path_key::folder_path_key("AINOZ/Old Mod", None))
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
        .bind(crate::common::path_key::folder_path_key(path, None))
        .bind(crate::common::path_key::folder_path_key(path, None).split('/').next().unwrap())
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
