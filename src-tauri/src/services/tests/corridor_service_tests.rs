use super::{get_corridor_state, resolve_restore_collection};
use crate::domain::models::{GameType, ItemStatus};
use crate::repo::collection_repo;
use crate::services::projected_state_service;
use crate::test_utils::{
    init_test_db, insert_test_game, insert_test_mod, insert_test_object,
    set_test_collection_snapshot, set_test_corridor_active_unchecked, TestGameFixture,
    TestModFixture, TestObjectFixture,
};

#[tokio::test]
async fn get_corridor_state_reads_full_runtime_without_safety_filter() {
    let ctx = init_test_db().await;

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-runtime",
            name: "Test Game",
            game_type: GameType::GIMI,
            path: "E:/Games/TestGame",
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert game");
    insert_test_object(
        &ctx.pool,
        &TestObjectFixture {
            id: "object-1",
            game_id: "game-runtime",
            name: "AINOZ",
            folder_path: "AINOZ",
            object_type: "Character",
        },
    )
    .await
    .expect("insert object");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "safe-mod",
            game_id: "game-runtime",
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
    .expect("insert safe mod");
    insert_test_mod(
        &ctx.pool,
        &TestModFixture {
            id: "unsafe-mod",
            game_id: "game-runtime",
            object_id: Some("object-1"),
            actual_name: "Red",
            folder_path: "AINOZ/Red",
            status: ItemStatus::Enabled,
            is_safe: false,
            object_type: Some("Character"),
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert unsafe mod");

    let snapshot = get_corridor_state(
        &ctx.pool,
        "game-runtime",
        crate::domain::corridor::Corridor::from_is_safe(true),
    )
    .await
    .expect("load runtime snapshot");
    let mod_paths = snapshot
        .current_mods
        .iter()
        .map(|member| member.mod_path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(mod_paths, vec!["AINOZ/Blue", "AINOZ/Red"]);
    assert!(snapshot.active_collection_id.is_none());
    assert!(snapshot.active_collection_name.is_none());
    assert!(snapshot.is_dirty);
}
#[tokio::test]
async fn resolve_restore_collection_falls_back_to_unsaved_when_active_pointer_is_stale() {
    let ctx = init_test_db().await;

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-1",
            name: "Test Game",
            game_type: GameType::GIMI,
            path: "E:/Games/TestGame",
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert game");

    let unsaved = collection_repo::create(
        &ctx.pool,
        "unsaved-1",
        "game-1",
        "Unsaved 202603251210",
        false,
        true,
    )
    .await
    .expect("create unsaved");

    set_test_corridor_active_unchecked(&ctx.pool, "game-1", false, Some("missing-active"))
        .await
        .expect("set stale active pointer");

    let resolved = resolve_restore_collection(&ctx.pool, "game-1", false)
        .await
        .expect("resolve target")
        .expect("fallback target");

    assert_eq!(resolved.0.id, unsaved.id);
    assert_eq!(resolved.1, "unsaved");
}

#[tokio::test]
async fn get_corridor_state_ignores_legacy_unsaved_active_collection() {
    let ctx = init_test_db().await;

    insert_test_game(
        &ctx.pool,
        &TestGameFixture {
            id: "game-1",
            name: "Test Game",
            game_type: GameType::GIMI,
            path: "E:/Games/TestGame",
            mods_path: Some("E:/Mods"),
        },
    )
    .await
    .expect("insert game");

    let unsaved =
        collection_repo::create(&ctx.pool, "unsaved-1", "game-1", "202603251217", true, true)
            .await
            .expect("create unsaved");
    set_test_collection_snapshot(
        &ctx.pool,
        &unsaved.id,
        &projected_state_service::empty_projected_state(),
    )
    .await
    .expect("seed unsaved snapshot");

    set_test_corridor_active_unchecked(&ctx.pool, "game-1", true, Some(&unsaved.id))
        .await
        .expect("set active pointer");

    let snapshot = get_corridor_state(
        &ctx.pool,
        "game-1",
        crate::domain::corridor::Corridor::from_is_safe(true),
    )
    .await
    .expect("get corridor state");

    assert_eq!(snapshot.active_collection_id.as_deref(), None);
    assert_eq!(snapshot.active_collection_name.as_deref(), None);
}
