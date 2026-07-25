use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Corridor — Represents one side of the Safe/Unsafe corridor for a game
// ---------------------------------------------------------------------------

/// Full state of a corridor row as stored in the DB (table: `corridor_state`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct CorridorState {
    pub game_id: String,
    pub is_safe: bool,
    pub active_collection_id: Option<String>,
    pub undo_collection_id: Option<String>,
}

/// Runtime cache of the physical corridor state (table: `corridor_runtime_cache`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CorridorRuntime {
    pub game_id: String,
    pub is_safe: bool,
    pub matched_collection_id: Option<String>,
    pub state_kind: String,
    pub state_name: Option<String>,
    pub signature: String,
    pub snapshot_json: String,
    pub snapshot_source: String,
    pub updated_at: String,
}

/// Lightweight snapshot returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CorridorSnapshot {
    pub game_id: String,
    pub is_safe: bool,
    pub active_collection_id: Option<String>,
    pub active_collection_name: Option<String>,
    pub active_collection_is_unsaved: bool,
    pub undo_collection_id: Option<String>,
    pub current_signature: String,
    pub is_dirty: bool,
    pub current_mods: Vec<crate::domain::collection::CollectionMod>,
    pub current_objects: Vec<crate::domain::collection::CollectionObject>,
    pub current_tree_nodes: Vec<crate::domain::collection::PreviewTreeNode>,
    pub projected_state: crate::domain::collection::ProjectedCollectionState,
}
