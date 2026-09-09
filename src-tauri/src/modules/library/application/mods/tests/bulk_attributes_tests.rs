use super::*;
use tempfile::TempDir;

// A folder that vanished mid-batch must surface as a failure, not be swallowed:
// its DB flag is already set, so a dropped write error hides a disk/DB mismatch.
#[test]
fn missing_folders_are_reported_not_swallowed() {
    let tmp = TempDir::new().unwrap();
    let present = tmp.path().join("Raiden_Outfit");
    std::fs::create_dir(&present).unwrap();
    let vanished = tmp.path().join("Renamed_By_Concurrent_Toggle");

    let update = info_json::ModInfoUpdate {
        is_favorite: Some(true),
        ..Default::default()
    };

    let result = partition_info_json_writes(
        vec![
            present.to_string_lossy().to_string(),
            vanished.to_string_lossy().to_string(),
        ],
        &update,
    );

    assert_eq!(result.success, vec![present.to_string_lossy().to_string()]);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].path, vanished.to_string_lossy());
    assert!(present.join("info.json").exists());
}

async fn seeded_mod() -> (sqlx::SqlitePool, TempDir, String) {
    let context = crate::test_utils::init_test_db().await;
    let temp = TempDir::new().unwrap();
    let mods_root = temp.path().join("Mods");
    let mod_path = mods_root.join("Alice").join("Blue");
    std::fs::create_dir_all(&mod_path).unwrap();
    crate::test_utils::insert_test_game(
        &context.pool,
        &crate::test_utils::TestGameFixture {
            id: "g1",
            name: "Game",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(mods_root.to_string_lossy().as_ref()),
        },
    )
    .await
    .unwrap();
    crate::test_utils::insert_test_object(
        &context.pool,
        &crate::test_utils::TestObjectFixture {
            id: "o1",
            game_id: "g1",
            name: "Alice",
            folder_path: "Alice",
            object_type: "Character",
        },
    )
    .await
    .unwrap();
    crate::test_utils::insert_test_mod(
        &context.pool,
        &crate::test_utils::TestModFixture {
            id: "m1",
            game_id: "g1",
            object_id: Some("o1"),
            actual_name: "Blue",
            folder_path: "Alice/Blue",
            status: crate::modules::games::domain::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(mods_root.to_string_lossy().as_ref()),
        },
    )
    .await
    .unwrap();
    (context.pool, temp, mod_path.to_string_lossy().to_string())
}

#[tokio::test]
async fn bulk_pin_mirrors_the_flag_to_database_and_info_json() {
    let (pool, _temp, mod_path) = seeded_mod().await;

    let result = bulk_pin(&pool, "g1".to_string(), vec![mod_path.clone()], true)
        .await
        .unwrap();

    assert_eq!(result.success, vec![mod_path.clone()]);
    let pinned: bool = sqlx::query_scalar("SELECT is_pinned FROM mods WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(pinned);
    assert!(
        info_json::read_info_json(Path::new(&mod_path))
            .unwrap()
            .unwrap()
            .is_pinned
    );
}

#[tokio::test]
async fn favorite_file_is_rolled_back_when_database_update_fails() {
    let (pool, _temp, mod_path) = seeded_mod().await;
    info_json::create_default_info_json(Path::new(&mod_path)).unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_favorite BEFORE UPDATE OF is_favorite ON mods BEGIN SELECT RAISE(ABORT, 'injected favorite failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let result = bulk_toggle_favorite(&pool, "g1".to_string(), vec![mod_path.clone()], true).await;

    assert!(result.is_err());
    assert!(
        !info_json::read_info_json(Path::new(&mod_path))
            .unwrap()
            .unwrap()
            .is_favorite
    );
}

#[tokio::test]
async fn bulk_safety_expands_nested_parent_and_supports_unsafe_to_safe_reversal() {
    let context = crate::test_utils::init_test_db().await;
    let temp = TempDir::new().unwrap();
    let mods_root = temp.path().join("Mods");
    let parent = mods_root.join("Alice").join("Variants");
    let blue = parent.join("Blue");
    let red = parent.join("Nested").join("Red");
    std::fs::create_dir_all(&blue).unwrap();
    std::fs::create_dir_all(&red).unwrap();
    crate::test_utils::insert_test_game(
        &context.pool,
        &crate::test_utils::TestGameFixture {
            id: "g-safety",
            name: "Game",
            game_type: crate::modules::games::domain::models::GameType::GIMI,
            path: temp.path().to_string_lossy().as_ref(),
            mods_path: Some(mods_root.to_string_lossy().as_ref()),
        },
    )
    .await
    .unwrap();
    for (id, name, path) in [
        ("blue", "Blue", "Alice/Variants/Blue"),
        ("red", "Red", "Alice/Variants/Nested/Red"),
    ] {
        crate::test_utils::insert_test_mod(
            &context.pool,
            &crate::test_utils::TestModFixture {
                id,
                game_id: "g-safety",
                object_id: None,
                actual_name: name,
                folder_path: path,
                status: crate::modules::games::domain::models::ItemStatus::Enabled,
                is_safe: false,
                object_type: Some("Other"),
                mods_path: Some(mods_root.to_string_lossy().as_ref()),
            },
        )
        .await
        .unwrap();
    }
    let config = crate::modules::settings::application::config::ConfigService::new_for_test_async(
        context.pool.clone(),
    )
    .await;
    let selected = crate::platform::fs::guard::validate_paths(
        &config,
        "g-safety",
        &[parent.to_string_lossy().to_string()],
    )
    .unwrap();

    let initially_resolved = resolve_safety_targets(&context.pool, "g-safety", &selected)
        .await
        .unwrap();
    assert_eq!(initially_resolved.targets.len(), 2);
    bulk_set_safety(&context.pool, "g-safety", initially_resolved, false)
        .await
        .unwrap();
    let resolved = resolve_safety_targets(&context.pool, "g-safety", &selected)
        .await
        .unwrap();
    let result = bulk_set_safety(&context.pool, "g-safety", resolved, true)
        .await
        .unwrap();

    assert_eq!(result.success.len(), 2);
    let rows: Vec<(bool, String)> =
        sqlx::query_as("SELECT is_safe, safety_source FROM mods ORDER BY id")
            .fetch_all(&context.pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        vec![(true, "manual".to_string()), (true, "manual".to_string())]
    );
    assert!(info_json::read_info_json(&blue).unwrap().unwrap().is_safe);
    assert!(info_json::read_info_json(&red).unwrap().unwrap().is_safe);
    assert!(!parent.join("info.json").exists());
}
