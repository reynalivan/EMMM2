use super::*;

#[tokio::test]
async fn deleting_active_collection_clears_baseline_without_creating_a_draft() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    let active = collection::create(
        &ctx.pool,
        "active-1",
        "game-1",
        "Active Preset",
        true,
        false,
    )
    .await
    .expect("create active collection");
    collection::runtime::set_active(&ctx.pool, "game-1", Some(&active.id))
        .await
        .expect("set active baseline");

    delete_collection(&ctx.pool, &active.id)
        .await
        .expect("delete active collection");

    let runtime = collection::runtime::get(&ctx.pool, "game-1")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert!(runtime.active_collection_id.is_none());
    assert!(runtime.draft_collection_id.is_none());
    assert!(collection::get_by_id(&ctx.pool, &active.id)
        .await
        .expect("query deleted collection")
        .is_none());
}

#[tokio::test]
async fn deleting_inactive_collection_preserves_active_baseline_and_draft() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    let active = collection::create(
        &ctx.pool,
        "active-1",
        "game-1",
        "Active Preset",
        true,
        false,
    )
    .await
    .expect("create active collection");
    let inactive = collection::create(
        &ctx.pool,
        "inactive-1",
        "game-1",
        "Inactive Preset",
        false,
        false,
    )
    .await
    .expect("create inactive collection");
    let draft = collection::create(&ctx.pool, "draft-1", "game-1", "Last changes", true, true)
        .await
        .expect("create draft");
    let mut tx = ctx.pool.begin().await.expect("begin runtime transaction");
    collection::runtime::set_active_tx(&mut tx, "game-1", Some(&active.id))
        .await
        .expect("set active baseline");
    collection::runtime::set_draft_tx(&mut tx, "game-1", &draft.id, Some(&active.id))
        .await
        .expect("set draft");
    tx.commit().await.expect("commit runtime transaction");

    delete_collection(&ctx.pool, &inactive.id)
        .await
        .expect("delete inactive collection");

    let runtime = collection::runtime::get(&ctx.pool, "game-1")
        .await
        .expect("load runtime")
        .expect("runtime exists");
    assert_eq!(
        runtime.active_collection_id.as_deref(),
        Some(active.id.as_str())
    );
    assert_eq!(
        runtime.draft_collection_id.as_deref(),
        Some(draft.id.as_str())
    );
}

#[tokio::test]
async fn deleting_draft_directly_is_rejected() {
    let ctx = init_test_db().await;
    seed_game(&ctx.pool, "game-1", Some("E:/Mods")).await;
    let draft = collection::create(&ctx.pool, "draft-1", "game-1", "Last changes", true, true)
        .await
        .expect("create draft");

    let error = delete_collection(&ctx.pool, &draft.id)
        .await
        .expect_err("draft deletion must use clear last changes");
    assert!(matches!(error, CollectionError::Validation(_)));
}
