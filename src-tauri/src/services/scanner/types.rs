use crate::services::scanner::deep_matcher::{MatchLevel, MatchResult};
use crate::services::scanner::walker;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Event Types ───────────────────────────────────────────────────

/// Progress events streamed to frontend via `Channel<ScanEvent>`.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ScanEvent {
    /// Scan has started, includes total folder count.
    #[serde(rename_all = "camelCase")]
    Started { total_folders: usize },
    /// One folder has been processed.
    #[serde(rename_all = "camelCase")]
    Progress { current: usize, folder_name: String },
    /// A match was found for a folder.
    #[serde(rename_all = "camelCase")]
    Matched {
        folder_name: String,
        object_name: String,
        confidence: String,
    },
    /// Scan is complete.
    #[serde(rename_all = "camelCase")]
    Finished { matched: usize, unmatched: usize },
}

// ─── Result Types ──────────────────────────────────────────────────

/// A single scan result item returned per mod folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultItem {
    pub path: String,
    pub raw_name: String,
    pub display_name: String,
    pub is_disabled: bool,
    pub matched_object: Option<String>,
    pub match_level: String,
    pub confidence: String,
    pub match_detail: Option<String>,
    pub detected_skin: Option<String>,
    /// Canonical folder name for this skin variant (first alias).
    pub skin_folder_name: Option<String>,
    pub thumbnail_path: Option<String>,
}

// ─── Helpers ───────────────────────────────────────────────────────

pub fn match_level_label(level: &MatchLevel) -> &'static str {
    match level {
        MatchLevel::L1Name => "L1-Name",
        MatchLevel::L2Token => "L2-Token",
        MatchLevel::L3Content => "L3-Content",
        MatchLevel::L4Ai => "L4-AI",
        MatchLevel::L5Fuzzy => "L5-Fuzzy",
        MatchLevel::Unmatched => "Unmatched",
    }
}

pub fn confidence_label(level: &MatchLevel) -> &'static str {
    match level {
        MatchLevel::L1Name | MatchLevel::L2Token => "High",
        MatchLevel::L3Content => "Medium",
        MatchLevel::L4Ai => "Medium",
        MatchLevel::L5Fuzzy => "Low",
        MatchLevel::Unmatched => "None",
    }
}

pub fn build_result_item(
    candidate: &walker::ModCandidate,
    match_result: &MatchResult,
    thumb: Option<PathBuf>,
) -> ScanResultItem {
    ScanResultItem {
        path: candidate.path.to_string_lossy().to_string(),
        raw_name: candidate.raw_name.clone(),
        display_name: candidate.display_name.clone(),
        is_disabled: candidate.is_disabled,
        matched_object: if match_result.object_name.is_empty() {
            None
        } else {
            Some(match_result.object_name.clone())
        },
        match_level: match_level_label(&match_result.level).to_string(),
        confidence: confidence_label(&match_result.level).to_string(),
        match_detail: if match_result.detail.is_empty() {
            None
        } else {
            Some(match_result.detail.clone())
        },
        detected_skin: match_result.detected_skin.clone(),
        skin_folder_name: match_result.skin_folder_name.clone(),
        thumbnail_path: thumb.map(|p| p.to_string_lossy().to_string()),
    }
}

/// Represents a row in the `objects` table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameObject {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub sort_order: i64,
    // stored as JSON string in DB, but we might want to deserialize it if we use sqlx json features
    // For now, let's keep it as String to match existing code or use sqlx::types::Json
    // Migration says TEXT for tags and metadata? Let's check migration 004.
    // Assuming TEXT for simplicity as per TRD "JSON string".
    pub tags: String,
    pub metadata: String,
    pub thumbnail_path: Option<String>,
    pub is_safe: bool,
    pub is_pinned: bool,
    pub created_at: String,
}
