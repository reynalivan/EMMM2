use crate::repo::object_repo::ObjectFilter;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceViewModelInput {
    pub filter: ObjectFilter,
    pub selected_object_folder_path: Option<String>,
    pub explorer_sub_path: Option<String>,
    pub selected_mod_path: Option<String>,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceSelection {
    pub selected_object_folder_path: Option<String>,
    pub explorer_sub_path: Option<String>,
    pub selected_mod_path: Option<String>,
    pub current_path: Vec<String>,
    pub reconciliation_status: WorkspaceSelectionReconciliationStatus,
    pub reconciliation_reason: Option<WorkspaceSelectionReconciliationReason>,
    pub affected_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSelectionReconciliationStatus {
    Unchanged,
    Fallback,
    Cleared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSelectionReconciliationReason {
    MissingObjectRoot,
    MissingExplorerPath,
    MissingModPath,
    CorridorMismatch,
    SourceUnavailable,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceExplorer {
    pub self_node_type: Option<String>,
    pub self_node_kind: WorkspaceNodeKind,
    pub self_display_mode: WorkspaceDisplayMode,
    pub self_type_chip: Option<WorkspaceTypeChip>,
    pub self_is_mod: bool,
    pub self_is_enabled: bool,
    pub self_is_effectively_active: bool,
    pub self_owner_object_id: Option<String>,
    pub self_owner_object_folder_path: Option<String>,
    pub self_classification_reasons: Vec<String>,
    pub children: Vec<WorkspaceExplorerNode>,
    pub conflicts: Vec<crate::services::explorer::types::ConflictGroup>,
    pub ancestor_disabled_by: Option<String>,
    pub ancestor_disabled_path: Option<String>,
    pub inactive_reason: Option<WorkspaceReason>,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceModInfoSummary {
    pub actual_name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub is_safe: bool,
    pub is_favorite: bool,
    pub has_info_json: bool,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceIniSummary {
    pub file_count: usize,
    pub file_names: Vec<String>,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceImageSummary {
    pub image_count: usize,
    pub primary_image_path: Option<String>,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceWarningSummary {
    pub state: WorkspaceWarningState,
    pub messages: Vec<WorkspaceWarning>,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspacePreview {
    pub selected_path: Option<String>,
    pub selected_node: Option<WorkspaceNode>,
    pub is_flat_mod_root: bool,
    pub display_title: Option<String>,
    pub display_subtitle: Option<String>,
    pub mod_info_summary: Option<WorkspaceModInfoSummary>,
    pub ini_summary: Option<WorkspaceIniSummary>,
    pub image_summary: Option<WorkspaceImageSummary>,
    pub warning_summary: WorkspaceWarningSummary,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceRuntime {
    pub game_id: String,
    pub safe_mode: bool,
    pub source_state: WorkspaceSourceState,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceSourceState {
    pub status: WorkspaceSourceStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSourceStatus {
    Available,
    Unavailable,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceViewModel {
    pub objects: Vec<WorkspaceObjectNode>,
    pub explorer: WorkspaceExplorer,
    pub preview: WorkspacePreview,
    pub selection: WorkspaceSelection,
    pub runtime: WorkspaceRuntime,
}
