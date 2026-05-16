use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::collection::CollectionReferenceImpact;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum DiskReconcileReason {
    StartupBoot,
    OnboardingCompleted,
    ModsViewEntered,
    WindowRefocused,
    WatcherBatch,
    ManualRepair,
    GameSwitched,
    InternalMutation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum DiskReconcilePathKind {
    Object,
    Mod,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum DiskReconcileStatus {
    Applied,
    SourceUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiskReconcilePathUpdate {
    pub from: String,
    pub to: String,
    pub kind: DiskReconcilePathKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiskReconcileChangeCounts {
    pub added: u32,
    pub removed: u32,
    pub renamed: u32,
    pub modified: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiskReconcileChangeSummary {
    pub object_changes: DiskReconcileChangeCounts,
    pub mod_changes: DiskReconcileChangeCounts,
    pub object_sample_names: Vec<String>,
    pub mod_sample_names: Vec<String>,
    pub has_user_visible_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DiskReconcileResult {
    pub game_id: String,
    pub reason: DiskReconcileReason,
    pub status: DiskReconcileStatus,
    pub error_message: Option<String>,
    pub changed_roots: Vec<String>,
    pub objects_changed: bool,
    pub folders_changed: bool,
    pub collections_changed: bool,
    pub runtime_file_changed: bool,
    pub overlay_refresh_triggered: bool,
    pub thumbnail_roots: Vec<String>,
    pub cleared_selection_paths: Vec<String>,
    pub path_updates: Vec<DiskReconcilePathUpdate>,
    pub collection_reference_impact: CollectionReferenceImpact,
    pub change_summary: DiskReconcileChangeSummary,
}
