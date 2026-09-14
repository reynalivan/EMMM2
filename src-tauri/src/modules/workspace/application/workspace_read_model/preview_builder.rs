use std::path::{Component, Path, PathBuf};

use crate::modules::library::application::mods::info_json::{read_info_json, ModInfo};
use crate::modules::workspace::application::explorer::listing::{
    build_mod_folder_from_path, enrich_mod_folder_for_game, find_disabled_ancestor,
};
use crate::modules::workspace::application::workspace_read_model::common::{
    build_folder_warning, build_inactive_warning,
};
use crate::modules::workspace::application::workspace_read_model::explorer_mapper::map_workspace_node;
use crate::modules::workspace::application::workspace_read_model::selection::resolve_existing_dir;
use crate::modules::workspace::domain::workspace::{
    WorkspaceDisplayMode, WorkspaceExplorerNode, WorkspaceModInfoSummary, WorkspaceNode,
    WorkspacePreview, WorkspacePreviewContextStatus, WorkspacePreviewInput,
    WorkspacePreviewSelection, WorkspaceSelectionReconciliationReason,
    WorkspaceSelectionReconciliationStatus, WorkspaceWarning, WorkspaceWarningState,
    WorkspaceWarningSummary,
};
use crate::shared::errors::AppError;
use crate::shared::path_key::{path_starts_with_key, strip_path_prefix_preserve_display};

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

pub(crate) struct PreviewRead {
    pub context_status: WorkspacePreviewContextStatus,
    pub preview: WorkspacePreview,
    pub selection: WorkspacePreviewSelection,
}

fn unchanged_preview_selection(selected_mod_path: Option<String>) -> WorkspacePreviewSelection {
    WorkspacePreviewSelection {
        selected_mod_path,
        reconciliation_status: WorkspaceSelectionReconciliationStatus::Unchanged,
        reconciliation_reason: None,
        affected_paths: Vec::new(),
    }
}

fn cleared_preview_selection(selected_mod_path: Option<String>) -> WorkspacePreviewSelection {
    let affected_paths = selected_mod_path.into_iter().collect();
    WorkspacePreviewSelection {
        selected_mod_path: None,
        reconciliation_status: WorkspaceSelectionReconciliationStatus::Cleared,
        reconciliation_reason: Some(WorkspaceSelectionReconciliationReason::MissingModPath),
        affected_paths,
    }
}

fn paths_match_display(left: &str, right: &str) -> bool {
    left.replace('\\', "/") == right.replace('\\', "/")
}

fn resolved_preview_selection(
    requested_path: Option<String>,
    selected_mod_path: Option<String>,
) -> WorkspacePreviewSelection {
    let Some(requested_path) = requested_path else {
        return unchanged_preview_selection(selected_mod_path);
    };
    if selected_mod_path
        .as_deref()
        .is_none_or(|selected_path| !paths_match_display(selected_path, &requested_path))
    {
        return WorkspacePreviewSelection {
            selected_mod_path,
            reconciliation_status: WorkspaceSelectionReconciliationStatus::Fallback,
            reconciliation_reason: Some(WorkspaceSelectionReconciliationReason::MissingModPath),
            affected_paths: vec![requested_path],
        };
    }
    unchanged_preview_selection(selected_mod_path)
}

fn checked_relative_context(mods_path: &Path, sub_path: Option<&str>) -> Result<PathBuf, AppError> {
    let Some(sub_path) = sub_path.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(mods_path.to_path_buf());
    };
    let relative = Path::new(sub_path);
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(AppError::Security(
            "Preview explorer path resolves outside of mods_path".to_string(),
        ));
    }
    Ok(mods_path.join(relative))
}

fn canonical_path_within(path: &Path, root: &Path) -> Result<Option<PathBuf>, AppError> {
    let Ok(canonical) = path.canonicalize() else {
        return Ok(None);
    };
    if !canonical.starts_with(root) {
        return Err(AppError::Security(
            "Preview path resolves outside of mods_path".to_string(),
        ));
    }
    Ok(Some(canonical))
}

fn is_direct_child(path: &Path, parent: &Path) -> bool {
    path.parent().is_some_and(|candidate| candidate == parent)
}

async fn build_preview_node(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &str,
    path: PathBuf,
    sub_path: Option<String>,
    ancestor_disabled_by: Option<&str>,
) -> Result<Option<WorkspaceExplorerNode>, AppError> {
    let folder =
        tokio::task::spawn_blocking(move || build_mod_folder_from_path(&path, sub_path.as_deref()))
            .await?;
    let Some(mut folder) = folder else {
        return Ok(None);
    };
    enrich_mod_folder_for_game(pool, game_id, mods_path, &mut folder).await?;
    Ok(Some(map_workspace_node(folder, ancestor_disabled_by)))
}

fn preview_from_node(node: WorkspaceExplorerNode, is_flat_mod_root: bool) -> WorkspacePreview {
    let mod_info_summary = load_preview_mod_info_summary(&node);
    let display_title = Some(mod_info_summary.actual_name.clone());
    let display_subtitle = build_display_subtitle(&mod_info_summary);
    let warning_summary = build_warning_summary(Some(&node));

    WorkspacePreview {
        selected_path: Some(node.path.clone()),
        selected_node: Some(WorkspaceNode::Explorer(node)),
        is_flat_mod_root,
        display_title,
        display_subtitle,
        mod_info_summary: Some(mod_info_summary),
        ini_summary: None,
        image_summary: None,
        warning_summary,
    }
}

/// Builds one preview directly from disk. It deliberately avoids the explorer
/// listing and object read-model so selecting a mod cannot retraverse siblings.
pub(crate) async fn read_preview(
    pool: &sqlx::SqlitePool,
    input: &WorkspacePreviewInput,
    mods_path: &str,
) -> Result<PreviewRead, AppError> {
    let requested_selection = input
        .selected_mod_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let root = Path::new(mods_path);
    let Ok(canonical_root) = root.canonicalize() else {
        return Ok(PreviewRead {
            context_status: WorkspacePreviewContextStatus::ContextStale,
            preview: empty_workspace_preview(),
            selection: unchanged_preview_selection(requested_selection),
        });
    };
    if !canonical_root.is_dir() {
        return Ok(PreviewRead {
            context_status: WorkspacePreviewContextStatus::ContextStale,
            preview: empty_workspace_preview(),
            selection: unchanged_preview_selection(requested_selection),
        });
    }
    let requested_context = checked_relative_context(root, input.explorer_sub_path.as_deref())?;
    let Some(canonical_context) = canonical_path_within(&requested_context, &canonical_root)?
    else {
        return Ok(PreviewRead {
            context_status: WorkspacePreviewContextStatus::ContextStale,
            preview: empty_workspace_preview(),
            selection: unchanged_preview_selection(requested_selection),
        });
    };
    if !canonical_context.is_dir() {
        return Ok(PreviewRead {
            context_status: WorkspacePreviewContextStatus::ContextStale,
            preview: empty_workspace_preview(),
            selection: unchanged_preview_selection(requested_selection),
        });
    }
    let context_sub_path = strip_path_prefix_preserve_display(
        &requested_context.to_string_lossy(),
        &root.to_string_lossy(),
        None,
    );
    let ancestor = context_sub_path
        .as_deref()
        .and_then(|path| find_disabled_ancestor(&root.to_string_lossy(), path));
    let ancestor_disabled_by = ancestor.as_ref().map(|(name, _)| name.as_str());
    // The mods root is navigation-only. Treating it as a candidate mod would
    // classify every direct child on each preview click, which recreates the
    // listing-sized read this endpoint exists to avoid.
    let self_node = if canonical_context == canonical_root {
        None
    } else {
        build_preview_node(
            pool,
            &input.game_id,
            &root.to_string_lossy(),
            requested_context.clone(),
            context_sub_path.clone(),
            ancestor_disabled_by,
        )
        .await?
    };
    let self_is_mod = self_node.as_ref().is_some_and(|node| {
        matches!(
            node.node_type.as_str(),
            "FlatModRoot" | "ModPackRoot" | "VariantContainer"
        )
    });
    let self_is_flat_mod = self_node
        .as_ref()
        .is_some_and(|node| node.display_mode == WorkspaceDisplayMode::FlatMod || self_is_mod);

    let selected_path = requested_selection.as_deref();
    let target_node = match selected_path {
        None => self_node,
        Some(selected_path) => {
            let selected = Path::new(selected_path);
            if !selected.is_absolute() {
                return Err(AppError::Security(
                    "Preview selected path must be absolute".to_string(),
                ));
            }
            if !path_starts_with_key(selected_path, mods_path, None) {
                return Err(AppError::Security(
                    "Preview selected path resolves outside of mods_path".to_string(),
                ));
            }
            let selected_on_disk = resolve_existing_dir(selected);
            let canonical_selected = selected_on_disk
                .as_deref()
                .map(|path| canonical_path_within(path, &canonical_root))
                .transpose()?
                .flatten();
            let selected_is_existing_file =
                selected.is_file() && canonical_path_within(selected, &canonical_root)?.is_some();
            if canonical_selected.is_none()
                && self_is_mod
                && selected_is_existing_file
                && path_starts_with_key(selected_path, &requested_context.to_string_lossy(), None)
            {
                self_node
            } else if let Some(canonical_selected) = canonical_selected {
                if canonical_selected.is_dir()
                    && is_direct_child(&canonical_selected, &canonical_context)
                {
                    let selected_sub_path = strip_path_prefix_preserve_display(
                        &selected_on_disk
                            .as_ref()
                            .expect("canonical selected path has a disk path")
                            .to_string_lossy(),
                        &root.to_string_lossy(),
                        None,
                    );
                    let selected_node = build_preview_node(
                        pool,
                        &input.game_id,
                        &root.to_string_lossy(),
                        selected_on_disk.expect("canonical selected path has a disk path"),
                        selected_sub_path,
                        ancestor_disabled_by,
                    )
                    .await?;
                    if self_is_mod
                        && selected_node.as_ref().is_some_and(|node| {
                            node.node_type
                                != crate::modules::workspace::domain::classifier::NodeType::ContainerFolder.as_str()
                        })
                    {
                        self_node
                    } else {
                        selected_node
                    }
                } else if self_is_mod
                    && path_starts_with_key(
                        &canonical_selected.to_string_lossy(),
                        &canonical_context.to_string_lossy(),
                        None,
                    )
                {
                    self_node
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    let Some(target_node) = target_node else {
        if selected_path.is_none() {
            return Ok(PreviewRead {
                context_status: WorkspacePreviewContextStatus::Ready,
                preview: empty_workspace_preview(),
                selection: unchanged_preview_selection(requested_selection),
            });
        }
        return Ok(PreviewRead {
            context_status: WorkspacePreviewContextStatus::Ready,
            preview: empty_workspace_preview(),
            selection: cleared_preview_selection(requested_selection),
        });
    };
    let preview = preview_from_node(target_node, self_is_flat_mod);
    Ok(PreviewRead {
        context_status: WorkspacePreviewContextStatus::Ready,
        selection: resolved_preview_selection(requested_selection, preview.selected_path.clone()),
        preview,
    })
}
