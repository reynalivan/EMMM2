use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub game_id: String,
    pub is_safe_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDetails {
    pub collection: Collection,
    pub mod_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCollectionInput {
    pub name: String,
    pub game_id: String,
    pub is_safe_context: bool,
    pub mod_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCollectionInput {
    pub id: String,
    pub game_id: String,
    pub name: Option<String>,
    pub is_safe_context: Option<bool>,
    pub mod_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyCollectionResult {
    pub changed_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoCollectionResult {
    pub restored_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub mod_id: String,
    pub previous_status: String,
}

#[derive(Debug, Clone)]
pub struct UndoSnapshot {
    pub game_id: String,
    pub entries: Vec<SnapshotEntry>,
}

pub struct CollectionsUndoState {
    snapshot: Mutex<Option<UndoSnapshot>>,
}

impl CollectionsUndoState {
    pub fn new() -> Self {
        Self {
            snapshot: Mutex::new(None),
        }
    }
}

impl Default for CollectionsUndoState {
    fn default() -> Self {
        Self::new()
    }
}

impl CollectionsUndoState {
    pub fn set(&self, snapshot: UndoSnapshot) {
        if let Ok(mut guard) = self.snapshot.lock() {
            *guard = Some(snapshot);
        }
    }

    pub fn take(&self) -> Option<UndoSnapshot> {
        self.snapshot.lock().ok().and_then(|mut guard| guard.take())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCollectionPayload {
    pub version: u32,
    pub name: String,
    pub game_id: String,
    pub is_safe_context: bool,
    pub items: Vec<ExportCollectionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCollectionItem {
    pub mod_id: String,
    pub actual_name: String,
    pub folder_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportCollectionResult {
    pub collection_id: String,
    pub imported_count: usize,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ModState {
    pub id: String,
    pub folder_path: String,
    pub status: String,
}
