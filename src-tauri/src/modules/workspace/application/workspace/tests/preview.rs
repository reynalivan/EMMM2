use super::*;

#[tokio::test]
async fn workspace_view_model_uses_flat_root_as_preview_target() {
    let (pool, _mods_root, mods_path, object_folder) = setup_workspace_fixture("AINOZ").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    fs::create_dir_all(&object_root).expect("object dir");
    write_file(&object_root.join("mod.ini"), "[TextureOverrideTest]\n");

    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: ObjectFilter {
                ..build_filter("game_workspace")
            },
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: None,
            selected_mod_path: None,
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(
        view_model.selection.selected_object_folder_path,
        Some(object_folder.clone())
    );
    assert_eq!(
        view_model.selection.explorer_sub_path,
        Some(object_folder.clone())
    );
    assert_eq!(
        view_model.selection.current_path,
        vec![object_folder.clone()]
    );
    assert_eq!(
        view_model.selection.reconciliation_status,
        WorkspaceSelectionReconciliationStatus::Unchanged
    );
    assert_eq!(view_model.selection.reconciliation_reason, None);
    assert!(view_model.selection.affected_paths.is_empty());
    assert_eq!(
        view_model.runtime.source_state.status,
        WorkspaceSourceStatus::Available
    );
    assert_eq!(view_model.runtime.source_state.message, None);
    assert!(view_model.preview.is_flat_mod_root);
    assert_eq!(
        view_model.preview.selected_path,
        Some(object_root.to_string_lossy().to_string())
    );
    assert_eq!(
        view_model
            .preview
            .selected_node
            .as_ref()
            .and_then(|node| match node {
                WorkspaceNode::Explorer(explorer) => Some(explorer.path.clone()),
                WorkspaceNode::Object(_) => None,
            }),
        Some(object_root.to_string_lossy().to_string())
    );
    assert_eq!(
        view_model
            .preview
            .selected_node
            .as_ref()
            .and_then(|node| match node {
                WorkspaceNode::Explorer(explorer) => Some(explorer.display_mode),
                WorkspaceNode::Object(_) => None,
            }),
        Some(WorkspaceDisplayMode::FlatMod)
    );
    assert_eq!(
        view_model
            .preview
            .mod_info_summary
            .as_ref()
            .map(|summary| summary.actual_name.clone()),
        Some(object_folder)
    );
    assert!(view_model.preview.ini_summary.is_none());
    assert!(view_model.preview.image_summary.is_none());
    assert_eq!(
        view_model.objects.first().map(|object| object.node_kind),
        Some(WorkspaceNodeKind::Object)
    );
    assert_eq!(
        view_model.objects.first().map(|object| object.display_mode),
        Some(WorkspaceDisplayMode::Unknown)
    );
    assert_eq!(
        view_model
            .objects
            .first()
            .and_then(|object| object.type_chip),
        None
    );
}

#[tokio::test]
async fn workspace_view_model_collapses_nested_selected_path_under_flat_root() {
    let (pool, _mods_root, mods_path, object_folder) = setup_workspace_fixture("ALBEDO").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    fs::create_dir_all(object_root.join("Textures")).expect("nested dir");
    write_file(
        &object_root.join("mod.ini"),
        "[TextureOverrideTest]\nfilename = Textures/example.dds\n",
    );
    write_file(&object_root.join("Textures").join("example.dds"), "texture");

    let nested_selection = object_root.join("Textures").join("example.dds");
    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: ObjectFilter {
                ..build_filter("game_workspace")
            },
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(object_folder.clone()),
            selected_mod_path: Some(nested_selection.to_string_lossy().to_string()),
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(
        view_model.preview.selected_path,
        Some(object_root.to_string_lossy().to_string())
    );
    assert_eq!(
        view_model
            .preview
            .selected_node
            .as_ref()
            .and_then(|node| match node {
                WorkspaceNode::Explorer(explorer) => Some(explorer.path.clone()),
                WorkspaceNode::Object(_) => None,
            }),
        Some(object_root.to_string_lossy().to_string())
    );
}

#[tokio::test]
async fn workspace_view_model_builds_nested_current_path_from_object_root() {
    let (pool, _mods_root, mods_path, object_folder) = setup_workspace_fixture("AMBERCN").await;
    let nested_path = std::path::Path::new(&mods_path)
        .join(&object_folder)
        .join("Variants")
        .join("School");
    fs::create_dir_all(&nested_path).expect("nested path");

    let explorer_sub_path = format!("{object_folder}/Variants/School");
    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: build_filter("game_workspace"),
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(explorer_sub_path.clone()),
            selected_mod_path: None,
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(
        view_model.selection.current_path,
        vec![
            object_folder.clone(),
            "Variants".to_string(),
            "School".to_string()
        ]
    );
    assert_eq!(
        view_model.selection.explorer_sub_path,
        Some(explorer_sub_path)
    );
}

#[tokio::test]
async fn workspace_view_model_marks_disabled_ancestor_children_as_inactive_branch() {
    let (pool, _mods_root, mods_path, object_folder) = setup_workspace_fixture("ALHAITHAM").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    let disabled_parent = object_root.join("DISABLED Variants");
    let variant = disabled_parent.join("School");
    fs::create_dir_all(&variant).expect("variant path");
    write_file(&variant.join("mod.ini"), "[TextureOverrideTest]\n");

    let explorer_sub_path = format!("{object_folder}/DISABLED Variants");
    let view_model = get_workspace_view_model(
        &pool,
        WorkspaceViewModelInput {
            filter: ObjectFilter {
                ..build_filter("game_workspace")
            },
            selected_object_folder_path: Some(object_folder.clone()),
            explorer_sub_path: Some(explorer_sub_path),
            selected_mod_path: Some(variant.to_string_lossy().to_string()),
        },
    )
    .await
    .expect("workspace view model");

    assert_eq!(
        view_model
            .explorer
            .inactive_reason
            .as_ref()
            .map(|reason| reason.code),
        Some(WorkspaceReasonCode::DisabledByContainer)
    );
    assert_eq!(
        view_model
            .explorer
            .inactive_reason
            .as_ref()
            .and_then(|reason| reason.args.get("container_name").cloned()),
        Some("Variants".to_string())
    );
    assert_eq!(
        view_model
            .preview
            .selected_node
            .as_ref()
            .and_then(|node| match node {
                WorkspaceNode::Explorer(explorer) => Some(explorer.node_kind),
                WorkspaceNode::Object(_) => None,
            }),
        Some(WorkspaceNodeKind::InactiveBranch)
    );
    assert_eq!(
        view_model
            .preview
            .warning_summary
            .messages
            .first()
            .map(|warning| warning.code),
        Some(WorkspaceWarningCode::InactiveReason)
    );
    assert_eq!(
        view_model
            .preview
            .warning_summary
            .messages
            .first()
            .and_then(|warning| warning.args.get("container_name").cloned()),
        Some("Variants".to_string())
    );
}

#[tokio::test]
async fn split_preview_keeps_selection_unchanged_when_the_structure_context_is_stale() {
    let (pool, _mods_root, mods_path, object_folder) =
        setup_workspace_fixture("STALE_PREVIEW").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    fs::create_dir_all(&object_root).expect("object root");

    let result = get_workspace_preview(
        &pool,
        WorkspacePreviewInput {
            game_id: "game_workspace".to_string(),
            explorer_sub_path: Some(format!("{object_folder}/Deleted")),
            selected_mod_path: Some(object_root.join("Deleted").to_string_lossy().to_string()),
        },
    )
    .await
    .expect("preview result");

    assert_eq!(
        result.context_status,
        WorkspacePreviewContextStatus::ContextStale
    );
    assert_eq!(
        result.selection.selected_mod_path,
        Some(object_root.join("Deleted").to_string_lossy().to_string())
    );
    assert_eq!(
        result.selection.reconciliation_status,
        WorkspaceSelectionReconciliationStatus::Unchanged
    );
}

#[tokio::test]
async fn split_preview_rejects_paths_outside_the_mods_root() {
    let (pool, _mods_root, _mods_path, _object_folder) =
        setup_workspace_fixture("ESCAPE_PREVIEW").await;
    let outside = tempfile::tempdir().expect("outside dir");

    let result = get_workspace_preview(
        &pool,
        WorkspacePreviewInput {
            game_id: "game_workspace".to_string(),
            explorer_sub_path: None,
            selected_mod_path: Some(outside.path().to_string_lossy().to_string()),
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(crate::shared::errors::AppError::Security(_))
    ));
}

#[tokio::test]
async fn split_preview_keeps_an_empty_root_selection_empty() {
    let (pool, _mods_root, _mods_path, _object_folder) =
        setup_workspace_fixture("EMPTY_ROOT_PREVIEW").await;

    let result = get_workspace_preview(
        &pool,
        WorkspacePreviewInput {
            game_id: "game_workspace".to_string(),
            explorer_sub_path: None,
            selected_mod_path: None,
        },
    )
    .await
    .expect("preview result");

    assert_eq!(result.context_status, WorkspacePreviewContextStatus::Ready);
    assert_eq!(result.preview.selected_path, None);
    assert_eq!(result.selection.selected_mod_path, None);
    assert_eq!(
        result.selection.reconciliation_status,
        WorkspaceSelectionReconciliationStatus::Unchanged
    );
}

#[tokio::test]
async fn split_preview_clears_a_missing_target_inside_a_flat_mod() {
    let (pool, _mods_root, mods_path, object_folder) =
        setup_workspace_fixture("MISSING_FLAT_TARGET").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    fs::create_dir_all(&object_root).expect("object root");
    write_file(&object_root.join("mod.ini"), "[TextureOverrideTest]\\n");
    let missing_target = object_root.join("Missing Variant");

    let result = get_workspace_preview(
        &pool,
        WorkspacePreviewInput {
            game_id: "game_workspace".to_string(),
            explorer_sub_path: Some(object_folder),
            selected_mod_path: Some(missing_target.to_string_lossy().to_string()),
        },
    )
    .await
    .expect("preview result");

    assert_eq!(result.context_status, WorkspacePreviewContextStatus::Ready);
    assert_eq!(result.preview.selected_path, None);
    assert_eq!(result.selection.selected_mod_path, None);
    assert_eq!(
        result.selection.reconciliation_status,
        WorkspaceSelectionReconciliationStatus::Cleared
    );
    assert_eq!(
        result.selection.reconciliation_reason,
        Some(WorkspaceSelectionReconciliationReason::MissingModPath)
    );
}

#[tokio::test]
async fn split_preview_keeps_a_direct_container_child_under_a_mod_root() {
    let (pool, _mods_root, mods_path, object_folder) =
        setup_workspace_fixture("CONTAINER_CHILD_PREVIEW").await;
    let object_root = std::path::Path::new(&mods_path).join(&object_folder);
    let presets = object_root.join("Presets");
    fs::create_dir_all(&presets).expect("presets folder");
    write_file(&object_root.join("mod.ini"), "[TextureOverrideTest]\\n");

    let result = get_workspace_preview(
        &pool,
        WorkspacePreviewInput {
            game_id: "game_workspace".to_string(),
            explorer_sub_path: Some(object_folder),
            selected_mod_path: Some(presets.to_string_lossy().to_string()),
        },
    )
    .await
    .expect("preview result");

    assert_eq!(
        result.preview.selected_path,
        Some(presets.to_string_lossy().to_string())
    );
    assert_eq!(
        result
            .preview
            .selected_node
            .as_ref()
            .and_then(|node| match node {
                WorkspaceNode::Explorer(explorer) => Some(explorer.node_type.as_str()),
                WorkspaceNode::Object(_) => None,
            }),
        Some("ContainerFolder")
    );
}
