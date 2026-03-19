use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub game_id: String,
    pub is_safe_context: bool,
    pub member_count: usize,
    pub is_last_unsaved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDetails {
    pub collection: Collection,
    pub mod_ids: Vec<String>,
    pub object_states: Vec<CollectionObjectState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionObjectState {
    pub object_id: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionPreviewMod {
    pub id: String,
    pub actual_name: String,
    pub folder_path: String,
    pub is_safe: bool,
    pub object_id: Option<String>,
    pub object_name: Option<String>,
    pub object_type: Option<String>,
    pub node_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeObjectState {
    pub object_id: String,
    pub name: String,
    pub object_type: String,
    pub is_enabled: bool,
    pub thumbnail_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorRuntimeSnapshot {
    pub game_id: String,
    pub is_safe: bool,
    pub active_collection_id: Option<String>,
    pub state_name: Option<String>,
    pub state_kind: CollectionStateKind,
    pub roots: Vec<CollectionPreviewMod>,
    pub object_states: Vec<RuntimeObjectState>,
    pub signature: String,
    pub snapshot_source: String,
    pub reconciled_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionRuntimePreview {
    pub collection: Collection,
    pub roots: Vec<CollectionPreviewMod>,
    pub object_states: Vec<RuntimeObjectState>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionRuntimeSummary {
    pub root_count: usize,
    pub object_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalCollectionSnapshot {
    pub roots: Vec<CollectionPreviewMod>,
    pub object_states: Vec<RuntimeObjectState>,
    pub summary: CollectionRuntimeSummary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollectionStateKind {
    Named,
    Unsaved,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionInput {
    pub name: String,
    pub game_id: String,
    pub is_safe_context: bool,
    pub auto_snapshot: Option<bool>,
    pub mod_ids: Vec<String>,
    pub object_states: Option<Vec<CollectionObjectState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCollectionInput {
    pub id: String,
    pub game_id: String,
    pub name: Option<String>,
    pub is_safe_context: Option<bool>,
    pub mod_ids: Option<Vec<String>>,
    pub object_states: Option<Vec<CollectionObjectState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyCollectionResult {
    pub changed_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyCollectionProgressPhase {
    Idle,
    Preparing,
    Renaming,
    UpdatingDb,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyCollectionProgress {
    pub phase: ApplyCollectionProgressPhase,
    pub completed: usize,
    pub total: usize,
    pub current_item: Option<String>,
    pub collection_name: Option<String>,
    pub is_safe: Option<bool>,
    pub error: Option<String>,
}

impl ApplyCollectionProgress {
    pub fn idle() -> Self {
        Self {
            phase: ApplyCollectionProgressPhase::Idle,
            completed: 0,
            total: 0,
            current_item: None,
            collection_name: None,
            is_safe: None,
            error: None,
        }
    }
}

pub struct ModState {
    pub id: String,
    pub folder_path: String,
    pub status: String,
    pub object_id: Option<String>,
}
