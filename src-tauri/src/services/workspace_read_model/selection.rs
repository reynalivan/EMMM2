use std::path::Path;

use crate::common::path_key::{canonical_name_key, strip_path_prefix_preserve_display};
use crate::domain::workspace::{
    WorkspaceSelectionReconciliationReason, WorkspaceSelectionReconciliationStatus,
    WorkspaceViewModelInput,
};

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

/// Resolve a folder that may exist under a different case or a `DISABLED `
/// prefix, returning it only if it is a directory.
///
/// One `metadata` call answers exists-and-is-a-directory together, and the
/// fallback scans the parent once against both candidate names — this runs per
/// object on every view-model fetch, and the parent is the mods root.
fn resolve_existing_dir(path: &Path) -> Option<std::path::PathBuf> {
    if std::fs::metadata(path).is_ok_and(|meta| meta.is_dir()) {
        return Some(path.to_path_buf());
    }

    let parent = path.parent()?;
    let name = path.file_name()?.to_string_lossy().to_string();
    // Hoisted: `names_equal_by_key` would otherwise re-derive these per entry.
    let name_key = canonical_name_key(&name);
    let disabled_key = canonical_name_key(&format!("{}{}", crate::DISABLED_PREFIX, name));

    std::fs::read_dir(parent)
        .ok()?
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
        .find(|entry| {
            let entry_key = canonical_name_key(&entry.file_name().to_string_lossy());
            entry_key == name_key || entry_key == disabled_key
        })
        .map(|entry| entry.path())
}

/// Whether `sub_path` still resolves to a directory under `mods_path`.
/// Cheaper than [`existing_relative_sub_path`] — skips the prefix strip.
pub fn relative_sub_path_exists(mods_path: &str, sub_path: &str) -> bool {
    let trimmed = sub_path.trim();
    if trimmed.is_empty() {
        return false;
    }

    resolve_existing_dir(&Path::new(mods_path).join(trimmed)).is_some()
}

pub fn existing_relative_sub_path(mods_path: &str, sub_path: &str) -> Option<String> {
    let trimmed = sub_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let resolved = resolve_existing_dir(&Path::new(mods_path).join(trimmed))?;
    strip_path_prefix_preserve_display(&resolved.to_string_lossy(), mods_path, None)
}

fn existing_absolute_path(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let resolved = resolve_existing_dir(Path::new(trimmed))?;
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
