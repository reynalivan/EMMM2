use super::*;

#[tokio::test]
async fn preview_apply_blocks_when_mods_root_is_unavailable() {
    let ctx = init_test_db().await;
    let temp_root = tempfile::tempdir().expect("create temp root");
    let missing_root = temp_root.path().join("missing-mods-root");
    let mods_path = missing_root.to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");

    let result = preview_apply(
        &ctx.pool,
        "game-1",
        &collection.id,
        crate::domain::corridor::Corridor::from_is_safe(true),
        Some(&mods_path),
    )
    .await;

    match result {
        Err(CollectionError::Corridor(crate::domain::errors::CorridorError::NoModsPath {
            game_id,
        })) => assert_eq!(game_id, "game-1"),
        other => panic!("expected source unavailable NoModsPath error, got {other:?}"),
    }
}

#[tokio::test]
async fn preview_apply_rejects_cross_corridor_request() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Unsafe", false, false)
            .await
            .expect("create collection");

    // Previewing an UNSAFE collection from the SAFE corridor must be rejected.
    let result = preview_apply(
        &ctx.pool,
        "game-1",
        &collection.id,
        crate::domain::corridor::Corridor::from_is_safe(true),
        None,
    )
    .await;

    assert!(
        matches!(result, Err(CollectionError::Validation(_))),
        "cross-corridor preview must be rejected, got {result:?}"
    );
}

#[tokio::test]
async fn get_collection_preview_rejects_cross_game_collection() {
    let ctx = init_test_db().await;
    for game_id in ["game-1", "game-2"] {
        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: game_id,
                name: "Test Game",
                game_type: GameType::GIMI,
                path: if game_id == "game-1" {
                    "E:/Games/TestGame1"
                } else {
                    "E:/Games/TestGame2"
                },
                mods_path: if game_id == "game-1" {
                    Some("E:/Mods1")
                } else {
                    Some("E:/Mods2")
                },
            },
        )
        .await
        .expect("insert game");
    }

    let collection =
        collection_repo::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
            .await
            .expect("create collection");

    let result = get_collection_preview(&ctx.pool, "game-2", &collection.id, Some("E:/Mods")).await;

    match result {
        Err(CollectionError::Validation(message)) => {
            assert!(message.contains("does not belong to game"));
        }
        other => panic!("expected game validation error, got {other:?}"),
    }
}
