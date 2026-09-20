use serde::{Deserialize, Serialize};

use super::*;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSwitchOriginSurface {
    FolderGrid,
    Preview,
    ObjectList,
    Collections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSwitchStatus {
    Applied,
    RequiresDuplicateResolution,
    RequiresParentEnable,
    Noop,
}

/// A disabled ancestor blocks a folder even when the folder itself has no
/// disabled prefix. This payload is derived after disk preflight so the UI
/// can explain the exact prerequisite without guessing from visible nodes.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceParentEnableRequirement {
    pub confirmation_token: String,
    pub requested_target: WorkspaceParentEnableImpact,
    pub parents: Vec<WorkspaceParentEnableParent>,
    pub will_activate: Vec<WorkspaceParentEnableImpact>,
    pub stay_disabled: Vec<WorkspaceParentEnableImpact>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceParentEnableParent {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceParentEnableImpact {
    pub path: String,
    pub name: String,
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
    /// Parent activation is independent from duplicate conflict policy. A
    /// boolean avoids a combinatorial set of resolution enum variants.
    #[serde(default)]
    pub enable_disabled_ancestors: bool,
    #[serde(default)]
    pub parent_enable_confirmation: Option<String>,
    pub origin_surface: WorkspaceSwitchOriginSurface,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceSwitchResult {
    pub status: WorkspaceSwitchStatus,
    pub primary_path: Option<String>,
    pub changed_folder_paths: Vec<String>,
    pub changed_object_ids: Vec<String>,
    pub duplicates: Vec<WorkspaceSwitchDuplicate>,
    pub parent_enable_requirement: Option<WorkspaceParentEnableRequirement>,
    pub impact: WorkspaceImpact,
    pub sync_warning: Option<crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning>,
    pub runtime_sync_generation: Option<u64>,
}
