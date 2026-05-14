use super::*;
use crate::test_utils::{insert_test_mod, insert_test_object, TestModFixture, TestObjectFixture};
use crate::types::errors::CommandResult;
use sqlx::SqlitePool;
use std::fs;
use tempfile::TempDir;

async fn setup_test_db() -> (TempDir, SqlitePool, String) {
    let tmp = TempDir::new().unwrap();
    let pool = crate::test_utils::init_test_db().await.pool;

    let game_id = "test_game_1".to_string();
    let mods_path = tmp.path().join("Mods");
    fs::create_dir(&mods_path).unwrap();

    crate::test_utils::insert_test_game(
        &pool,
        &crate::test_utils::TestGameFixture {
            id: &game_id,
            name: "Test Game",
            game_type: crate::database::models::GameType::GIMI,
            path: "/",
            mods_path: Some(&mods_path.to_string_lossy()),
        },
    )
    .await
    .unwrap();

    (tmp, pool, game_id)
}

#[tokio::test]
async fn test_get_objects_with_disabled_prefix() -> CommandResult<()> {
    let (tmp, pool, game_id) = setup_test_db().await;
    let mods_path = tmp.path().join("Mods");

    // Create a disabled folder in the mods directory
    let folder_name = "DISABLED MyFallbackMod";
    let mod_dir = mods_path.join(folder_name);
    fs::create_dir(&mod_dir).unwrap();

    // The physical folder exists; now also insert an object row in the DB
    // so that `get_objects_cmd_inner` (which queries the DB) can find it.
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "obj_disabled",
            game_id: &game_id,
            name: "MyFallbackMod",
            folder_path: folder_name,
            object_type: "Other",
        },
    )
    .await
    .unwrap();

    let filter = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };

    let objects = get_objects_cmd_inner(filter, &pool).await?.objects;

    // We expect the object to be indexed
    assert_eq!(objects.len(), 1, "Expected 1 object to be discovered");

    let obj = &objects[0];
    // The visual name should NOT contain the "DISABLED " prefix
    assert_eq!(
        obj.name, "MyFallbackMod",
        "Object name should have the prefix stripped"
    );
    assert_eq!(
        obj.folder_path, "DISABLED MyFallbackMod",
        "Folder path must reflect physical directory"
    );
    assert_eq!(
        obj.is_object_disabled, true,
        "TC-10-xx: Object disabled state should be reflected correctly"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_objects_safe_mode_filtering() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    // Insert an unsafe object manually into DB
    let obj_id = "test_unsafe_obj";
    let mods_path = _tmp.path().join("Mods");
    std::fs::create_dir_all(mods_path.join("NSFW_Mod_Folder")).unwrap();

    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: obj_id,
            game_id: &game_id,
            name: "NSFW_Mod",
            folder_path: "NSFW_Mod_Folder",
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    // 1. Fetch with safe_mode=false (should return 1)
    let filter_unfiltered = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let results_unfiltered = get_objects_cmd_inner(filter_unfiltered, &pool)
        .await?
        .objects;
    assert_eq!(
        results_unfiltered.len(),
        1,
        "Unsafe object should be returned when safe_mode=false"
    );

    // 2. Fetch with safe_mode=true — Phase 1: ALL objects returned, unsafe ones get zeroed counts
    let filter_safe = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: true,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let results_safe = get_objects_cmd_inner(filter_safe, &pool).await?.objects;
    assert_eq!(
        results_safe.len(),
        1,
        "Phase 1: Unsafe object IS returned in safe mode (with zeroed counts)"
    );
    let unsafe_obj = &results_safe[0];
    assert_eq!(
        unsafe_obj.mod_count, 0,
        "Phase 1: mod_count must be zeroed for unsafe objects in safe mode"
    );
    assert_eq!(
        unsafe_obj.enabled_count, 0,
        "Phase 1: enabled_count must be zeroed for unsafe objects in safe mode"
    );

    Ok(())
}

#[tokio::test]
async fn test_create_object_cmd() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    let payload = CreateObjectInput {
        game_id: game_id.clone(),
        name: "New Hero".to_string(),
        folder_path: Some("New Hero Folder".to_string()),
        object_type: "Weapon".to_string(),
        sub_category: None,
        status: None,
        metadata: Some(serde_json::json!({})),
        thumbnail_url: None,
        hash_db: None,
        custom_skins: None,
    };

    let obj_id_result = create_object_cmd_inner(payload, &pool, None).await?;

    // Verify it exists in DB
    let filter = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let objects = get_objects_cmd_inner(filter, &pool).await?.objects;
    assert_eq!(objects.len(), 1, "Created object must be retrievable");
    let result = &objects[0];

    assert_eq!(
        result.name, "New Hero",
        "TC-10-01: Object Name should match"
    );
    assert_eq!(
        result.object_type, "Weapon",
        "TC-10-01: Object type should match"
    );
    assert_eq!(
        result.id, obj_id_result,
        "TC-10-01: Returned ID must match the indexed ID"
    );

    Ok(())
}

#[tokio::test]
async fn test_update_object_cmd() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    let obj_id = "test_obj_update";
    let mods_path = _tmp.path().join("Mods");
    std::fs::create_dir_all(mods_path.join("test_obj_folder")).unwrap();

    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: obj_id,
            game_id: &game_id,
            name: "OldName",
            folder_path: "test_obj_folder",
            object_type: "Other",
        },
    )
    .await
    .unwrap();

    let payload = UpdateObjectInput {
        name: Some("NewName".to_string()),
        object_type: Some("Character".to_string()),
        sub_category: None,
        status: None,
        metadata: Some(serde_json::json!({"test":true})),
        thumbnail_path: None,
        is_auto_sync: None,
        is_pinned: None,
        tags: Some(vec!["Pyro".to_string()]),
        hash_db: None,
        custom_skins: None,
    };

    let _updated = update_object_cmd_inner(obj_id.to_string(), &payload, &pool).await?;

    let filter = ObjectFilter {
        game_id: game_id.clone(),
        search_query: None,
        object_type: None,
        safe_mode: false,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let objects = get_objects_cmd_inner(filter, &pool).await?.objects;
    let updated = objects.into_iter().find(|o| o.id == obj_id).unwrap();

    assert_eq!(updated.name, "NewName", "TC-10-04: Name must be updated");
    assert_eq!(
        updated.object_type, "Character",
        "TC-10-04: Type must be updated"
    );

    Ok(())
}

#[tokio::test]
async fn test_delete_object_fk_constraints() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;
    let trash_dir = _tmp.path().join("trash");
    fs::create_dir(&trash_dir).unwrap();
    let watcher_state = crate::services::scanner::watcher::WatcherState::default();
    let op_lock = crate::services::fs_utils::operation_lock::OperationLock::new();

    // Create an empty object
    let empty_obj_id = "empty_obj";
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: empty_obj_id,
            game_id: &game_id,
            name: "Empty",
            folder_path: "Empty",
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    // Create an object with mods inside it
    let full_obj_id = "full_obj";
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: full_obj_id,
            game_id: &game_id,
            name: "Full",
            folder_path: "Full",
            object_type: "Weapon",
        },
    )
    .await
    .unwrap();

    insert_test_mod(
        &pool,
        &TestModFixture {
            id: "mod1",
            game_id: &game_id,
            object_id: Some(full_obj_id),
            actual_name: "ModName",
            folder_path: "Path",
            status: crate::database::models::ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Weapon"),
            mods_path: Some("C:\\Mods".into()),
        },
    )
    .await
    .unwrap();

    // Delete empty object (should succeed)
    let res_empty = crate::services::objects::mutate::delete_object(
        &pool,
        empty_obj_id,
        false,
        &trash_dir,
        &watcher_state,
        &op_lock,
    )
    .await;
    assert!(
        res_empty.is_ok(),
        "TC-10-07: Empty object should be deleted successfully"
    );

    // Delete object with mods (should now cascade-delete mods and succeed)
    let res_full = crate::services::objects::mutate::delete_object(
        &pool,
        full_obj_id,
        true,
        &trash_dir,
        &watcher_state,
        &op_lock,
    )
    .await;
    assert!(
        res_full.is_ok(),
        "Deleting object with mods should succeed via cascade delete"
    );

    // Verify the mod row was also removed
    let mod_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE object_id = ?")
        .bind(full_obj_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(mod_count, 0, "Child mods should be cascade-deleted");

    Ok(())
}

#[tokio::test]
async fn test_apply_object_match_cmd() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    let obj_id = "test_obj_match";
    let mods_path = _tmp.path().join("Mods");
    std::fs::create_dir_all(mods_path.join("test_obj_match_folder")).unwrap();

    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: obj_id,
            game_id: &game_id,
            name: "PhysicalName",
            folder_path: "test_obj_match_folder",
            object_type: "Other",
        },
    )
    .await
    .unwrap();

    apply_object_match_cmd_inner(
        &ApplyObjectMatchInput {
            game_id: game_id.clone(),
            object_id: Some(obj_id.to_string()),
            folder_path: None,
            matched_entry_key: Some("kazuha".to_string()),
            matched_alias_name: Some("Kazuha".to_string()),
            matched_confidence: Some(0.91),
            matched_reason: Some("Manual deep match".to_string()),
            matched_source: Some("manual_match".to_string()),
        },
        &pool,
    )
    .await?;

    let row = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT name, matched_entry_key, matched_alias_name, matched_source
         FROM objects
         WHERE id = ?",
    )
    .bind(obj_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, "PhysicalName");
    assert_eq!(row.1, "kazuha");
    assert_eq!(row.2, "Kazuha");
    assert_eq!(row.3, "manual_match");

    Ok(())
}

#[tokio::test]
async fn test_object_counts_use_terminal_preview_semantics() -> CommandResult<()> {
    let (tmp, pool, game_id) = setup_test_db().await;
    let mods_path = tmp.path().join("Mods");
    let object_folder = "Albedo";

    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "obj_terminal_counts",
            game_id: &game_id,
            name: "Albedo",
            folder_path: object_folder,
            object_type: "Character",
        },
    )
    .await
    .unwrap();

    let flat_dir = mods_path.join(object_folder).join("FlatSolo");
    fs::create_dir_all(&flat_dir).unwrap();
    fs::write(
        flat_dir.join("flat.ini"),
        "[TextureOverrideFlat]\nhash = 1234\n",
    )
    .unwrap();

    let modpack_dir = mods_path.join(object_folder).join("BigPack");
    fs::create_dir_all(&modpack_dir).unwrap();
    fs::write(
        modpack_dir.join("pack.ini"),
        "[TextureOverridePack]\nhash = 5678\n",
    )
    .unwrap();
    fs::write(modpack_dir.join("asset.dds"), "asset").unwrap();

    let variant_dir = mods_path.join(object_folder).join("SchoolVest");
    for child in ["1.school", "2.no_skirt", "3.no_shirt"] {
        let child_dir = variant_dir.join(child);
        fs::create_dir_all(&child_dir).unwrap();
        fs::write(
            child_dir.join("variant.ini"),
            format!("[TextureOverride{child}]\nhash = 9999\n"),
        )
        .unwrap();
    }
    fs::write(
        variant_dir.join("root.ini"),
        "[TextureOverrideRoot]\nhash = 4242\nfilename = 1.school/file.buf\n",
    )
    .unwrap();

    let disabled_container_dir = mods_path
        .join(object_folder)
        .join("DISABLED DisabledNest")
        .join("InnerLeaf");
    fs::create_dir_all(&disabled_container_dir).unwrap();
    fs::write(
        disabled_container_dir.join("inner.ini"),
        "[TextureOverrideInner]\nhash = 3131\n",
    )
    .unwrap();

    let container_only_dir = mods_path.join(object_folder).join("New folder");
    fs::create_dir_all(&container_only_dir).unwrap();

    let mods_root = mods_path.to_string_lossy().to_string();
    for fixture in [
        TestModFixture {
            id: "flat-row",
            game_id: &game_id,
            object_id: Some("obj_terminal_counts"),
            actual_name: "FlatSolo",
            folder_path: "Albedo/FlatSolo",
            status: crate::database::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_root),
        },
        TestModFixture {
            id: "pack-row",
            game_id: &game_id,
            object_id: Some("obj_terminal_counts"),
            actual_name: "BigPack",
            folder_path: "Albedo/BigPack",
            status: crate::database::models::ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_root),
        },
        TestModFixture {
            id: "variant-root-row",
            game_id: &game_id,
            object_id: Some("obj_terminal_counts"),
            actual_name: "SchoolVest",
            folder_path: "Albedo/SchoolVest",
            status: crate::database::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_root),
        },
        TestModFixture {
            id: "variant-child-row",
            game_id: &game_id,
            object_id: Some("obj_terminal_counts"),
            actual_name: "1.school",
            folder_path: "Albedo/SchoolVest/1.school",
            status: crate::database::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_root),
        },
        TestModFixture {
            id: "disabled-container-row",
            game_id: &game_id,
            object_id: Some("obj_terminal_counts"),
            actual_name: "DISABLED DisabledNest",
            folder_path: "Albedo/DISABLED DisabledNest",
            status: crate::database::models::ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_root),
        },
        TestModFixture {
            id: "disabled-child-row",
            game_id: &game_id,
            object_id: Some("obj_terminal_counts"),
            actual_name: "InnerLeaf",
            folder_path: "Albedo/DISABLED DisabledNest/InnerLeaf",
            status: crate::database::models::ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_root),
        },
        TestModFixture {
            id: "container-only-row",
            game_id: &game_id,
            object_id: Some("obj_terminal_counts"),
            actual_name: "New folder",
            folder_path: "Albedo/New folder",
            status: crate::database::models::ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_root),
        },
    ] {
        insert_test_mod(&pool, &fixture).await.unwrap();
    }

    let filter = ObjectFilter {
        game_id,
        search_query: None,
        object_type: None,
        safe_mode: true,
        meta_filters: None,
        sort_by: None,
        status_filter: None,
    };
    let objects = get_objects_cmd_inner(filter, &pool).await?.objects;
    let object = objects
        .iter()
        .find(|entry| entry.id == "obj_terminal_counts")
        .expect("object to be present");

    assert_eq!(
        object.mod_count, 4,
        "Total count should collapse variant/modpack roots and ignore pure container rows"
    );
    assert_eq!(
        object.enabled_count, 2,
        "Enabled count should exclude disabled-container descendants from active impact"
    );

    Ok(())
}
