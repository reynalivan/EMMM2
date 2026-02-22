use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct SyncResult {
    pub total_scanned: usize,
    pub new_mods: usize,
    pub updated_mods: usize,
    pub deleted_mods: usize,
    pub new_objects: usize,
}

/// A single preview item returned by scan_preview (before user confirms).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanPreviewItem {
    pub folder_path: String,
    pub display_name: String,
    pub is_disabled: bool,
    pub matched_object: Option<String>,
    pub match_level: String,
    pub confidence: String,
    pub confidence_score: u8,
    pub match_detail: Option<String>,
    pub detected_skin: Option<String>,
    pub object_type: Option<String>,
    pub thumbnail_path: Option<String>,
    /// Tags from MasterDB entry (JSON array string)
    pub tags_json: Option<String>,
    /// Metadata from MasterDB entry (JSON object string)
    pub metadata_json: Option<String>,
    /// Whether this mod already exists in DB
    pub already_in_db: bool,
    /// Whether this mod already has an object_id assigned
    pub already_matched: bool,
}

/// User-confirmed item sent back from the review modal.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedScanItem {
    pub folder_path: String,
    pub display_name: String,
    pub is_disabled: bool,
    pub matched_object: Option<String>,
    pub object_type: Option<String>,
    pub thumbnail_path: Option<String>,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub skip: bool,
}
