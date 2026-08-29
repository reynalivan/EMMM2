use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    Clean,
    Modified,
    Unsaved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum LastChangesSource {
    Live,
    Draft,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct LastChangesSnapshot {
    pub source: LastChangesSource,
    pub collection_id: Option<String>,
    pub base_collection_id: Option<String>,
    pub can_restore: bool,
}

/// The compact runtime read model used by global collection status surfaces.
/// Full members and preview trees stay behind the collection preview queries.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CollectionRuntimeDescriptor {
    pub game_id: String,
    pub active_collection_id: Option<String>,
    pub active_collection_name: Option<String>,
    pub runtime_status: RuntimeStatus,
    #[specta(type = f64)]
    pub missing_count: usize,
    pub safety: RuntimeSafetySummary,
    pub counts: RuntimeCounts,
    pub last_changes: Option<LastChangesSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct RuntimeSafetySummary {
    pub is_safe: bool,
    pub is_safety_classified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct RuntimeCounts {
    #[specta(type = f64)]
    pub active_mod_count: usize,
    #[specta(type = f64)]
    pub object_count: usize,
    #[specta(type = f64)]
    pub enabled_object_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CollectionRuntimeSnapshot {
    pub game_id: String,
    pub active_collection_id: Option<String>,
    pub active_collection_name: Option<String>,
    pub current_signature: String,
    pub is_dirty: bool,
    pub runtime_status: RuntimeStatus,
    pub is_safe: bool,
    pub is_safety_classified: bool,
    #[specta(type = f64)]
    pub missing_count: usize,
    pub last_changes: Option<LastChangesSnapshot>,
    pub current_mods: Vec<crate::domain::collection::CollectionMod>,
    pub current_objects: Vec<crate::domain::collection::CollectionObject>,
    pub current_tree_nodes: Vec<crate::domain::collection::PreviewTreeNode>,
    pub projected_state: crate::domain::collection::ProjectedCollectionState,
}
