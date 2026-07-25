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

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceSwitchResult {
    pub status: WorkspaceSwitchStatus,
    pub primary_path: Option<String>,
    pub changed_folder_paths: Vec<String>,
    pub changed_object_ids: Vec<String>,
    pub duplicates: Vec<WorkspaceSwitchDuplicate>,
    pub impact: WorkspaceImpact,
}
