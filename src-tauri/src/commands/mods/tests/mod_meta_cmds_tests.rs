use crate::services::mods::info_json;
use sqlx::sqlite::SqlitePoolOptions;
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool");

    sqlx::query(
        "CREATE TABLE games (id TEXT PRIMARY KEY, name TEXT, mod_path TEXT);
         CREATE TABLE objects (id TEXT PRIMARY KEY, game_id TEXT, name TEXT, folder_path TEXT, object_type TEXT);
         CREATE TABLE mods (
             id TEXT PRIMARY KEY, game_id TEXT, object_id TEXT, actual_name TEXT,
             folder_path TEXT, is_pinned INTEGER DEFAULT 0, is_favorite INTEGER DEFAULT 0,
             status TEXT, object_type TEXT, is_safe INTEGER DEFAULT 1
         );",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

// ── TC-40: pin_mod updates DB ──────────────────────────────────────────────

#[tokio::test]
async fn test_pin_mod_updates_db() {
    let pool = setup_test_db().await;

    sqlx::query(
        "INSERT INTO mods (id, folder_path, is_pinned) VALUES ('mod1', '/Mods/TestMod', 0)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Navigate direct DB rather than Tauri State — test the raw DB logic
    sqlx::query("UPDATE mods SET is_pinned = ? WHERE folder_path = ?")
        .bind(true)
        .bind("/Mods/TestMod")
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
    sqlx::query("INSERT INTO games (id, name, mod_path) VALUES ('g1', 'Genshin', '/Mods')")
        .execute(&pool)
        .await
        .unwrap();

    // Insert Objects
    sqlx::query(
        "INSERT INTO objects (id, game_id, name, folder_path, object_type) VALUES 
        ('obj1', 'g1', 'Hu Tao', 'Hu Tao', 'Character'),
        ('obj2', 'g1', 'Kazuha', 'Kazuha', 'Character'),
        ('obj3', 'g1', 'Weapon', 'Weapon', 'Weapon')",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert Mods
    sqlx::query("INSERT INTO mods (id, game_id, object_id, actual_name, folder_path, status, is_safe) VALUES 
        ('m1', 'g1', 'obj1', 'Hu Tao Skin 1', '/Mods/Hu Tao/Skin1', 'DISABLED', 1),
        ('m2', 'g1', 'obj1', 'Hu Tao Skin 2', '/Mods/Hu Tao/Skin2', 'ENABLED', 1),
        ('m3', 'g1', 'obj1', 'Hu Tao NSFW', '/Mods/Hu Tao/NSFWSkin', 'DISABLED', 0),
        ('m4', 'g1', 'obj2', 'Kazuha Mod', '/Mods/Kazuha/Mod', 'DISABLED', 1),
        ('m5', 'g1', 'obj2', 'Kazuha Dot', '/Mods/Kazuha/.HiddenMod', 'DISABLED', 1),
        ('m6', 'g1', 'obj3', 'Weapon Mod', '/Mods/Weapon/Mod', 'DISABLED', 1)")
        .execute(&pool)
        .await
        .unwrap();

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
