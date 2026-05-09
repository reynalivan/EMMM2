use super::*;

#[tokio::test]
async fn workspace_view_model_clears_selection_when_db_object_path_is_missing_on_disk() {
    let (pool, _mods_root, _mods_path, object_folder) =
        setup_workspace_fixture("STALE_OBJECT").await;

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: build_filter("game_workspace"),
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: None,
            selected_mod_path: None,
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(view_model.objects.len(), 1);
    assert!(view_model.selection.selected_object_folder_path.is_none());
    assert!(view_model.selection.explorer_sub_path.is_none());
    assert!(view_model.selection.selected_mod_path.is_none());
    assert!(view_model.selection.current_path.is_empty());
    assert!(view_model.preview.selected_node.is_none());
    assert_eq!(
        view_model.selection.reconciliation_status,
        WorkspaceSelectionReconciliationStatus::Cleared
    );
    assert_eq!(
        view_model.selection.reconciliation_reason,
        Some(WorkspaceSelectionReconciliationReason::MissingObjectRoot)
    );
    assert_eq!(view_model.selection.affected_paths, vec![object_folder]);
    let stale_object = view_model.objects.first().expect("stale object row");
    assert_eq!(
        stale_object.switch_policy_key,
        WorkspaceSwitchPolicyKey::Blocked
    );
    assert!(!stale_object.capabilities.can_toggle);
    assert!(!stale_object.capabilities.can_open_in_explorer);
    assert_eq!(
        stale_object
            .primary_warning
            .as_ref()
            .map(|warning| warning.code),
        Some(WorkspaceWarningCode::FolderWarning)
    );
}

#[tokio::test]
async fn workspace_view_model_falls_back_to_object_root_when_nested_grid_path_is_missing() {
    let (pool, _mods_root, mods_path, object_folder) = setup_workspace_fixture("STALE_GRID").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    fs::create_dir_all(&object_root).expect("object root");

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: build_filter("game_workspace"),
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(format!("{object_folder}/Deleted Variant")),
            selected_mod_path: Some(
                object_root
                    .join("Deleted Variant")
                    .to_string_lossy()
                    .to_string(),
            ),
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(
        view_model.selection.selected_object_folder_path,
        Some(object_folder.clone())
    );
    assert_eq!(view_model.selection.explorer_sub_path, Some(object_folder));
    assert!(view_model.selection.selected_mod_path.is_none());
    assert_eq!(view_model.selection.current_path.len(), 1);
    assert!(view_model.preview.selected_node.is_none());
    assert_eq!(
        view_model.selection.reconciliation_status,
        WorkspaceSelectionReconciliationStatus::Fallback
    );
    assert_eq!(
        view_model.selection.reconciliation_reason,
        Some(WorkspaceSelectionReconciliationReason::MissingExplorerPath)
    );
    assert_eq!(
        view_model.selection.affected_paths,
        vec![
            format!("STALE_GRID/Deleted Variant"),
            object_root
                .join("Deleted Variant")
                .to_string_lossy()
                .to_string(),
        ]
    );
}

#[tokio::test]
async fn workspace_view_model_reports_unavailable_source_without_traversing_missing_root() {
    let (pool, mods_root, mods_path, object_folder) =
        setup_workspace_fixture("MISSING_SOURCE").await;
    drop(mods_root);

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: build_filter("game_workspace"),
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder.clone()),
            selected_mod_path: Some(
                std::path::Path::new(&mods_path)
                    .join(&object_folder)
                    .to_string_lossy()
                    .to_string(),
            ),
        },
    )
    .await
    .expect("workspace view model");

    assert!(view_model.explorer.children.is_empty());
    assert!(view_model.preview.selected_node.is_none());
    assert_eq!(
        view_model.selection.reconciliation_status,
        WorkspaceSelectionReconciliationStatus::Cleared
    );
    assert_eq!(
        view_model.selection.reconciliation_reason,
        Some(WorkspaceSelectionReconciliationReason::SourceUnavailable)
    );
    assert_eq!(
        view_model.runtime.source_state.status,
        WorkspaceSourceStatus::Unavailable
    );
    assert!(view_model.runtime.source_state.message.is_some());
}
