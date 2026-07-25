//! Trash entry metadata and delete-result payloads.

use crate::domain::collection::CollectionReferenceImpact;
use serde::{Deserialize, Serialize};

/// Metadata stored alongside each trashed item for restore.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TrashMetadata {
    /// Unique ID for this trash entry
    pub id: String,
    /// Original absolute path before deletion
    pub original_path: String,
    /// Display name of the mod folder
    pub original_name: String,
    /// ISO 8601 timestamp of deletion
    pub deleted_at: String,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Associated game_id (for DB cleanup)
    pub game_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DeleteModResult {
    pub collection_impact: CollectionReferenceImpact,
}
