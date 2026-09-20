use super::*;

#[tokio::test]
async fn preview_apply_blocks_when_mods_root_is_unavailable() {
    let ctx = init_test_db().await;
    let temp_root = tempfile::tempdir().expect("create temp root");
    let missing_root = temp_root.path().join("missing-mods-root");
    let mods_path = missing_root.to_string_lossy().to_string();

    seed_game(&ctx.pool, "game-1", Some(&mods_path)).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");

    let result = preview_apply(&ctx.pool, "game-1", &collection.id, Some(&mods_path), false).await;

    match result {
        Err(CollectionError::RuntimeState(
            crate::shared::errors::RuntimeStateError::NoModsPath { game_id },
        )) => assert_eq!(game_id, "game-1"),
        other => panic!("expected source unavailable NoModsPath error, got {other:?}"),
    }
}

#[tokio::test]
async fn preview_apply_allows_collection_regardless_of_safety_classification() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;

    let collection =
        collection::create(&ctx.pool, "collection-1", "game-1", "Unsafe", false, false)
            .await
            .expect("create collection");

    let result = preview_apply(&ctx.pool, "game-1", &collection.id, None, false).await;

    assert!(result.is_ok(), "safety is view metadata, got {result:?}");
}

#[tokio::test]
async fn preview_apply_uses_the_safe_mode_effective_target() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    seed_ainoz_object(&ctx.pool, "object-1", "game-1").await;

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
        .await
        .expect("create collection");
    let safe_mod = test_collection_mod(&collection.id, "AINOZ/Safe", "Safe");
    let mut unsafe_mod = test_collection_mod(&collection.id, "AINOZ/Unsafe", "Unsafe");
    unsafe_mod.is_safe = false;
    let objects = vec![test_collection_object(&collection.id)];
    let mods = vec![safe_mod, unsafe_mod];
    let state = projected_state::build_projected_state(&mods, &objects, None);
    persist_projected_state(&ctx.pool, &collection.id, &mods, &objects, &state)
        .await
        .expect("persist collection state");

    let preview = preview_apply(&ctx.pool, "game-1", &collection.id, None, true)
        .await
        .expect("preview collection apply");

    assert!(preview.safe_mode_enabled);
    assert_eq!(preview.target_projected_state.active_roots.len(), 2);
    assert_eq!(
        preview.effective_target_projected_state.active_roots.len(),
        1
    );
    assert_eq!(
        preview.effective_target_projected_state.active_roots[0].display_name,
        "Safe"
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

    let collection = collection::create(&ctx.pool, "collection-1", "game-1", "Preset", true, false)
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
