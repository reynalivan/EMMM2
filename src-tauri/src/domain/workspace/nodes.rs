use crate::repo::object_repo::ObjectSummary;
use serde::Serialize;
use std::collections::HashMap;

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
