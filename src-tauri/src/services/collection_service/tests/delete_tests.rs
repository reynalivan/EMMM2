use super::*;

#[tokio::test]
async fn delete_collection_clears_legacy_pointers_without_promoting_unsaved() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;

    let active =
        collection_repo::create(&ctx.pool, "named-1", "game-1", "Named Preset", true, false)
            .await
            .expect("create active");
    let unsaved = collection_repo::create(
        &ctx.pool,
        "unsaved-1",
        "game-1",
        "Unsaved 202603251200",
        true,
        true,
    )
    .await
    .expect("create unsaved");
    set_test_corridor_active_unchecked(&ctx.pool, "game-1", true, Some(&active.id))
        .await
        .expect("set pointers");

    delete_collection(&ctx.pool, &active.id)
        .await
        .expect("delete active");

    let snapshot = corridor_repo::get(&ctx.pool, "game-1", true)
        .await
        .expect("load corridor")
        .expect("corridor exists");

    assert!(snapshot.active_collection_id.is_none());
    assert!(collection_repo::get_by_id(&ctx.pool, &unsaved.id)
        .await
        .expect("query unsaved")
        .is_some());
}

#[tokio::test]
async fn delete_saved_collection_does_not_recreate_unsaved_state() {
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

    let dirty_summary = handle_dirty_state(&ctx.pool, "game-1", true)
        .await
        .expect("build dirty runtime summary");
    assert!(dirty_summary.is_unsaved);
    assert!(!dirty_summary.is_active);
    assert!(collection_repo::get_by_id(&ctx.pool, &dirty_summary.id)
        .await
        .expect("query dirty summary")
        .is_none());

    let named = create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Named Preset".to_string(),
            is_safe: true,
            save_mode: None,
            source_collection_id: None,
        },
    )
    .await
    .expect("create named collection");
    assert!(!named.is_unsaved);
    assert!(!named.is_active);

    let unsaved_after_save =
        collection_repo::find_unsaved_for_corridor(&ctx.pool, "game-1", true, None)
            .await
            .expect("query unsaved after save");
    assert!(unsaved_after_save.is_none());

    delete_collection(&ctx.pool, &named.id)
        .await
        .expect("delete named collection");

    let corridor = corridor_repo::get(&ctx.pool, "game-1", true)
        .await
        .expect("load corridor")
        .expect("corridor exists");
    let recreated_unsaved =
        collection_repo::find_unsaved_for_corridor(&ctx.pool, "game-1", true, None)
            .await
            .expect("query recreated unsaved");
    let collections = collection_repo::list_for_corridor(&ctx.pool, "game-1", true, true)
        .await
        .expect("list collections");

    assert!(corridor.active_collection_id.is_none());
    assert!(recreated_unsaved.is_none());
    assert!(collections.is_empty());
}

#[tokio::test]
async fn delete_in_other_corridor_does_not_create_unsaved_here() {
    let ctx = init_test_db().await;

    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;

    let unsafe_named = collection_repo::create(
        &ctx.pool,
        "unsafe-named",
        "game-1",
        "Unsafe Named",
        false,
        false,
    )
    .await
    .expect("create unsafe named");

    set_test_corridor_active_unchecked(&ctx.pool, "game-1", false, Some(&unsafe_named.id))
        .await
        .expect("set unsafe active");
    set_test_corridor_active_unchecked(&ctx.pool, "game-1", true, None)
        .await
        .expect("seed safe corridor");

    delete_collection(&ctx.pool, &unsafe_named.id)
        .await
        .expect("delete unsafe named");

    let safe_unsaved = collection_repo::find_unsaved_for_corridor(&ctx.pool, "game-1", true, None)
        .await
        .expect("query safe unsaved");
    let safe_corridor = corridor_repo::get(&ctx.pool, "game-1", true)
        .await
        .expect("load safe corridor")
        .expect("safe corridor exists");

    assert!(safe_unsaved.is_none());
    assert!(safe_corridor.active_collection_id.is_none());
}
