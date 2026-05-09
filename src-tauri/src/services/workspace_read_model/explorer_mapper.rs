use crate::domain::workspace::{
    WorkspaceCapabilities, WorkspaceDisplayMode, WorkspaceExplorer, WorkspaceExplorerNode,
    WorkspaceNodeKind, WorkspaceSwitchPolicyKey, WorkspaceSwitchState,
};
use crate::services::explorer::classifier::NodeType;
use crate::services::explorer::types::{FolderGridResponse, ModFolder};
use crate::services::workspace_read_model::common::{
    build_disabled_by_container_reason, build_folder_warning, build_inactive_warning,
    map_display_mode, map_node_kind, map_type_chip, map_warning_state,
};

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

pub(crate) fn map_workspace_node(
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

pub(crate) fn map_workspace_explorer(explorer: FolderGridResponse) -> WorkspaceExplorer {
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

pub(crate) fn empty_workspace_explorer() -> WorkspaceExplorer {
    WorkspaceExplorer {
        self_node_type: None,
        self_node_kind: WorkspaceNodeKind::Container,
        self_display_mode: WorkspaceDisplayMode::Unknown,
        self_type_chip: None,
        self_is_mod: false,
        self_is_enabled: false,
        self_is_effectively_active: false,
        self_owner_object_id: None,
        self_owner_object_folder_path: None,
        self_classification_reasons: Vec::new(),
        children: Vec::new(),
        conflicts: Vec::new(),
        ancestor_disabled_by: None,
        ancestor_disabled_path: None,
        inactive_reason: None,
    }
}
