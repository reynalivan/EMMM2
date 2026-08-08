use serde::{Deserialize, Serialize};

/// Which side of the Safe Mode corridor the app is operating in.
///
/// Deliberately not serde/specta: a corridor is derived server-side
/// (`ConfigService::current_corridor`) and can never arrive over IPC — the
/// privacy gate stops being spoofable the moment it stops being a wire type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corridor {
    Safe,
    Unsafe,
}

impl Corridor {
    /// From a stored `is_safe` flag (corridor rows, collection rows).
    pub fn from_is_safe(is_safe: bool) -> Self {
        if is_safe {
            Self::Safe
        } else {
            Self::Unsafe
        }
    }

    /// The `is_safe` value this corridor stores and filters by.
    pub fn is_safe(self) -> bool {
        matches!(self, Self::Safe)
    }
}

// ---------------------------------------------------------------------------
// Corridor — Represents one side of the Safe/Unsafe corridor for a game
// ---------------------------------------------------------------------------

/// Full state of a corridor row as stored in the DB (table: `corridor_state`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct CorridorState {
    pub game_id: String,
    pub is_safe: bool,
    pub active_collection_id: Option<String>,
}

/// Lightweight snapshot returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CorridorSnapshot {
    pub game_id: String,
    pub is_safe: bool,
    pub active_collection_id: Option<String>,
    pub active_collection_name: Option<String>,
    pub current_signature: String,
    pub is_dirty: bool,
    pub current_mods: Vec<crate::domain::collection::CollectionMod>,
    pub current_objects: Vec<crate::domain::collection::CollectionObject>,
    pub current_tree_nodes: Vec<crate::domain::collection::PreviewTreeNode>,
    pub projected_state: crate::domain::collection::ProjectedCollectionState,
}
