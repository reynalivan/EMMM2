//! Workspace behavior before the view-only safety filter is applied in the UI.

use super::*;

#[tokio::test]
async fn workspace_view_model_returns_all_explorer_children_for_frontend_filtering() {
    let (pool, _mods_root, mods_path, object_folder) = setup_workspace_fixture("KEQING").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    let safe_child = object_root.join("Safe Outfit");
    let unsafe_child = object_root.join("Private Outfit");
    fs::create_dir_all(&safe_child).expect("safe child");
    fs::create_dir_all(&unsafe_child).expect("unsafe child");
    write_file(
        &unsafe_child.join("info.json"),
        r#"{"actual_name":"Private Outfit","is_safe":false}"#,
    );

    let safe_view = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: build_filter("game_workspace"),
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder.clone()),
            selected_mod_path: None,
        },
    )
    .await
    .expect("safe workspace view model");

    assert_eq!(safe_view.objects.len(), 1);
    assert_eq!(safe_view.explorer.children.len(), 2);

    let unsafe_view = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: build_filter("game_workspace"),
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder.clone()),
            selected_mod_path: None,
        },
    )
    .await
    .expect("unsafe workspace view model");

    assert_eq!(unsafe_view.objects.len(), 1);
    assert_eq!(unsafe_view.explorer.children.len(), 2);
}

#[tokio::test]
async fn shallow_listing_returns_known_and_unknown_folders_for_frontend_filtering() {
    let (pool, _mods_root, mods_path, object_folder) =
        setup_workspace_fixture("SHALLOW_SAFE").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    let safe_child = object_root.join("Safe Outfit");
    let unsafe_child = object_root.join("Private Outfit");
    let unknown_child = object_root.join("Unclassified Outfit");
    fs::create_dir_all(&safe_child).expect("safe child");
    fs::create_dir_all(&unsafe_child).expect("unsafe child");
    fs::create_dir_all(&unknown_child).expect("unknown child");

    insert_test_mod(
        &pool,
        &TestModFixture {
            id: "safe-mod",
            game_id: "game_workspace",
            object_id: Some("obj_workspace"),
            actual_name: "Safe Outfit",
            folder_path: "SHALLOW_SAFE/Safe Outfit",
            status: ItemStatus::Enabled,
            is_safe: true,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert safe mod");
    insert_test_mod(
        &pool,
        &TestModFixture {
            id: "unsafe-mod",
            game_id: "game_workspace",
            object_id: Some("obj_workspace"),
            actual_name: "Private Outfit",
            folder_path: "SHALLOW_SAFE/Private Outfit",
            status: ItemStatus::Enabled,
            is_safe: false,
            object_type: Some("Character"),
            mods_path: Some(&mods_path),
        },
    )
    .await
    .expect("insert unsafe mod");

    let view_model = get_workspace_view_model_with_listing_mode(
        &pool,
        WorkspaceViewModelInput {
            filter: build_filter("game_workspace"),
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder),
            selected_mod_path: None,
        },
        true,
    )
    .await
    .expect("shallow safe workspace view model");

    assert_eq!(view_model.explorer.children.len(), 3);
}

#[tokio::test]
async fn workspace_view_model_keeps_preview_selection_independent_of_safety() {
    let (pool, _mods_root, mods_path, object_folder) = setup_workspace_fixture("NAHIDA").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    let safe_child = object_root.join("Safe Outfit");
    let unsafe_child = object_root.join("Private Outfit");
    fs::create_dir_all(&safe_child).expect("safe child");
    fs::create_dir_all(&unsafe_child).expect("unsafe child");
    write_file(
        &unsafe_child.join("info.json"),
        r#"{"actual_name":"Private Outfit","is_safe":false}"#,
    );

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: build_filter("game_workspace"),
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder.clone()),
            selected_mod_path: Some(unsafe_child.to_string_lossy().to_string()),
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(
        view_model.preview.selected_path.as_deref(),
        Some(unsafe_child.to_string_lossy().as_ref())
    );
    assert!(view_model.preview.selected_node.is_some());
    assert_eq!(
        view_model.selection.selected_mod_path.as_deref(),
        Some(unsafe_child.to_string_lossy().as_ref())
    );
    assert_eq!(
        view_model.selection.reconciliation_status,
        WorkspaceSelectionReconciliationStatus::Unchanged
    );
    assert!(view_model.selection.reconciliation_reason.is_none());
    assert!(view_model.selection.affected_paths.is_empty());
}

#[tokio::test]
async fn workspace_view_model_shows_flat_root_preview_independent_of_safety() {
    let (pool, _mods_root, mods_path, object_folder) = setup_workspace_fixture("FURINA").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    fs::create_dir_all(&object_root).expect("object dir");
    write_file(&object_root.join("mod.ini"), "[TextureOverrideTest]\n");
    write_file(
        &object_root.join("info.json"),
        r#"{"actual_name":"Furina Private","is_safe":false}"#,
    );

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: build_filter("game_workspace"),
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder),
            selected_mod_path: None,
        },
    )
    .await
    .expect("workspace view model");

    assert!(view_model.preview.selected_path.is_some());
    assert!(view_model.preview.selected_node.is_some());
}
