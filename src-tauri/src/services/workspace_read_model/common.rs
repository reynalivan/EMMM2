use std::collections::HashMap;

use crate::domain::workspace::{
    WorkspaceDisplayMode, WorkspaceNodeKind, WorkspaceReason, WorkspaceReasonCode,
    WorkspaceTypeChip, WorkspaceWarning, WorkspaceWarningCode, WorkspaceWarningState,
};
use crate::repo::object_repo::ObjectSummary;
use crate::services::explorer::classifier::NodeType;
use crate::services::path_key::path_starts_with_key;

pub(crate) fn build_workspace_args(entries: &[(&str, &str)]) -> HashMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

pub(crate) fn build_disabled_by_container_reason(
    ancestor_disabled_by: Option<&str>,
) -> Option<WorkspaceReason> {
    ancestor_disabled_by.map(|value| WorkspaceReason {
        code: WorkspaceReasonCode::DisabledByContainer,
        args: build_workspace_args(&[("container_name", value)]),
    })
}

pub(crate) fn build_object_inactive_reason(object: &ObjectSummary) -> Option<WorkspaceReason> {
    if object.is_object_disabled {
        return Some(WorkspaceReason {
            code: WorkspaceReasonCode::ObjectFolderDisabled,
            args: build_workspace_args(&[]),
        });
    }

    None
}

pub(crate) fn build_inactive_warning(reason: &WorkspaceReason) -> WorkspaceWarning {
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

pub(crate) fn build_folder_warning(message: &str) -> WorkspaceWarning {
    WorkspaceWarning {
        code: WorkspaceWarningCode::FolderWarning,
        args: build_workspace_args(&[("message", message)]),
        state: WorkspaceWarningState::Warning,
    }
}

pub(crate) fn build_naming_conflict_warning() -> WorkspaceWarning {
    WorkspaceWarning {
        code: WorkspaceWarningCode::NamingConflict,
        args: build_workspace_args(&[]),
        state: WorkspaceWarningState::Warning,
    }
}

pub(crate) fn map_display_mode(node_type: &str) -> WorkspaceDisplayMode {
    match node_type {
        "ContainerFolder" => WorkspaceDisplayMode::ContainerFolder,
        "ModPackRoot" => WorkspaceDisplayMode::ModPack,
        "VariantContainer" => WorkspaceDisplayMode::Variant,
        "FlatModRoot" => WorkspaceDisplayMode::FlatMod,
        "InternalAssets" => WorkspaceDisplayMode::InternalAssets,
        _ => WorkspaceDisplayMode::Unknown,
    }
}

pub(crate) fn map_type_chip(display_mode: WorkspaceDisplayMode) -> Option<WorkspaceTypeChip> {
    match display_mode {
        WorkspaceDisplayMode::ModPack => Some(WorkspaceTypeChip::ModPack),
        WorkspaceDisplayMode::Variant => Some(WorkspaceTypeChip::Variant),
        WorkspaceDisplayMode::FlatMod => Some(WorkspaceTypeChip::FlatMod),
        WorkspaceDisplayMode::ContainerFolder
        | WorkspaceDisplayMode::InternalAssets
        | WorkspaceDisplayMode::Unknown => None,
    }
}

pub(crate) fn map_node_kind(node_type: &str, ancestor_disabled: bool) -> WorkspaceNodeKind {
    if ancestor_disabled {
        return WorkspaceNodeKind::InactiveBranch;
    }

    if node_type == NodeType::ContainerFolder.as_str() {
        return WorkspaceNodeKind::Container;
    }

    WorkspaceNodeKind::TerminalMod
}

pub(crate) fn map_warning_state(
    warnings: &[String],
    inactive_reason: Option<&WorkspaceReason>,
    has_primary_warning: bool,
) -> WorkspaceWarningState {
    if warnings.is_empty() && inactive_reason.is_none() && !has_primary_warning {
        return WorkspaceWarningState::None;
    }

    WorkspaceWarningState::Warning
}

pub(crate) fn paths_equal_by_key(left: &str, right: &str) -> bool {
    path_starts_with_key(left, right, None) && path_starts_with_key(right, left, None)
}
