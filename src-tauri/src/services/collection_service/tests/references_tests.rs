use super::*;

#[tokio::test]
async fn auto_heal_rebuilds_snapshot_roots_signature_and_path_keys() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
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
    };
    let object = CollectionObject {
        kind: MemberKind::Object,
        collection_id: collection.id.clone(),
        object_id: "object-1".to_string(),
        is_enabled: true,
        display_name: Some("AINOZ".to_string()),
        path_key: Some("AINOZ".to_string()),
    };
    let old_state = projected_state_service::build_projected_state(
        std::slice::from_ref(&old_mod),
        std::slice::from_ref(&object),
        Some("E:/Mods"),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        true,
        &[old_mod],
        &[object],
        &old_state,
    )
    .await
    .expect("persist old state");

    handle_mod_moved_or_renamed(&ctx.pool, "AINOZ/Old Mod", "AINOZ/New Mod", None)
        .await
        .expect("auto heal path");

    let healed = collection_repo::get_by_id(&ctx.pool, &collection.id)
        .await
        .expect("load collection")
        .expect("collection exists");
    let healed_state = projected_state_service::parse_snapshot_json(
        healed.snapshot_json.as_deref().expect("snapshot json"),
    )
    .expect("parse healed snapshot");
    let healed_mods = collection_repo::get_mods(&ctx.pool, &collection.id)
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
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");
    let old_mod = test_collection_mod(&collection.id, "AINOZ/Old Mod", "Old Mod");
    let object = test_collection_object(&collection.id);
    let old_state = projected_state_service::build_projected_state(
        std::slice::from_ref(&old_mod),
        std::slice::from_ref(&object),
        None,
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        true,
        &[old_mod],
        &[object],
        &old_state,
    )
    .await
    .expect("persist old state");

    let impact = handle_mod_moved_or_renamed(&ctx.pool, "AINOZ/Old Mod", "AINOZ/New Mod", None)
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
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");
    let mod_member = test_collection_mod(&collection.id, "AINOZ/Blue", "Blue");
    let object = test_collection_object(&collection.id);
    let projected_state = projected_state_service::build_projected_state(
        std::slice::from_ref(&mod_member),
        std::slice::from_ref(&object),
        None,
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        true,
        &[mod_member],
        &[object],
        &projected_state,
    )
    .await
    .expect("persist collection state");

    let impact = handle_mod_moved_or_renamed(&ctx.pool, "AINOZ/Blue", "AINOZ/DISABLED Blue", None)
        .await
        .expect("classify runtime prefix transition");
    let collection_mods = collection_repo::get_mods(&ctx.pool, &collection.id)
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
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");
    let mod_member = test_collection_mod(&collection.id, "AINOZ/Blue", "Blue");
    let object = test_collection_object(&collection.id);
    let projected_state = projected_state_service::build_projected_state(
        std::slice::from_ref(&mod_member),
        std::slice::from_ref(&object),
        None,
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        true,
        &[mod_member],
        &[object],
        &projected_state,
    )
    .await
    .expect("persist collection state");

    let mut tx = ctx.pool.begin().await.expect("begin tx");
    let impact = handle_object_renamed_tx(&mut tx, "AINOZ", "DISABLED AINOZ")
        .await
        .expect("classify object runtime prefix transition");
    tx.commit().await.expect("commit tx");
    let collection_mods = collection_repo::get_mods(&ctx.pool, &collection.id)
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
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");
    let mod_member = test_collection_mod(&collection.id, "AINOZ/Blue", "Blue");
    let object = test_collection_object(&collection.id);
    let projected_state = projected_state_service::build_projected_state(
        std::slice::from_ref(&mod_member),
        std::slice::from_ref(&object),
        Some(&mods_path),
    );
    persist_projected_state(
        &ctx.pool,
        &collection.id,
        true,
        &[mod_member],
        &[object],
        &projected_state,
    )
    .await
    .expect("persist collection state");

    std::fs::remove_dir_all(mods_root.path().join("AINOZ/Blue")).expect("remove mod folder");

    let impact = handle_mod_missing(&ctx.pool, "AINOZ/Blue")
        .await
        .expect("mark missing impact");
    let preview = get_collection_preview(&ctx.pool, "game-1", &collection.id, Some(&mods_path))
        .await
        .expect("load preview");
    let collection_mods = collection_repo::get_mods(&ctx.pool, &collection.id)
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
