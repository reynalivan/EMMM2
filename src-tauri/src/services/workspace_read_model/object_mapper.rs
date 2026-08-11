use std::collections::HashMap;

use crate::common::path_key::folder_path_key;
use crate::domain::models::ItemStatus;
use crate::domain::objects::ObjectSummary;
use crate::domain::workspace::{
    WorkspaceCapabilities, WorkspaceDisplayMode, WorkspaceNodeKind, WorkspaceObjectNode,
    WorkspaceSwitchPolicyKey, WorkspaceSwitchState, WorkspaceWarningState,
};
use crate::services::explorer::types::ModFolder;
use crate::services::workspace_read_model::common::{
    build_folder_warning, build_inactive_warning, build_naming_conflict_warning,
    build_object_inactive_reason,
};
use crate::services::workspace_read_model::selection::relative_sub_path_exists;

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
        is_registered: true,
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

fn unregistered_root_capabilities() -> WorkspaceCapabilities {
    WorkspaceCapabilities {
        can_toggle: false,
        can_rename: false,
        can_delete: false,
        can_move: false,
        can_toggle_safe: false,
        can_sync: false,
        can_enable_only_this: false,
        can_pin: false,
        can_edit_metadata: false,
        can_reveal_in_explorer: false,
        can_move_category: false,
        can_open_in_explorer: false,
    }
}

fn runtime_root_object(folder: &ModFolder) -> ObjectSummary {
    let path_hash = blake3::hash(folder.path.as_bytes()).to_hex();
    ObjectSummary {
        id: format!("fs_{}", &path_hash[..16]),
        name: folder.name.clone(),
        folder_path: folder.folder_name.clone(),
        matched_entry_key: None,
        matched_alias_name: None,
        matched_confidence: None,
        matched_reason: None,
        matched_source: None,
        object_type: "Other".to_string(),
        sub_category: None,
        status: if folder.is_enabled {
            ItemStatus::Enabled
        } else {
            ItemStatus::Disabled
        },
        metadata: "{}".to_string(),
        tags: "[]".to_string(),
        hash_db: None,
        custom_skins: None,
        is_pinned: false,
        is_auto_sync: false,
        thumbnail_path: folder.thumbnail_path.clone(),
        created_at: None,
        mod_count: 0,
        enabled_count: 0,
        is_object_disabled: !folder.is_enabled,
        has_naming_conflict: folder.conflict_group_id.is_some(),
        active_mod_paths: None,
    }
}

fn map_runtime_root(folder: &ModFolder) -> WorkspaceObjectNode {
    let object = runtime_root_object(folder);
    let switch_state = if folder.is_enabled {
        WorkspaceSwitchState::Enabled
    } else {
        WorkspaceSwitchState::Disabled
    };

    WorkspaceObjectNode {
        display_name: object.name.clone(),
        is_registered: false,
        is_effectively_active: folder.is_enabled,
        inactive_reason: None,
        warning_state: WorkspaceWarningState::None,
        primary_warning: None,
        switch_state,
        switch_reason: None,
        switch_policy_key: WorkspaceSwitchPolicyKey::Blocked,
        node_kind: WorkspaceNodeKind::Object,
        display_mode: WorkspaceDisplayMode::Unknown,
        type_chip: None,
        capabilities: unregistered_root_capabilities(),
        object,
    }
}

pub(crate) struct WorkspaceObjectMapping<'a> {
    pub objects: Vec<ObjectSummary>,
    pub root_folders: &'a [ModFolder],
    pub mods_path: &'a str,
    pub source_available: bool,
    pub include_unregistered: bool,
}

pub(crate) fn map_workspace_objects(
    mapping: WorkspaceObjectMapping<'_>,
) -> Vec<WorkspaceObjectNode> {
    let mut objects_by_path: HashMap<String, ObjectSummary> = mapping
        .objects
        .into_iter()
        .map(|object| {
            let key = folder_path_key(&object.folder_path, Some(mapping.mods_path));
            (key, object)
        })
        .collect();
    let mut roots = Vec::with_capacity(mapping.root_folders.len() + objects_by_path.len());

    for folder in mapping.root_folders {
        let key = folder_path_key(&folder.path, Some(mapping.mods_path));
        if let Some(object) = objects_by_path.remove(&key) {
            roots.push(map_workspace_object(object, true));
            continue;
        }
        if mapping.include_unregistered && folder.owner_object_id.is_none() {
            roots.push(map_runtime_root(folder));
        }
    }

    roots.extend(objects_by_path.into_values().map(|object| {
        let folder_exists = mapping.source_available
            && relative_sub_path_exists(mapping.mods_path, &object.folder_path);
        map_workspace_object(object, folder_exists)
    }));
    roots
}
