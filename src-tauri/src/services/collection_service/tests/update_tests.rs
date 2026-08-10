use super::*;

#[tokio::test]
async fn update_collection_rejects_collection_from_another_game() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods1")).await;
    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-2",
            name: "Second Game",
            game_type: GameType::GIMI,
            path: "E:/Games/TestGame2",
            mods_path: Some("E:/Mods2"),
        },
    )
    .await
    .expect("insert second game");
    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Original", true, false)
            .await
            .expect("create collection");

    let error = update_collection(
        &ctx.pool,
        UpdateCollectionInput {
            id: collection.id.clone(),
            game_id: "game-2".to_string(),
            name: Some("Cross-game rename".to_string()),
        },
    )
    .await
    .expect_err("cross-game update must fail");

    assert!(matches!(error, CollectionError::Validation(_)));
    let unchanged = collection_repo::get_by_id(&ctx.pool, &collection.id)
        .await
        .expect("query collection")
        .expect("collection exists");
    assert_eq!(unchanged.name, "Original");
}

#[tokio::test]
async fn rename_updates_name_key_and_preserves_duplicate_invariant() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Original", true, false)
            .await
            .expect("create collection");

    update_collection(
        &ctx.pool,
        UpdateCollectionInput {
            id: collection.id.clone(),
            game_id: "game-1".to_string(),
            name: Some("Renamed Preset".to_string()),
        },
    )
    .await
    .expect("rename collection");

    let renamed = collection_repo::get_by_id(&ctx.pool, &collection.id)
        .await
        .expect("query renamed collection")
        .expect("renamed collection exists");
    assert_eq!(
        renamed.name_key,
        crate::common::path_key::canonical_name_key("Renamed Preset")
    );
    collection_repo::create(&ctx.pool, "collection-2", "game-1", "Original", true, false)
        .await
        .expect("old name becomes available");
    let duplicate = collection_repo::create(
        &ctx.pool,
        "collection-3",
        "game-1",
        "renamed preset",
        true,
        false,
    )
    .await
    .expect_err("canonical duplicate must fail");
    assert!(matches!(duplicate, CollectionError::DuplicateName { .. }));
}
