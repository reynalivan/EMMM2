use std::path::Path;

use crate::commands::mods::preview_cmds::{
    list_mod_ini_files_inner, list_mod_preview_images_inner,
};
use crate::domain::workspace::{
    WorkspaceDisplayMode, WorkspaceExplorer, WorkspaceExplorerNode, WorkspaceImageSummary,
    WorkspaceIniSummary, WorkspaceModInfoSummary, WorkspaceNode, WorkspacePreview,
    WorkspaceSelectionReconciliationReason, WorkspaceSelectionReconciliationStatus,
    WorkspaceWarning, WorkspaceWarningState, WorkspaceWarningSummary,
};
use crate::services::explorer::classifier::NodeType;
use crate::services::explorer::listing::build_mod_folder_from_path;
use crate::services::mods::info_json::{read_info_json, ModInfo};
use crate::services::path_key::path_starts_with_key;
use crate::services::workspace_read_model::common::{
    build_folder_warning, build_inactive_warning, paths_equal_by_key,
};
use crate::services::workspace_read_model::explorer_mapper::map_workspace_node;
use crate::services::workspace_read_model::selection::{
    push_affected_path, ResolvedWorkspaceSelection,
};

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
        .find(|folder| paths_equal_by_key(&folder.path, external_selected_path));

    if let Some(child) = selected_child {
        if let Some(self_path) = self_mod_path {
            if child.node_type == NodeType::ContainerFolder.as_str() {
                return Some(external_selected_path.to_string());
            }
            return Some(self_path.to_string());
        }

        return Some(external_selected_path.to_string());
    }

    let self_path = self_mod_path?;

    if paths_equal_by_key(external_selected_path, self_path) {
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
        .find(|folder| paths_equal_by_key(&folder.path, target_path))
    {
        return Some(WorkspaceNode::Explorer(child.clone()));
    }

    let self_path = resolve_self_mod_path(mods_path, explorer_sub_path, explorer, safe_mode)?;
    if !paths_equal_by_key(&self_path, target_path) {
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

pub(crate) fn empty_workspace_preview() -> WorkspacePreview {
    WorkspacePreview {
        selected_path: None,
        selected_node: None,
        is_flat_mod_root: false,
        display_title: None,
        display_subtitle: None,
        mod_info_summary: None,
        ini_summary: None,
        image_summary: None,
        warning_summary: WorkspaceWarningSummary {
            state: WorkspaceWarningState::None,
            messages: Vec::new(),
        },
    }
}

pub(crate) fn build_preview(
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

pub(crate) fn clear_preview_selection_for_corridor_mismatch(
    resolved_selection: &mut ResolvedWorkspaceSelection,
    preview: &WorkspacePreview,
) {
    if resolved_selection.selected_mod_path.is_none() || preview.selected_path.is_some() {
        return;
    }

    if let Some(path) = resolved_selection.selected_mod_path.clone() {
        push_affected_path(&mut resolved_selection.affected_paths, &path);
    }
    resolved_selection.selected_mod_path = None;
    resolved_selection.reconciliation_status = WorkspaceSelectionReconciliationStatus::Cleared;
    resolved_selection.reconciliation_reason =
        Some(WorkspaceSelectionReconciliationReason::CorridorMismatch);
}
