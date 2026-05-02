use crate::repo::object_repo::{ObjectFilter, ObjectSummary};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceCapabilities {
    pub can_toggle: bool,
    pub can_rename: bool,
    pub can_delete: bool,
    pub can_move: bool,
    pub can_toggle_safe: bool,
    pub can_sync: bool,
    pub can_enable_only_this: bool,
    pub can_pin: bool,
    pub can_edit_metadata: bool,
    pub can_reveal_in_explorer: bool,
    pub can_move_category: bool,
    pub can_open_in_explorer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceReasonCode {
    DisabledByContainer,
    ObjectFolderDisabled,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceReason {
    pub code: WorkspaceReasonCode,
    pub args: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceWarningCode {
    FolderWarning,
    InactiveReason,
    NamingConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSwitchState {
    Enabled,
    Disabled,
    EffectivelyDisabled,
    BlockedByAncestor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSwitchPolicyKey {
    Mod,
    Object,
    Blocked,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceObjectNode {
    #[serde(flatten)]
    pub object: ObjectSummary,
    pub node_kind: WorkspaceNodeKind,
    pub display_mode: WorkspaceDisplayMode,
    pub type_chip: Option<WorkspaceTypeChip>,
    pub display_name: String,
    pub is_effectively_active: bool,
    pub inactive_reason: Option<WorkspaceReason>,
    pub warning_state: WorkspaceWarningState,
    pub primary_warning: Option<WorkspaceWarning>,
    pub switch_state: WorkspaceSwitchState,
    pub switch_reason: Option<WorkspaceReason>,
    pub switch_policy_key: WorkspaceSwitchPolicyKey,
    pub capabilities: WorkspaceCapabilities,
}

#[derive(Clone, Serialize, specta::Type)]
#[serde(untagged)]
pub enum WorkspaceNode {
    Explorer(WorkspaceExplorerNode),
    Object(WorkspaceObjectNode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceNodeKind {
    Object,
    Container,
    TerminalMod,
    InactiveBranch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceDisplayMode {
    ContainerFolder,
    ModPack,
    Variant,
    FlatMod,
    InternalAssets,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceTypeChip {
    ModPack,
    Variant,
    FlatMod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceWarningState {
    None,
    Warning,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceWarning {
    pub code: WorkspaceWarningCode,
    pub args: HashMap<String, String>,
    pub state: WorkspaceWarningState,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceExplorerNode {
    pub node_type: String,
    pub classification_reasons: Vec<String>,
    pub id: Option<String>,
    pub owner_object_id: Option<String>,
    pub owner_object_folder_path: Option<String>,
    pub name: String,
    pub folder_name: String,
    pub path: String,
    pub is_enabled: bool,
    pub is_directory: bool,
    pub thumbnail_path: Option<String>,
    #[specta(type = f64)]
    pub modified_at: u64,
    #[specta(type = f64)]
    pub size_bytes: u64,
    pub has_info_json: bool,
    pub is_favorite: bool,
    pub is_misplaced: bool,
    pub is_safe: bool,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub category: Option<String>,
    pub conflict_group_id: Option<String>,
    pub conflict_state: Option<String>,
    pub warnings: Vec<String>,
    pub node_kind: WorkspaceNodeKind,
    pub display_mode: WorkspaceDisplayMode,
    pub type_chip: Option<WorkspaceTypeChip>,
    pub display_name: String,
    pub is_effectively_active: bool,
    pub ancestor_disabled: bool,
    pub inactive_reason: Option<WorkspaceReason>,
    pub warning_state: WorkspaceWarningState,
    pub primary_warning: Option<WorkspaceWarning>,
    pub switch_state: WorkspaceSwitchState,
    pub switch_reason: Option<WorkspaceReason>,
    pub switch_policy_key: WorkspaceSwitchPolicyKey,
    pub capabilities: WorkspaceCapabilities,
    pub can_navigate: bool,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSwitchTargetKind {
    ModPath,
    ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceSwitchTarget {
    pub kind: WorkspaceSwitchTargetKind,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSwitchResolution {
    Normal,
    ForceEnable,
    EnableOnlyThis,
    EnableParentThenContinue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSwitchOriginSurface {
    FolderGrid,
    Preview,
    ObjectList,
    Collections,
    Corridor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSwitchStatus {
    Applied,
    RequiresDuplicateResolution,
    Noop,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceSwitchDuplicate {
    pub mod_id: String,
    pub object_id: String,
    pub folder_path: String,
    pub actual_name: String,
    pub is_variant: bool,
    pub parent_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WorkspaceSwitchInput {
    pub game_id: String,
    pub target: WorkspaceSwitchTarget,
    pub desired_enabled: bool,
    pub resolution: WorkspaceSwitchResolution,
    pub origin_surface: WorkspaceSwitchOriginSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceRefreshScope {
    WorkspaceChanged,
    ObjectRowsChanged,
    FolderStructureChanged,
    FolderMetadataChanged,
    PreviewChanged,
    ThumbnailChanged,
    ConflictsChanged,
    CorridorChanged,
    CollectionsChanged,
    DashboardChanged,
    ActiveKeybindingsChanged,
    TrashChanged,
    SettingsChanged,
    BrowserDownloadsChanged,
    BrowserImportQueueChanged,
    BrowserHomepageChanged,
    DedupChanged,
    DedupReportChanged,
    ScannerChanged,
    PinsChanged,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspacePathRewrite {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceImpact {
    pub rewrites: Vec<WorkspacePathRewrite>,
    pub cleared_targets: Vec<String>,
    pub changed_object_ids: Vec<String>,
    pub changed_folder_paths: Vec<String>,
    pub refresh_scopes: Vec<WorkspaceRefreshScope>,
    pub projection_dirty: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceSwitchResult {
    pub status: WorkspaceSwitchStatus,
    pub primary_path: Option<String>,
    pub changed_folder_paths: Vec<String>,
    pub changed_object_ids: Vec<String>,
    pub duplicates: Vec<WorkspaceSwitchDuplicate>,
    pub impact: WorkspaceImpact,
}

#[derive(Clone, Serialize, specta::Type)]
pub struct WorkspaceViewModel {
    pub objects: Vec<WorkspaceObjectNode>,
    pub explorer: WorkspaceExplorer,
    pub preview: WorkspacePreview,
    pub selection: WorkspaceSelection,
    pub runtime: WorkspaceRuntime,
}
