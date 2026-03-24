use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Corridor — Represents one side of the Safe/Unsafe corridor for a game
// ---------------------------------------------------------------------------

/// The composite key that uniquely identifies a corridor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CorridorId {
    pub game_id: String,
    pub is_safe: bool,
}

impl CorridorId {
    pub fn new(game_id: impl Into<String>, is_safe: bool) -> Self {
        Self {
            game_id: game_id.into(),
            is_safe,
        }
    }

    /// Returns the i32 representation expected by SQLite (0 or 1).
    pub fn is_safe_i32(&self) -> i32 {
        if self.is_safe {
            1
        } else {
            0
        }
    }

    /// Returns the human-readable mode label.
    pub fn mode_label(&self) -> &'static str {
        if self.is_safe {
            "Safe"
        } else {
            "Unsafe"
        }
    }
}

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
    pub undo_collection_id: Option<String>,
    pub current_signature: String,
    pub is_dirty: bool,
}

/// Preview of a Safe/Unsafe switch, containing current vs target members.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CorridorSwitchPreview {
    pub leaving_state_name: Option<String>,
    pub leaving_mods: Vec<crate::domain::collection::CollectionMod>,
    pub leaving_objects: Vec<crate::domain::collection::CollectionObject>,
    pub target_state_name: Option<String>,
    pub target_state_kind: String, // "none" or "undo_snapshot"
    pub target_mods: Vec<crate::domain::collection::CollectionMod>,
    pub target_objects: Vec<crate::domain::collection::CollectionObject>,
}

/// Result of a corridor switch operation.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SwitchResult {
    pub success: bool,
    pub mods_disabled: usize,
    pub mods_restored: usize,
    pub new_signature: String,
    /// The collection ID that was restored in the target corridor (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restored_collection_id: Option<String>,
}
