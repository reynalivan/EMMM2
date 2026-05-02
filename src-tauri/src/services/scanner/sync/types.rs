use crate::services::scanner::core::types::CollisionInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    #[specta(type = f64)]
    pub total_scanned: usize,
    #[specta(type = f64)]
    pub new_mods: usize,
    #[specta(type = f64)]
    pub updated_mods: usize,
    #[specta(type = f64)]
    pub deleted_mods: usize,
    #[specta(type = f64)]
    pub new_objects: usize,
    pub collisions: Vec<CollisionInfo>,
}

/// A single preview item returned by scan_preview (before user confirms).
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanPreviewItem {
    pub folder_path: String,
    pub display_name: String,
    pub is_disabled: bool,
    pub matched_entry_key: Option<String>,
    pub matched_alias_name: Option<String>,
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
    /// Top-k scored candidates from the matcher (name + percentage).
    pub scored_candidates: Vec<ScoredCandidate>,
    /// Optional hash_db from MasterDB entry (JSON object string)
    pub hash_db_json: Option<String>,
    /// Optional custom_skins from MasterDB entry (JSON array string)
    pub custom_skins_json: Option<String>,
    pub db_thumbnail: Option<String>,
}

/// A lightweight scored candidate for the frontend dropdown.
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScoredCandidate {
    pub name: String,
    pub object_type: String,
    pub score_pct: u8,
}

/// User-confirmed item sent back from the review modal.
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedScanItem {
    pub folder_path: String,
    pub display_name: String,
    pub is_disabled: bool,
    pub matched_entry_key: Option<String>,
    pub matched_alias_name: Option<String>,
    pub matched_confidence: Option<f64>,
    pub matched_reason: Option<String>,
    pub object_type: Option<String>,
    pub thumbnail_path: Option<String>,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub hash_db_json: Option<String>,
    pub custom_skins_json: Option<String>,
    pub db_thumbnail: Option<String>,
    pub skip: bool,
    #[serde(default)]
    pub move_from_temp: bool,
}
