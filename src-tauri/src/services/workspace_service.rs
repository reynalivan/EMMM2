use std::path::Path;

use crate::commands::mods::preview_cmds::{
    list_mod_ini_files_inner, list_mod_preview_images_inner,
};
use crate::domain::workspace::{
    WorkspaceCapabilities, WorkspaceDisplayMode, WorkspaceExplorer, WorkspaceExplorerNode,
    WorkspaceImageSummary, WorkspaceIniSummary, WorkspaceModInfoSummary, WorkspaceNode,
    WorkspaceNodeKind, WorkspaceObjectNode, WorkspacePreview, WorkspaceReason, WorkspaceReasonCode,
    WorkspaceRuntime, WorkspaceSelection, WorkspaceSwitchPolicyKey, WorkspaceSwitchState,
    WorkspaceTypeChip, WorkspaceViewModel, WorkspaceViewModelInput, WorkspaceWarning,
    WorkspaceWarningCode, WorkspaceWarningState, WorkspaceWarningSummary,
};
use crate::services::explorer::classifier::NodeType;
use crate::services::explorer::helpers::apply_runtime_corridor_filter_to_response;
use crate::services::explorer::listing::{build_mod_folder_from_path, list_mod_folders_for_game};
use crate::services::explorer::types::{FolderGridResponse, ModFolder};
use crate::services::mods::info_json::{read_info_json, ModInfo};
use crate::services::objects::query::get_filtered_objects_with_conflict_check;
use crate::services::path_key::{
    names_equal_by_key, path_starts_with_key, strip_path_prefix_preserve_display,
};

async fn load_game_mods_path(pool: &sqlx::SqlitePool, game_id: &str) -> Result<String, String> {
    sqlx::query_scalar::<_, String>("SELECT mods_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Game '{}' has no mods_path", game_id))
}

fn resolve_explorer_sub_path(input: &WorkspaceViewModelInput) -> Option<String> {
    if let Some(sub_path) = input.explorer_sub_path.as_deref() {
        let trimmed = sub_path.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    input
        .selected_object_folder_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn build_current_path(
    selected_object_folder_path: Option<&str>,
    explorer_sub_path: Option<&str>,
) -> Vec<String> {
    let Some(sub_path) = explorer_sub_path else {
        return Vec::new();
    };

    if let Some(object_path) = selected_object_folder_path {
        let root_name = Path::new(object_path)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| object_path.to_string());
        let mut path = vec![root_name];
        if let Some(relative) = strip_path_prefix_preserve_display(sub_path, object_path, None) {
            path.extend(
                relative
                    .split('/')
                    .filter(|segment| !segment.trim().is_empty())
                    .map(str::to_string),
            );
        }
        return path;
    }

    sub_path
        .split(['/', '\\'])
        .filter(|segment| !segment.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn build_workspace_args(entries: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn build_disabled_by_container_reason(
    ancestor_disabled_by: Option<&str>,
) -> Option<WorkspaceReason> {
    ancestor_disabled_by.map(|value| WorkspaceReason {
        code: WorkspaceReasonCode::DisabledByContainer,
        args: build_workspace_args(&[("container_name", value)]),
    })
}

fn build_object_inactive_reason(
    object: &crate::repo::object_repo::ObjectSummary,
) -> Option<WorkspaceReason> {
    if object.is_object_disabled {
        return Some(WorkspaceReason {
            code: WorkspaceReasonCode::ObjectFolderDisabled,
            args: build_workspace_args(&[]),
        });
    }

    None
}

fn build_inactive_warning(reason: &WorkspaceReason) -> WorkspaceWarning {
    let mut args = reason.args.clone();
    args.insert(
        "reason_code".to_string(),
        match reason.code {
            WorkspaceReasonCode::DisabledByContainer => "disabled_by_container".to_string(),
            WorkspaceReasonCode::ObjectFolderDisabled => "object_folder_disabled".to_string(),
        },
    );

    WorkspaceWarning {
        code: WorkspaceWarningCode::InactiveReason,
        args,
        state: WorkspaceWarningState::Warning,
    }
}

fn build_folder_warning(message: &str) -> WorkspaceWarning {
    WorkspaceWarning {
        code: WorkspaceWarningCode::FolderWarning,
        args: build_workspace_args(&[("message", message)]),
        state: WorkspaceWarningState::Warning,
    }
}

fn build_naming_conflict_warning() -> WorkspaceWarning {
    WorkspaceWarning {
        code: WorkspaceWarningCode::NamingConflict,
        args: build_workspace_args(&[]),
        state: WorkspaceWarningState::Warning,
    }
}

fn map_display_mode(node_type: &str) -> WorkspaceDisplayMode {
    match node_type {
        "ContainerFolder" => WorkspaceDisplayMode::ContainerFolder,
        "ModPackRoot" => WorkspaceDisplayMode::ModPack,
        "VariantContainer" => WorkspaceDisplayMode::Variant,
        "FlatModRoot" => WorkspaceDisplayMode::FlatMod,
        "InternalAssets" => WorkspaceDisplayMode::InternalAssets,
        _ => WorkspaceDisplayMode::Unknown,
    }
}

fn map_type_chip(display_mode: WorkspaceDisplayMode) -> Option<WorkspaceTypeChip> {
    match display_mode {
        WorkspaceDisplayMode::ModPack => Some(WorkspaceTypeChip::ModPack),
        WorkspaceDisplayMode::Variant => Some(WorkspaceTypeChip::Variant),
        WorkspaceDisplayMode::FlatMod => Some(WorkspaceTypeChip::FlatMod),
        WorkspaceDisplayMode::ContainerFolder
        | WorkspaceDisplayMode::InternalAssets
        | WorkspaceDisplayMode::Unknown => None,
    }
}

fn map_node_kind(node_type: &str, ancestor_disabled: bool) -> WorkspaceNodeKind {
    if ancestor_disabled {
        return WorkspaceNodeKind::InactiveBranch;
    }

    if node_type == NodeType::ContainerFolder.as_str() {
        return WorkspaceNodeKind::Container;
    }

    WorkspaceNodeKind::TerminalMod
}

fn map_warning_state(
    warnings: &[String],
    inactive_reason: Option<&WorkspaceReason>,
    has_primary_warning: bool,
) -> WorkspaceWarningState {
    if warnings.is_empty() && inactive_reason.is_none() && !has_primary_warning {
        return WorkspaceWarningState::None;
    }

    WorkspaceWarningState::Warning
}

fn build_folder_capabilities(
    folder: &ModFolder,
    node_kind: WorkspaceNodeKind,
    display_mode: WorkspaceDisplayMode,
) -> WorkspaceCapabilities {
    let is_internal_assets = display_mode == WorkspaceDisplayMode::InternalAssets;
    let is_terminal_node = node_kind == WorkspaceNodeKind::TerminalMod;
    let can_move = is_terminal_node && !is_internal_assets;

    WorkspaceCapabilities {
        can_toggle: !is_internal_assets,
        can_rename: !is_internal_assets,
        can_delete: !is_internal_assets,
        can_move,
        can_toggle_safe: is_terminal_node,
        can_sync: is_terminal_node,
        can_enable_only_this: is_terminal_node && !folder.is_enabled,
        can_pin: can_move,
        can_edit_metadata: false,
        can_reveal_in_explorer: !is_internal_assets,
        can_move_category: false,
        can_open_in_explorer: !is_internal_assets,
    }
}

fn map_folder_switch_state(folder: &ModFolder, ancestor_disabled: bool) -> WorkspaceSwitchState {
    if ancestor_disabled {
        return WorkspaceSwitchState::BlockedByAncestor;
    }

    if folder.is_enabled {
        return WorkspaceSwitchState::Enabled;
    }

    WorkspaceSwitchState::Disabled
}

fn map_folder_switch_policy_key(ancestor_disabled: bool) -> WorkspaceSwitchPolicyKey {
    if ancestor_disabled {
        return WorkspaceSwitchPolicyKey::Blocked;
    }

    WorkspaceSwitchPolicyKey::Mod
}

fn map_workspace_node(
    folder: ModFolder,
    ancestor_disabled_by: Option<&str>,
) -> WorkspaceExplorerNode {
    let node_type = folder.node_type.clone();
    let display_name = folder.name.clone();
    let inactive_reason = build_disabled_by_container_reason(ancestor_disabled_by);
    let ancestor_disabled = inactive_reason.is_some();
    let display_mode = map_display_mode(&node_type);
    let node_kind = map_node_kind(&folder.node_type, ancestor_disabled);
    let primary_warning = folder
        .warnings
        .first()
        .map(|message| build_folder_warning(message))
        .or_else(|| inactive_reason.as_ref().map(build_inactive_warning));
    let warning_state = map_warning_state(
        &folder.warnings,
        inactive_reason.as_ref(),
        primary_warning.is_some(),
    );
    let switch_reason = inactive_reason.clone();
    let switch_state = map_folder_switch_state(&folder, ancestor_disabled);
    let switch_policy_key = map_folder_switch_policy_key(ancestor_disabled);
    let capabilities = build_folder_capabilities(&folder, node_kind, display_mode);

    WorkspaceExplorerNode {
        node_type,
        classification_reasons: folder.classification_reasons,
        id: folder.id,
        owner_object_id: folder.owner_object_id,
        owner_object_folder_path: folder.owner_object_folder_path,
        name: display_name.clone(),
        folder_name: folder.folder_name,
        path: folder.path,
        is_enabled: folder.is_enabled,
        is_directory: folder.is_directory,
        thumbnail_path: folder.thumbnail_path,
        modified_at: folder.modified_at,
        size_bytes: folder.size_bytes,
        has_info_json: folder.has_info_json,
        is_favorite: folder.is_favorite,
        is_misplaced: folder.is_misplaced,
        is_safe: folder.is_safe,
        metadata: folder.metadata,
        category: folder.category,
        conflict_group_id: folder.conflict_group_id,
        conflict_state: folder.conflict_state,
        warnings: folder.warnings,
        node_kind,
        display_mode,
        type_chip: map_type_chip(display_mode),
        display_name,
        is_effectively_active: folder.is_enabled && !ancestor_disabled,
        ancestor_disabled,
        inactive_reason,
        warning_state,
        primary_warning,
        switch_state,
        switch_reason,
        switch_policy_key,
        capabilities,
        can_navigate: folder.node_type == NodeType::ContainerFolder.as_str(),
    }
}

fn map_workspace_explorer(explorer: FolderGridResponse) -> WorkspaceExplorer {
    let inactive_reason =
        build_disabled_by_container_reason(explorer.ancestor_disabled_by.as_deref());
    let self_display_mode = explorer
        .self_node_type
        .as_deref()
        .map(map_display_mode)
        .unwrap_or(WorkspaceDisplayMode::Unknown);
    let self_ancestor_disabled = inactive_reason.is_some();

    WorkspaceExplorer {
        self_node_type: explorer.self_node_type.clone(),
        self_node_kind: explorer
            .self_node_type
            .as_deref()
            .map(|node_type| map_node_kind(node_type, self_ancestor_disabled))
            .unwrap_or_else(|| map_node_kind("", self_ancestor_disabled)),
        self_display_mode,
        self_type_chip: map_type_chip(self_display_mode),
        self_is_mod: explorer.self_is_mod,
        self_is_enabled: explorer.self_is_enabled,
        self_is_effectively_active: explorer.self_is_enabled && !self_ancestor_disabled,
        self_owner_object_id: explorer.self_owner_object_id,
        self_owner_object_folder_path: explorer.self_owner_object_folder_path,
        self_classification_reasons: explorer.self_classification_reasons,
        children: explorer
            .children
            .into_iter()
            .map(|folder| map_workspace_node(folder, explorer.ancestor_disabled_by.as_deref()))
            .collect(),
        conflicts: explorer.conflicts,
        ancestor_disabled_by: explorer.ancestor_disabled_by,
        ancestor_disabled_path: explorer.ancestor_disabled_path,
        inactive_reason,
    }
}

fn map_object_switch_state(
    object: &crate::repo::object_repo::ObjectSummary,
    _inactive_reason: Option<&WorkspaceReason>,
) -> WorkspaceSwitchState {
    if object.is_object_disabled {
        return WorkspaceSwitchState::Disabled;
    }

    WorkspaceSwitchState::Enabled
}

fn map_object_switch_policy_key(
    object: &crate::repo::object_repo::ObjectSummary,
) -> WorkspaceSwitchPolicyKey {
    if object.mod_count <= 0 {
        return WorkspaceSwitchPolicyKey::Blocked;
    }

    WorkspaceSwitchPolicyKey::Object
}

fn map_workspace_object(object: crate::repo::object_repo::ObjectSummary) -> WorkspaceObjectNode {
    let inactive_reason = build_object_inactive_reason(&object);
    let primary_warning = if object.has_naming_conflict {
        Some(build_naming_conflict_warning())
    } else {
        inactive_reason.as_ref().map(build_inactive_warning)
    };
    let warning_state = if primary_warning.is_some() {
        WorkspaceWarningState::Warning
    } else {
        WorkspaceWarningState::None
    };
    let capabilities = WorkspaceCapabilities {
        can_toggle: object.mod_count > 0,
        can_rename: true,
        can_delete: true,
        can_move: false,
        can_toggle_safe: false,
        can_sync: true,
        can_enable_only_this: false,
        can_pin: true,
        can_edit_metadata: true,
        can_reveal_in_explorer: true,
        can_move_category: true,
        can_open_in_explorer: true,
    };
    let switch_reason = inactive_reason.clone();
    let switch_state = map_object_switch_state(&object, inactive_reason.as_ref());
    let switch_policy_key = map_object_switch_policy_key(&object);

    WorkspaceObjectNode {
        display_name: object.name.clone(),
        is_effectively_active: !object.is_object_disabled,
        inactive_reason,
        warning_state,
        primary_warning,
        switch_state,
        switch_reason,
        switch_policy_key,
        node_kind: WorkspaceNodeKind::Object,
        display_mode: WorkspaceDisplayMode::Unknown,
        type_chip: None,
        capabilities,
        object,
    }
}

fn resolve_preview_target_path(
    selected_mod_path: Option<&str>,
    self_mod_path: Option<&str>,
    children: &[WorkspaceExplorerNode],
) -> Option<String> {
    let Some(external_selected_path) = selected_mod_path else {
        return self_mod_path.map(str::to_string);
    };
    let selected_child = children
        .iter()
        .find(|folder| names_equal_by_key(&folder.path, external_selected_path));

    if let Some(child) = selected_child {
        if let Some(self_path) = self_mod_path {
            if child.node_type == NodeType::ContainerFolder.as_str() {
                return Some(external_selected_path.to_string());
            }
            return Some(self_path.to_string());
        }

        return Some(external_selected_path.to_string());
    }

    let Some(self_path) = self_mod_path else {
        return None;
    };

    if names_equal_by_key(external_selected_path, self_path) {
        return Some(self_path.to_string());
    }

    if path_starts_with_key(external_selected_path, self_path, None) {
        return Some(self_path.to_string());
    }

    None
}

fn resolve_self_mod_path(
    mods_path: &str,
    explorer_sub_path: Option<&str>,
    explorer: &WorkspaceExplorer,
    safe_mode: bool,
) -> Option<String> {
    if !explorer.self_is_mod {
        return None;
    }

    let sub_path = explorer_sub_path?;
    let self_path = Path::new(mods_path).join(sub_path);
    let folder = build_mod_folder_from_path(&self_path, explorer_sub_path)?;
    if folder.is_safe != safe_mode {
        return None;
    }

    Some(self_path.to_string_lossy().to_string())
}

fn resolve_preview_node(
    preview_path: Option<&str>,
    explorer: &WorkspaceExplorer,
    explorer_sub_path: Option<&str>,
    mods_path: &str,
    safe_mode: bool,
) -> Option<WorkspaceNode> {
    let target_path = preview_path?;

    if let Some(child) = explorer
        .children
        .iter()
        .find(|folder| names_equal_by_key(&folder.path, target_path))
    {
        return Some(WorkspaceNode::Explorer(child.clone()));
    }

    let self_path = resolve_self_mod_path(mods_path, explorer_sub_path, explorer, safe_mode)?;
    if !names_equal_by_key(&self_path, target_path) {
        return None;
    }

    build_mod_folder_from_path(Path::new(&self_path), explorer_sub_path).map(|folder| {
        WorkspaceNode::Explorer(map_workspace_node(
            folder,
            explorer.ancestor_disabled_by.as_deref(),
        ))
    })
}

fn as_explorer_node(node: Option<&WorkspaceNode>) -> Option<&WorkspaceExplorerNode> {
    match node {
        Some(WorkspaceNode::Explorer(explorer)) => Some(explorer),
        Some(WorkspaceNode::Object(_)) | None => None,
    }
}

fn load_preview_mod_info_summary(node: &WorkspaceExplorerNode) -> WorkspaceModInfoSummary {
    let fallback = ModInfo::from_folder_name(&node.display_name);
    let info = read_info_json(Path::new(&node.path))
        .ok()
        .flatten()
        .unwrap_or(fallback);

    WorkspaceModInfoSummary {
        actual_name: if info.actual_name.trim().is_empty() {
            node.display_name.clone()
        } else {
            info.actual_name
        },
        author: info.author,
        version: info.version,
        description: info.description,
        is_safe: info.is_safe,
        is_favorite: info.is_favorite,
        has_info_json: node.has_info_json,
    }
}

fn load_preview_ini_summary(node: &WorkspaceExplorerNode) -> WorkspaceIniSummary {
    let files = list_mod_ini_files_inner(Path::new(&node.path)).unwrap_or_default();
    WorkspaceIniSummary {
        file_count: files.len(),
        file_names: files.into_iter().map(|entry| entry.filename).collect(),
    }
}

fn load_preview_image_summary(node: &WorkspaceExplorerNode) -> WorkspaceImageSummary {
    let images = list_mod_preview_images_inner(Path::new(&node.path)).unwrap_or_default();
    WorkspaceImageSummary {
        image_count: images.len(),
        primary_image_path: images.first().cloned(),
    }
}

fn build_warning_summary(node: Option<&WorkspaceExplorerNode>) -> WorkspaceWarningSummary {
    let Some(node) = node else {
        return WorkspaceWarningSummary {
            state: WorkspaceWarningState::None,
            messages: Vec::new(),
        };
    };

    let mut messages: Vec<WorkspaceWarning> = node
        .warnings
        .iter()
        .map(|message| build_folder_warning(message))
        .collect();

    if let Some(reason) = node.inactive_reason.clone() {
        messages.push(build_inactive_warning(&reason));
    }

    let state = if messages.is_empty() {
        WorkspaceWarningState::None
    } else {
        WorkspaceWarningState::Warning
    };

    WorkspaceWarningSummary { state, messages }
}

fn build_display_subtitle(summary: &WorkspaceModInfoSummary) -> Option<String> {
    let author = summary.author.trim();
    let version = summary.version.trim();
    if author.is_empty() && version.is_empty() {
        return None;
    }

    if author.is_empty() {
        return Some(format!("v{version}"));
    }

    if version.is_empty() {
        return Some(author.to_string());
    }

    Some(format!("{author} • v{version}"))
}

fn build_preview(
    explorer: &WorkspaceExplorer,
    explorer_sub_path: Option<&str>,
    mods_path: &str,
    selected_mod_path: Option<&str>,
    safe_mode: bool,
) -> WorkspacePreview {
    let self_mod_path = resolve_self_mod_path(mods_path, explorer_sub_path, explorer, safe_mode);
    let selected_path = resolve_preview_target_path(
        selected_mod_path,
        self_mod_path.as_deref(),
        &explorer.children,
    );
    let selected_node = resolve_preview_node(
        selected_path.as_deref(),
        explorer,
        explorer_sub_path,
        mods_path,
        safe_mode,
    );
    let explorer_node = as_explorer_node(selected_node.as_ref());
    let mod_info_summary = explorer_node.map(load_preview_mod_info_summary);
    let display_title = mod_info_summary
        .as_ref()
        .map(|summary| summary.actual_name.clone())
        .or_else(|| explorer_node.map(|node| node.display_name.clone()));
    let display_subtitle = mod_info_summary.as_ref().and_then(build_display_subtitle);
    let ini_summary = explorer_node.map(load_preview_ini_summary);
    let image_summary = explorer_node.map(load_preview_image_summary);
    let warning_summary = build_warning_summary(explorer_node);

    WorkspacePreview {
        selected_path,
        selected_node,
        is_flat_mod_root: explorer.self_display_mode == WorkspaceDisplayMode::FlatMod
            || explorer.self_is_mod,
        display_title,
        display_subtitle,
        mod_info_summary,
        ini_summary,
        image_summary,
        warning_summary,
    }
}

pub async fn get_workspace_view_model(
    pool: &sqlx::SqlitePool,
    input: WorkspaceViewModelInput,
) -> Result<WorkspaceViewModel, String> {
    let explorer_sub_path = resolve_explorer_sub_path(&input);
    let mods_path = load_game_mods_path(pool, &input.filter.game_id).await?;
    let objects = get_filtered_objects_with_conflict_check(pool, &input.filter)
        .await?
        .objects;
    let raw_explorer = list_mod_folders_for_game(
        pool,
        &input.filter.game_id,
        mods_path.clone(),
        explorer_sub_path.clone(),
    )
    .await?;
    let explorer = map_workspace_explorer(apply_runtime_corridor_filter_to_response(
        raw_explorer,
        input.filter.safe_mode,
    ));
    let preview = build_preview(
        &explorer,
        explorer_sub_path.as_deref(),
        &mods_path,
        input.selected_mod_path.as_deref(),
        input.filter.safe_mode,
    );
    let selection = WorkspaceSelection {
        selected_object_folder_path: input.selected_object_folder_path.clone(),
        explorer_sub_path: explorer_sub_path.clone(),
        selected_mod_path: preview.selected_path.clone(),
        current_path: build_current_path(
            input.selected_object_folder_path.as_deref(),
            explorer_sub_path.as_deref(),
        ),
    };

    Ok(WorkspaceViewModel {
        objects: objects.into_iter().map(map_workspace_object).collect(),
        explorer,
        preview,
        selection,
        runtime: WorkspaceRuntime {
            game_id: input.filter.game_id,
            safe_mode: input.filter.safe_mode,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::get_workspace_view_model;
    use crate::database::models::GameType;
    use crate::domain::workspace::{
        WorkspaceDisplayMode, WorkspaceNode, WorkspaceNodeKind, WorkspaceReasonCode,
        WorkspaceViewModelInput, WorkspaceWarningCode,
    };
    use crate::repo::object_repo::ObjectFilter;
    use crate::test_utils::{
        init_test_db, insert_test_game, insert_test_object, TestGameFixture, TestObjectFixture,
    };
    use std::fs;
    use tempfile::TempDir;

    fn build_filter(game_id: &str) -> ObjectFilter {
        ObjectFilter {
            game_id: game_id.to_string(),
            search_query: None,
            object_type: None,
            safe_mode: false,
            meta_filters: None,
            sort_by: None,
            status_filter: None,
        }
    }

    fn write_file(path: &std::path::Path, content: &str) {
        fs::write(path, content).expect("write test file");
    }

    async fn setup_workspace_fixture(
        object_folder: &str,
    ) -> (sqlx::SqlitePool, TempDir, String, String) {
        let ctx = init_test_db().await;
        let mods_root = TempDir::new().expect("tempdir");
        let mods_path = mods_root.path().join("Mods");
        fs::create_dir_all(&mods_path).expect("mods root");

        insert_test_game(
            &ctx.pool,
            &TestGameFixture {
                id: "game_workspace",
                name: "Test Game",
                game_type: GameType::GIMI,
                path: mods_root.path().to_string_lossy().as_ref(),
                mods_path: Some(mods_path.to_string_lossy().as_ref()),
            },
        )
        .await
        .expect("insert game");

        insert_test_object(
            &ctx.pool,
            &TestObjectFixture {
                id: "obj_workspace",
                game_id: "game_workspace",
                name: object_folder,
                folder_path: Some(object_folder),
                object_type: "Character",
            },
        )
        .await
        .expect("insert object");

        (
            ctx.pool,
            mods_root,
            mods_path.to_string_lossy().to_string(),
            object_folder.to_string(),
        )
    }

    #[tokio::test]
    async fn workspace_view_model_uses_flat_root_as_preview_target() {
        let (pool, _mods_root, mods_path, object_folder) = setup_workspace_fixture("AINOZ").await;
        let object_root = std::path::Path::new(&mods_path).join(&object_folder);
        fs::create_dir_all(&object_root).expect("object dir");
        write_file(&object_root.join("mod.ini"), "[TextureOverrideTest]\n");

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

        let nested_selection = object_root.join("Textures").join("example.dds");
        let view_model = get_workspace_view_model(
            &pool,
            WorkspaceViewModelInput {
                filter: build_filter("game_workspace"),
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
        let (pool, _mods_root, mods_path, object_folder) =
            setup_workspace_fixture("ALHAITHAM").await;
        let object_root = std::path::Path::new(&mods_path).join(&object_folder);
        let disabled_parent = object_root.join("DISABLED Variants");
        let variant = disabled_parent.join("School");
        fs::create_dir_all(&variant).expect("variant path");
        write_file(&variant.join("mod.ini"), "[TextureOverrideTest]\n");

        let explorer_sub_path = format!("{object_folder}/DISABLED Variants");
        let view_model = get_workspace_view_model(
            &pool,
            WorkspaceViewModelInput {
                filter: build_filter("game_workspace"),
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
}
