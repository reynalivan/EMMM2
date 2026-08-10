use super::*;

#[tokio::test]
async fn create_collection_rolls_back_row_when_snapshot_persistence_fails() {
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
    sqlx::query(
        r#"CREATE TRIGGER fail_collection_mod_insert
        BEFORE INSERT ON collection_mods
        BEGIN
            SELECT RAISE(ABORT, 'forced snapshot persistence failure');
        END"#,
    )
    .execute(&ctx.pool)
    .await
    .expect("create failure trigger");

    create_collection(
        &ctx.pool,
        CreateCollectionInput {
            game_id: "game-1".to_string(),
            name: "Atomic Preset".to_string(),
            save_mode: Some(CreateCollectionMode::SaveCurrentState),
            source_collection_id: None,
        },
    )
    .await
    .expect_err("snapshot failure must fail create");

    let collections = collection_repo::list_for_game(&ctx.pool, "game-1")
        .await
        .expect("list collections");
    assert!(collections.is_empty());
}
