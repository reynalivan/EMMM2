use super::*;
use crate::domain::objects::{CreateObjectInput, ObjectFilter, UpdateObjectInput};
use crate::test_utils::{insert_test_mod, insert_test_object, TestModFixture, TestObjectFixture};
type CommandResult<T> = Result<T, crate::domain::errors::AppError>;
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
            game_type: crate::domain::models::GameType::GIMI,
            path: "/",
            mods_path: Some(&mods_path.to_string_lossy()),
        },
    )
    .await
    .unwrap();

    (tmp, pool, game_id)
}

async fn reconcile_test_disk(pool: &SqlitePool, game_id: &str, mods_path: &std::path::Path) {
    crate::services::disk_reconcile::reconcile::reconcile_disk_projection(
        crate::services::disk_reconcile::reconcile::ReconcileDiskProjectionRequest {
            pool,
            game_id,
            mods_path,
            safe_mode_keywords: &[],
            reason: &crate::services::disk_reconcile::types::DiskReconcileReason::InternalMutation,
            changed_paths: &[],
            force_full: true,
            watcher_events: None,
            path_hints: &[],
            progress_reporter: None,
        },
    )
    .await
    .expect("disk projection");
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
        ..Default::default()
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
    assert!(
        obj.is_object_disabled,
        "TC-10-xx: Object disabled state should be reflected correctly"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_objects_returns_the_full_unfiltered_object_list() -> CommandResult<()> {
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

    let filter = ObjectFilter {
        game_id: game_id.clone(),
        ..Default::default()
    };
    let results_unfiltered = get_objects_cmd_inner(filter, &pool).await?.objects;
    assert_eq!(
        results_unfiltered.len(),
        1,
        "Safety classification is a frontend filter and must not hide objects"
    );

    Ok(())
}

#[tokio::test]
async fn test_create_object_cmd() -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;

    let payload = CreateObjectInput {
        status: None,
        game_id: game_id.clone(),
        name: "New Hero".to_string(),
        folder_path: Some("New Hero Folder".to_string()),
        object_type: "Weapon".to_string(),
        sub_category: None,
        metadata: Some(serde_json::json!({})),
        thumbnail_url: None,
        hash_db: None,
        custom_skins: None,
    };

    let obj_id_result =
        crate::services::objects::mutate::create_object_cmd_inner(&pool, None, payload).await?;

    // Verify it exists in DB
    let filter = ObjectFilter {
        game_id: game_id.clone(),
        ..Default::default()
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
        metadata: Some(serde_json::json!({"test":true})),
        thumbnail_path: None,
        is_auto_sync: None,
        is_pinned: None,
        tags: Some(vec!["Pyro".to_string()]),
        hash_db: None,
        custom_skins: None,
    };

    crate::services::objects::mutate::update_object(&pool, obj_id, &payload).await?;

    let filter = ObjectFilter {
        game_id: game_id.clone(),
        ..Default::default()
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
    let mods_path = _tmp.path().join("Mods");
    let watcher_state = crate::services::scanner::watcher::WatcherState::default();
    let op_lock = crate::services::fs_utils::operation_lock::OperationLock::new();
    let op_guard = op_lock.acquire().await.unwrap();

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
            status: crate::domain::models::ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Weapon"),
            mods_path: Some("C:\\Mods"),
        },
    )
    .await
    .unwrap();

    // Delete empty object (should succeed)
    let res_empty = crate::services::objects::mutate::delete_object(
        &pool,
        empty_obj_id,
        false,
        &watcher_state,
        &op_guard,
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
        &watcher_state,
        &op_guard,
    )
    .await;
    assert!(
        res_full.is_ok(),
        "Deleting object with mods should succeed via cascade delete"
    );
    reconcile_test_disk(&pool, &game_id, &mods_path).await;

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
async fn committed_object_mutation_stages_runtime_effects_when_finalization_fails(
) -> CommandResult<()> {
    let (_tmp, pool, game_id) = setup_test_db().await;
    let state = crate::services::disk_reconcile::orchestrator::DiskReconcileState::new();
    insert_test_object(
        &pool,
        &TestObjectFixture {
            id: "object-runtime-effects",
            game_id: &game_id,
            name: "Alice",
            folder_path: "Alice",
            object_type: "Other",
        },
    )
    .await?;

    crate::services::objects::mutate::set_object_and_mods_category(
        &pool,
        &game_id,
        "object-runtime-effects",
        "Character",
    )
    .await?;

    let settlement = crate::services::app::runtime_effects::settle_committed_runtime_effects_with(
        &state,
        &game_id,
        crate::services::disk_reconcile::types::PendingRuntimeEffects {
            collections_dirty: true,
            overlay_refresh: true,
        },
        || async {
            Err(crate::domain::errors::AppError::Io(
                "injected effect failure".to_string(),
            ))
        },
    )
    .await;

    let category: String =
        sqlx::query_scalar("SELECT object_type FROM objects WHERE id = 'object-runtime-effects'")
            .fetch_one(&pool)
            .await?;
    assert_eq!(category, "Character");
    assert_eq!(
        settlement.pending_runtime_effects,
        crate::services::disk_reconcile::types::PendingRuntimeEffects {
            collections_dirty: true,
            overlay_refresh: true,
        }
    );
    assert!(settlement.warning.is_some());
    assert_eq!(settlement.effect_attempts, 2);
    assert_eq!(
        state.stage_runtime_effects(
            &game_id,
            crate::services::disk_reconcile::types::PendingRuntimeEffects::default(),
        ),
        settlement.pending_runtime_effects,
        "the existing reconcile state retains the failed post-commit intent"
    );

    Ok(())
}

#[test]
fn object_command_results_distinguish_committed_mutation_from_sync_warning() {
    let warning = crate::services::disk_reconcile::types::CommittedMutationSyncWarning {
        kind: crate::services::disk_reconcile::types::CommittedMutationSyncWarningKind::ReconcileFailed,
        message: "projection pending".to_string(),
    };
    let created = CreateObjectResult {
        id: "object-1".to_string(),
        sync_warning: Some(warning.clone()),
    };
    let deleted = crate::services::disk_reconcile::types::CommittedMutationResult {
        sync_warning: Some(warning),
    };

    assert_eq!(created.id, "object-1");
    assert!(created.sync_warning.is_some());
    assert!(deleted.sync_warning.is_some());
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
            status: crate::domain::models::ItemStatus::Enabled,
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
            status: crate::domain::models::ItemStatus::Disabled,
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
            status: crate::domain::models::ItemStatus::Enabled,
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
            status: crate::domain::models::ItemStatus::Enabled,
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
            status: crate::domain::models::ItemStatus::Disabled,
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
            status: crate::domain::models::ItemStatus::Enabled,
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
            status: crate::domain::models::ItemStatus::Disabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_root),
        },
    ] {
        insert_test_mod(&pool, &fixture).await.unwrap();
    }

    let filter = ObjectFilter {
        game_id,
        ..Default::default()
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
