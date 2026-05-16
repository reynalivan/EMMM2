use std::path::Path;

use crate::domain::workspace::{
    WorkspaceSelectionReconciliationReason, WorkspaceSelectionReconciliationStatus,
    WorkspaceViewModelInput,
};
use crate::services::path_key::strip_path_prefix_preserve_display;

#[derive(Debug, Clone)]
pub struct ResolvedWorkspaceSelection {
    pub selected_object_folder_path: Option<String>,
    pub explorer_sub_path: Option<String>,
    pub selected_mod_path: Option<String>,
    pub reconciliation_status: WorkspaceSelectionReconciliationStatus,
    pub reconciliation_reason: Option<WorkspaceSelectionReconciliationReason>,
    pub affected_paths: Vec<String>,
}

fn resolve_requested_explorer_sub_path(input: &WorkspaceViewModelInput) -> Option<String> {
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

fn trimmed_input_path(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
}

pub fn push_affected_path(paths: &mut Vec<String>, path: &str) {
    if paths.iter().any(|current| current == path) {
        return;
    }

    paths.push(path.to_string());
}

fn resolve_existing_path(path: &Path) -> Option<std::path::PathBuf> {
    if path.exists() {
        return Some(path.to_path_buf());
    }

    let parent = path.parent()?;
    let name = path.file_name()?.to_string_lossy().to_string();
    let entries = std::fs::read_dir(parent).ok()?;

    entries
        .flatten()
        .find(|entry| {
            crate::services::path_key::names_equal_by_key(
                &entry.file_name().to_string_lossy(),
                &name,
            )
        })
        .map(|entry| entry.path())
        .or_else(|| {
            let disabled_name = format!("{}{}", crate::DISABLED_PREFIX, name);
            std::fs::read_dir(parent)
                .ok()?
                .flatten()
                .find(|entry| {
                    crate::services::path_key::names_equal_by_key(
                        &entry.file_name().to_string_lossy(),
                        &disabled_name,
                    )
                })
                .map(|entry| entry.path())
        })
}

pub fn existing_relative_sub_path(mods_path: &str, sub_path: &str) -> Option<String> {
    let trimmed = sub_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let requested = Path::new(mods_path).join(trimmed);
    let resolved = resolve_existing_path(&requested)?;
    if !resolved.is_dir() {
        return None;
    }

    let resolved_path = resolved.to_string_lossy().to_string();
    strip_path_prefix_preserve_display(&resolved_path, mods_path, None)
}

fn existing_absolute_path(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let resolved = resolve_existing_path(Path::new(trimmed))?;
    if !resolved.is_dir() {
        return None;
    }

    Some(resolved.to_string_lossy().to_string())
}

fn paths_match(left: &str, right: &str) -> bool {
    left.replace('\\', "/") == right.replace('\\', "/")
}

pub fn resolve_workspace_selection(
    mods_path: &str,
    input: &WorkspaceViewModelInput,
) -> ResolvedWorkspaceSelection {
    let mut reconciliation_status = WorkspaceSelectionReconciliationStatus::Unchanged;
    let mut reconciliation_reason = None;
    let mut affected_paths = Vec::new();

    let requested_object_folder_path = trimmed_input_path(&input.selected_object_folder_path);
    let selected_object_folder_path = requested_object_folder_path
        .as_deref()
        .and_then(|path| existing_relative_sub_path(mods_path, path));

    if let Some(requested_path) = requested_object_folder_path.as_deref() {
        if selected_object_folder_path.is_none() {
            return ResolvedWorkspaceSelection {
                selected_object_folder_path: None,
                explorer_sub_path: None,
                selected_mod_path: None,
                reconciliation_status: WorkspaceSelectionReconciliationStatus::Cleared,
                reconciliation_reason: Some(
                    WorkspaceSelectionReconciliationReason::MissingObjectRoot,
                ),
                affected_paths: vec![requested_path.to_string()],
            };
        }

        if let Some(selected_path) = selected_object_folder_path.as_deref() {
            if !paths_match(selected_path, requested_path) {
                reconciliation_status = WorkspaceSelectionReconciliationStatus::Fallback;
                reconciliation_reason =
                    Some(WorkspaceSelectionReconciliationReason::MissingObjectRoot);
                push_affected_path(&mut affected_paths, requested_path);
            }
        }
    }

    let requested_explorer_sub_path = resolve_requested_explorer_sub_path(input);
    let explorer_sub_path = requested_explorer_sub_path
        .as_deref()
        .and_then(|path| existing_relative_sub_path(mods_path, path))
        .or_else(|| selected_object_folder_path.clone());

    if let Some(requested_path) = requested_explorer_sub_path.as_deref() {
        if explorer_sub_path
            .as_deref()
            .is_none_or(|selected_path| !paths_match(selected_path, requested_path))
        {
            reconciliation_status = WorkspaceSelectionReconciliationStatus::Fallback;
            reconciliation_reason =
                Some(WorkspaceSelectionReconciliationReason::MissingExplorerPath);
            push_affected_path(&mut affected_paths, requested_path);
        }
    }

    let requested_mod_path = trimmed_input_path(&input.selected_mod_path);
    let selected_mod_path = requested_mod_path
        .as_deref()
        .and_then(existing_absolute_path);

    if let Some(requested_path) = requested_mod_path.as_deref() {
        if selected_mod_path.is_none() {
            push_affected_path(&mut affected_paths, requested_path);
            if reconciliation_status == WorkspaceSelectionReconciliationStatus::Unchanged {
                reconciliation_status = WorkspaceSelectionReconciliationStatus::Cleared;
                reconciliation_reason =
                    Some(WorkspaceSelectionReconciliationReason::MissingModPath);
            }
        } else if selected_mod_path
            .as_deref()
            .is_some_and(|selected_path| !paths_match(selected_path, requested_path))
        {
            push_affected_path(&mut affected_paths, requested_path);
            if reconciliation_status == WorkspaceSelectionReconciliationStatus::Unchanged {
                reconciliation_status = WorkspaceSelectionReconciliationStatus::Fallback;
                reconciliation_reason =
                    Some(WorkspaceSelectionReconciliationReason::MissingModPath);
            }
        }
    }

    ResolvedWorkspaceSelection {
        selected_object_folder_path,
        explorer_sub_path,
        selected_mod_path,
        reconciliation_status,
        reconciliation_reason,
        affected_paths,
    }
}

pub fn resolve_unavailable_workspace_selection(
    input: &WorkspaceViewModelInput,
) -> ResolvedWorkspaceSelection {
    let mut affected_paths = Vec::new();
    if let Some(path) = trimmed_input_path(&input.selected_object_folder_path) {
        push_affected_path(&mut affected_paths, &path);
    }
    if let Some(path) = trimmed_input_path(&input.explorer_sub_path) {
        push_affected_path(&mut affected_paths, &path);
    }
    if let Some(path) = trimmed_input_path(&input.selected_mod_path) {
        push_affected_path(&mut affected_paths, &path);
    }

    let reconciliation_status = if affected_paths.is_empty() {
        WorkspaceSelectionReconciliationStatus::Unchanged
    } else {
        WorkspaceSelectionReconciliationStatus::Cleared
    };
    let reconciliation_reason = if affected_paths.is_empty() {
        None
    } else {
        Some(WorkspaceSelectionReconciliationReason::SourceUnavailable)
    };

    ResolvedWorkspaceSelection {
        selected_object_folder_path: None,
        explorer_sub_path: None,
        selected_mod_path: None,
        reconciliation_status,
        reconciliation_reason,
        affected_paths,
    }
}

pub fn build_current_path(
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
