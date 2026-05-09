use crate::domain::workspace::{
    WorkspaceCapabilities, WorkspaceDisplayMode, WorkspaceNodeKind, WorkspaceObjectNode,
    WorkspaceSwitchPolicyKey, WorkspaceSwitchState, WorkspaceWarningState,
};
use crate::repo::object_repo::ObjectSummary;
use crate::services::workspace_read_model::common::{
    build_folder_warning, build_inactive_warning, build_naming_conflict_warning,
    build_object_inactive_reason,
};
use crate::services::workspace_read_model::selection::existing_relative_sub_path;

fn map_object_switch_state(object: &ObjectSummary) -> WorkspaceSwitchState {
    if object.is_object_disabled {
        return WorkspaceSwitchState::Disabled;
    }

    WorkspaceSwitchState::Enabled
}

fn map_object_switch_policy_key(object: &ObjectSummary) -> WorkspaceSwitchPolicyKey {
    if object.mod_count <= 0 {
        return WorkspaceSwitchPolicyKey::Blocked;
    }

    WorkspaceSwitchPolicyKey::Object
}

fn map_workspace_object(object: ObjectSummary, object_folder_exists: bool) -> WorkspaceObjectNode {
    let inactive_reason = build_object_inactive_reason(&object);
    let missing_folder_warning =
        (!object_folder_exists).then(|| build_folder_warning("Object folder is missing on disk"));
    let primary_warning = if let Some(warning) = missing_folder_warning {
        Some(warning)
    } else if object.has_naming_conflict {
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
        can_toggle: object_folder_exists && object.mod_count > 0,
        can_rename: true,
        can_delete: true,
        can_move: false,
        can_toggle_safe: false,
        can_sync: true,
        can_enable_only_this: false,
        can_pin: true,
        can_edit_metadata: true,
        can_reveal_in_explorer: object_folder_exists,
        can_move_category: true,
        can_open_in_explorer: object_folder_exists,
    };
    let switch_reason = inactive_reason.clone();
    let switch_state = map_object_switch_state(&object);
    let switch_policy_key = if object_folder_exists {
        map_object_switch_policy_key(&object)
    } else {
        WorkspaceSwitchPolicyKey::Blocked
    };

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

pub(crate) fn map_workspace_objects(
    objects: Vec<ObjectSummary>,
    mods_path: &str,
    source_available: bool,
) -> Vec<WorkspaceObjectNode> {
    objects
        .into_iter()
        .map(|object| {
            let object_folder_exists = source_available
                && existing_relative_sub_path(mods_path, &object.folder_path).is_some();
            map_workspace_object(object, object_folder_exists)
        })
        .collect()
}
