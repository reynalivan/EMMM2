use super::*;
use crate::database::game_repo::{upsert_game, GameRow};
use crate::database::mod_repo::{insert_new_mod, update_mod_path_status_and_reason};

#[tokio::test]
async fn test_preview_corridor_switch() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;

    // Insert game
    let game = GameRow {
        id: "g1".into(),
        name: "Game 1".into(),
        game_type: "GIMI".into(),
        path: "C:\\Game1".into(),
        mod_path: Some("C:\\Game1\\Mods".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(pool, &game).await.unwrap();

    // Insert an active SFW mod
    insert_new_mod(pool, "mod_sfw1", "g1", "SFW Mod 1", "Mods/SFW1", "ENABLED")
        .await
        .unwrap();

    // Insert a SYSTEM-disabled NSFW mod (i.e., currently in Safe Mode)
    // First insert as normal, then update reason to SYSTEM and safe context to false
    insert_new_mod(
        pool,
        "mod_nsfw1",
        "g1",
        "NSFW Mod 1",
        "Mods/DISABLED NSFW1",
        "DISABLED",
    )
    .await
    .unwrap();
    update_mod_path_status_and_reason(
        pool,
        "g1",
        "Mods/DISABLED NSFW1",
        "Mods/DISABLED NSFW1",
        "DISABLED",
        Some("SYSTEM"),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE mods SET is_safe = 0 WHERE id = 'mod_nsfw1'")
        .execute(pool)
        .await
        .unwrap();

    // Test preview: switching from SFW (Safe) to NSFW (Unsafe)
    let preview = preview_corridor_switch(pool, "g1", true, false)
        .await
        .expect("preview failed");

    // The 'leaving' state should show the active SFW mod
    assert_eq!(preview.leaving_mods.len(), 1);
    assert_eq!(preview.leaving_mods[0].id, "mod_sfw1");

    // The 'target' state should show the SYSTEM-disabled NSFW mod that will be restored
    assert_eq!(preview.target_mods.len(), 1);
    assert_eq!(preview.target_mods[0].id, "mod_nsfw1");
    // And description should format the count
    assert_eq!(preview.target_description, "Restoring 1 Mods");

    // Test preview: switching to an empty corridor (e.g., Unsafe -> Safe, but no safe mods are SYSTEM disabled)
    update_mod_path_status_and_reason(pool, "g1", "Mods/SFW1", "Mods/SFW1", "ENABLED", None)
        .await
        .unwrap(); // Make sure SFW mod is enabled, not disabled

    let preview_empty = preview_corridor_switch(pool, "g1", false, true)
        .await
        .unwrap();

    // Leaving should be empty in this case (since we didn't enable any NSFW mods)
    assert_eq!(preview_empty.leaving_mods.len(), 0);

    // Target should be empty because there are no SYSTEM disabled SFW mods
    assert_eq!(preview_empty.target_mods.len(), 0);
    assert_eq!(
        preview_empty.target_description,
        "Empty State (All Disabled)"
    );
}

#[tokio::test]
async fn test_switch_mode_preserves_depth_1() {
    let ctx = crate::test_utils::init_test_db().await;
    let pool = &ctx.pool;

    // Insert game
    let game = GameRow {
        id: "g1".into(),
        name: "Game 1".into(),
        game_type: "GIMI".into(),
        path: "C:\\Game1".into(),
        mod_path: Some("C:\\Game1\\Mods".into()),
        game_exe: None,
        launcher_path: None,
        loader_exe: None,
        launch_args: None,
    };
    upsert_game(pool, &game).await.unwrap();

    // Insert a depth-1 "Object" folder (should be preserved)
    insert_new_mod(pool, "obj1", "g1", "Object 1", "Obj1", "ENABLED")
        .await
        .unwrap();

    // Insert a depth-2 "Mod" folder (should be disabled)
    insert_new_mod(pool, "mod1", "g1", "Mod 1", "Obj1/Mod1", "ENABLED")
        .await
        .unwrap();

    // We can't easily test the actual FS rename in a unit test without a mock watcher/fs,
    // but we can test the `disable_all_enabled_mods` internal logic if we were to expose it
    // or test the DB result if `switch_mode` was fully injectable.

    // For now, let's just test that our filter logic works if we call it.
    // Since disable_all_enabled_mods is private, let's test switch_mode if possible,
    // or just trust the logic if we can't easily mock the bulk_toggle_mods which calls FS.

    // Actually, bulk_toggle_mods will fail because "C:\Game1\Mods" doesn't exist.
    // Let's use a temp dir.
    let temp = tempfile::tempdir().unwrap();
    let mods_path = temp.path().to_string_lossy().to_string();

    let game_updated = GameRow {
        mod_path: Some(mods_path.clone()),
        ..game
    };
    upsert_game(pool, &game_updated).await.unwrap();

    // Create the actual directories
    std::fs::create_dir_all(temp.path().join("Obj1")).unwrap();
    std::fs::create_dir_all(temp.path().join("Obj1/Mod1")).unwrap();

    let watcher = WatcherState::default();

    // Switch to NSFW (leaving SFW)
    // This calls disable_all_enabled_mods
    switch_mode(Mode::NSFW, pool, &watcher, "g1").await.unwrap();

    // Check DB status
    let obj_status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = 'obj1'")
        .fetch_one(pool)
        .await
        .unwrap();
    let mod_status: String = sqlx::query_scalar("SELECT status FROM mods WHERE id = 'mod1'")
        .fetch_one(pool)
        .await
        .unwrap();

    // Object (depth 1) should stay ENABLED
    assert_eq!(obj_status, "ENABLED");
    // Mod (depth 2) should be DISABLED
    assert_eq!(mod_status, "DISABLED");
}
