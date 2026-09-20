use crate::modules::catalog::domain::objects::ObjectFilter;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceStructureInput {
    pub filter: ObjectFilter,
    pub selected_object_folder_path: Option<String>,
    pub explorer_sub_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceExplorerSortField {
    Name,
    ModifiedAt,
    SizeBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceExplorerSortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceExplorerSafetyFilter {
    All,
    Safe,
    Unsafe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceExplorerQuery {
    pub game_id: String,
    pub explorer_sub_path: Option<String>,
    pub search_query: Option<String>,
    pub sort_field: WorkspaceExplorerSortField,
    pub sort_order: WorkspaceExplorerSortOrder,
    pub safety_filter: WorkspaceExplorerSafetyFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceExplorerPageInput {
    pub query: WorkspaceExplorerQuery,
    pub cursor: Option<String>,
    pub page_size: u32,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceExplorerPage {
    pub items: Vec<WorkspaceExplorerNode>,
    pub next_cursor: Option<String>,
    #[specta(type = f64)]
    pub total_matching: u64,
    pub query_fingerprint: String,
    /// Opaque identity of the immutable backend listing used by this page.
    /// Bulk actions must carry this value so they cannot silently include
    /// folders that appeared after the user selected the result set.
    pub listing_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum WorkspaceExplorerSelection {
    Explicit { paths: Vec<String> },
    AllMatching { excluded_paths: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceExplorerSelectionInput {
    pub query: WorkspaceExplorerQuery,
    pub listing_revision: String,
    pub selection: WorkspaceExplorerSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceExplorerBulkAction {
    Toggle {
        enable: bool,
        operation_id: String,
    },
    Delete {
        operation_id: String,
    },
    UpdateInfo {
        update: crate::modules::library::application::mods::info_json::ModInfoUpdate,
    },
    SetSafety {
        safe: bool,
    },
    SetFavorite {
        favorite: bool,
    },
    SetPin {
        pin: bool,
    },
    MoveToObject {
        target_object_id: String,
        target_subpath: Option<String>,
        status: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceExplorerBulkInput {
    pub selection: WorkspaceExplorerSelectionInput,
    pub action: WorkspaceExplorerBulkAction,
}

#[derive(Clone)]
pub struct ResolvedWorkspaceExplorerSelection {
    pub paths: Vec<String>,
    pub expected_identities: Vec<(String, String)>,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceNavigationSelection {
    pub selected_object_folder_path: Option<String>,
    pub explorer_sub_path: Option<String>,
    pub current_path: Vec<String>,
    pub reconciliation_status: WorkspaceSelectionReconciliationStatus,
    pub reconciliation_reason: Option<WorkspaceSelectionReconciliationReason>,
    pub affected_paths: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspacePreviewInput {
    pub game_id: String,
    pub explorer_sub_path: Option<String>,
    pub selected_mod_path: Option<String>,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspacePreviewRequestIdentity {
    pub game_id: String,
    pub explorer_sub_path: Option<String>,
    pub selected_mod_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspacePreviewContextStatus {
    Ready,
    ContextStale,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspacePreviewSelection {
    pub selected_mod_path: Option<String>,
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
    pub conflicts: Vec<crate::modules::workspace::application::explorer::types::ConflictGroup>,
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
    pub source_state: WorkspaceSourceState,
    pub recovery_status: WorkspaceRecoveryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRecoveryStatus {
    Ready,
    Syncing,
    Failed,
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
pub struct WorkspaceStructureViewModel {
    pub objects: Vec<WorkspaceObjectNode>,
    pub explorer: WorkspaceExplorer,
    pub selection: WorkspaceNavigationSelection,
    pub runtime: WorkspaceRuntime,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspacePreviewResult {
    pub request_identity: WorkspacePreviewRequestIdentity,
    pub context_status: WorkspacePreviewContextStatus,
    pub preview: WorkspacePreview,
    pub selection: WorkspacePreviewSelection,
}

/// Legacy aggregate used only by the pre-split workspace fixtures. It is not
/// exposed through Tauri and prevents parity tests from becoming a second
/// runtime path.
#[cfg(test)]
#[derive(Clone)]
pub struct WorkspaceViewModelInput {
    pub filter: ObjectFilter,
    pub selected_object_folder_path: Option<String>,
    pub explorer_sub_path: Option<String>,
    pub selected_mod_path: Option<String>,
}

#[cfg(test)]
#[derive(Clone)]
pub struct WorkspaceSelection {
    pub selected_object_folder_path: Option<String>,
    pub explorer_sub_path: Option<String>,
    pub selected_mod_path: Option<String>,
    pub current_path: Vec<String>,
    pub reconciliation_status: WorkspaceSelectionReconciliationStatus,
    pub reconciliation_reason: Option<WorkspaceSelectionReconciliationReason>,
    pub affected_paths: Vec<String>,
}

#[cfg(test)]
#[derive(Clone)]
pub struct WorkspaceViewModel {
    pub objects: Vec<WorkspaceObjectNode>,
    pub explorer: WorkspaceExplorer,
    pub preview: WorkspacePreview,
    pub selection: WorkspaceSelection,
    pub runtime: WorkspaceRuntime,
}
