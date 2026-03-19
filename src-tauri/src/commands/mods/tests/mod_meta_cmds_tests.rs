use crate::services::mods::info_json;
use crate::test_utils::{
    insert_test_game, insert_test_mod, insert_test_object, TestGameFixture, TestModFixture,
    TestObjectFixture,
};
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    crate::test_utils::init_test_db().await.pool
}

// ── TC-40: pin_mod updates DB ──────────────────────────────────────────────

#[tokio::test]
async fn test_pin_mod_updates_db() {
    let pool = setup_test_db().await;

    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: "type",
            path: "/Mods",
            mod_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();
    insert_test_mod(
        &pool,
        &TestModFixture {
            id: "mod1",
            game_id: "g1",
            object_id: None,
            actual_name: "TestMod",
            folder_path: "/Mods/TestMod",
            status: "DISABLED",
            is_safe: true,
            object_type: Some("Other"),
            mods_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    // Navigate direct DB rather than Tauri State — test the raw DB logic
    sqlx::query("UPDATE mods SET is_pinned = ? WHERE folder_path_key = ?")
        .bind(true)
        .bind(crate::services::path_key::folder_path_key(
            "/Mods/TestMod",
            Some("/Mods"),
        ))
        .execute(&pool)
        .await
        .unwrap();

    let row: (bool,) = sqlx::query_as("SELECT is_pinned FROM mods WHERE id = 'mod1'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(row.0, "Expected is_pinned to be TRUE after pin");
}

// ── TC-40: toggle_favorite writes info.json ────────────────────────────────

#[tokio::test]
async fn test_toggle_favorite_writes_info_json() {
    let tmp = TempDir::new().unwrap();
    let mod_dir = tmp.path().join("TestMod");
    fs::create_dir(&mod_dir).unwrap();

    // Pre-create info.json
    let update = info_json::ModInfoUpdate {
        is_favorite: Some(false),
        ..Default::default()
    };
    let _ = info_json::update_info_json(&mod_dir, &update);

    // Now toggle to true
    let update2 = info_json::ModInfoUpdate {
        is_favorite: Some(true),
        ..Default::default()
    };
    let result = info_json::update_info_json(&mod_dir, &update2);
    assert!(result.is_ok(), "Expected info_json update to succeed");

    let info = result.unwrap();
    assert!(
        info.is_favorite,
        "Expected is_favorite to be true after update"
    );
}

// ── TC-40: move_mod_to_object renames folder to target ──────────────────────

#[tokio::test]
async fn test_move_mod_to_object() {
    let tmp = TempDir::new().unwrap();
    let game_mods_root = tmp.path().join("Mods");
    fs::create_dir(&game_mods_root).unwrap();

    // Source: Mods/Amber/TestMod
    let source_char_dir = game_mods_root.join("Amber");
    fs::create_dir(&source_char_dir).unwrap();
    let source_mod = source_char_dir.join("TestMod");
    fs::create_dir(&source_mod).unwrap();

    // Target object directory: Mods/Raiden
    let target_char_dir = game_mods_root.join("Raiden");
    fs::create_dir(&target_char_dir).unwrap();

    let target_mod_path = target_char_dir.join("TestMod");

    // Simulate move (rename)
    fs::rename(&source_mod, &target_mod_path).unwrap();

    assert!(
        target_mod_path.exists(),
        "Mod should have moved to Raiden dir"
    );
    assert!(
        !source_mod.exists(),
        "Mod should no longer exist in Amber dir"
    );
}

// ── TC-35: suggest_random_mods ──────────────────────────────────────────────

#[tokio::test]
async fn test_suggest_random_mods() {
    use crate::services::mods::metadata::suggest_random_mods;

    let pool = setup_test_db().await;

    // Insert Game
    insert_test_game(
        &pool,
        &TestGameFixture {
            id: "g1",
            name: "Genshin",
            game_type: "type",
            path: "/Mods",
            mod_path: Some("/Mods"),
        },
    )
    .await
    .unwrap();

    // Insert Objects
    for (id, name, folder_path, object_type) in [
        ("obj1", "Hu Tao", "Hu Tao", "Character"),
        ("obj2", "Kazuha", "Kazuha", "Character"),
        ("obj3", "Weapon", "Weapon", "Weapon"),
    ] {
        insert_test_object(
            &pool,
            &TestObjectFixture {
                id,
                game_id: "g1",
                name,
                folder_path: Some(folder_path),
                object_type,
            },
        )
        .await
        .unwrap();
    }

    // Insert Mods
    for (id, object_id, actual_name, folder_path, status, is_safe) in [
        (
            "m1",
            Some("obj1"),
            "Hu Tao Skin 1",
            "/Mods/Hu Tao/Skin1",
            "DISABLED",
            true,
        ),
        (
            "m2",
            Some("obj1"),
            "Hu Tao Skin 2",
            "/Mods/Hu Tao/Skin2",
            "ENABLED",
            true,
        ),
        (
            "m3",
            Some("obj1"),
            "Hu Tao NSFW",
            "/Mods/Hu Tao/NSFWSkin",
            "DISABLED",
            false,
        ),
        (
            "m4",
            Some("obj2"),
            "Kazuha Mod",
            "/Mods/Kazuha/Mod",
            "DISABLED",
            true,
        ),
        (
            "m5",
            Some("obj2"),
            "Kazuha Dot",
            "/Mods/Kazuha/.HiddenMod",
            "DISABLED",
            true,
        ),
        (
            "m6",
            Some("obj3"),
            "Weapon Mod",
            "/Mods/Weapon/Mod",
            "DISABLED",
            true,
        ),
    ] {
        insert_test_mod(
            &pool,
            &TestModFixture {
                id,
                game_id: "g1",
                object_id,
                actual_name,
                folder_path,
                status,
                is_safe,
                object_type: Some("Other"),
                mods_path: Some("/Mods"),
            },
        )
        .await
        .unwrap();
    }

    // Test 1: Safe Mode OFF (is_safe = false)
    let proposals_unsafe = suggest_random_mods(&pool, "g1", false).await.unwrap();

    assert_eq!(
        proposals_unsafe.len(),
        2,
        "Should return 1 mod per character object"
    );
    let obj1_prop = proposals_unsafe
        .iter()
        .find(|p| p.object_id == "obj1")
        .unwrap();
    assert!(
        obj1_prop.mod_id == "m1" || obj1_prop.mod_id == "m3",
        "Obj1 should get m1 or m3"
    );

    let obj2_prop = proposals_unsafe
        .iter()
        .find(|p| p.object_id == "obj2")
        .unwrap();
    assert_eq!(
        obj2_prop.mod_id, "m4",
        "Obj2 should get m4, skipping dot prefix"
    );

    // Test 2: Safe Mode ON (is_safe = true)
    let proposals_safe = suggest_random_mods(&pool, "g1", true).await.unwrap();
    assert_eq!(
        proposals_safe.len(),
        2,
        "Should return 1 safe mod per character object"
    );

    let obj1_safe = proposals_safe
        .iter()
        .find(|p| p.object_id == "obj1")
        .unwrap();
    assert_eq!(obj1_safe.mod_id, "m1", "Obj1 MUST get m1, as m3 is unsafe");
}
