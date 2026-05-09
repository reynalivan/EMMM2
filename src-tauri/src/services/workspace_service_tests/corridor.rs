use super::*;

#[tokio::test]
async fn workspace_view_model_filters_explorer_children_by_corridor() {
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
            filter: ObjectFilter {
                safe_mode: true,
                ..build_filter("game_workspace")
            },
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder.clone()),
            selected_mod_path: None,
        },
    )
    .await
    .expect("safe workspace view model");

    assert_eq!(safe_view.objects.len(), 1);
    assert_eq!(safe_view.explorer.children.len(), 1);
    assert_eq!(safe_view.explorer.children[0].display_name, "Safe Outfit");

    let unsafe_view = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: ObjectFilter {
                safe_mode: false,
                ..build_filter("game_workspace")
            },
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder.clone()),
            selected_mod_path: None,
        },
    )
    .await
    .expect("unsafe workspace view model");

    assert_eq!(unsafe_view.objects.len(), 1);
    assert_eq!(unsafe_view.explorer.children.len(), 1);
    assert_eq!(
        unsafe_view.explorer.children[0].display_name,
        "Private Outfit"
    );
}

#[tokio::test]
async fn workspace_view_model_drops_stale_preview_path_from_opposite_corridor() {
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
            filter: ObjectFilter {
                safe_mode: true,
                ..build_filter("game_workspace")
            },
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder.clone()),
            selected_mod_path: Some(unsafe_child.to_string_lossy().to_string()),
        },
    )
    .await
    .expect("workspace view model");

    assert!(view_model.preview.selected_path.is_none());
    assert!(view_model.preview.selected_node.is_none());
    assert!(view_model.selection.selected_mod_path.is_none());
    assert_eq!(
        view_model.selection.reconciliation_status,
        WorkspaceSelectionReconciliationStatus::Cleared
    );
    assert_eq!(
        view_model.selection.reconciliation_reason,
        Some(WorkspaceSelectionReconciliationReason::CorridorMismatch)
    );
    assert_eq!(
        view_model.selection.affected_paths,
        vec![unsafe_child.to_string_lossy().to_string()]
    );
}

#[tokio::test]
async fn workspace_view_model_hides_flat_root_preview_when_self_mod_is_outside_corridor() {
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
            filter: ObjectFilter {
                safe_mode: true,
                ..build_filter("game_workspace")
            },
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder),
            selected_mod_path: None,
        },
    )
    .await
    .expect("workspace view model");

    assert!(view_model.preview.selected_path.is_none());
    assert!(view_model.preview.selected_node.is_none());
}
