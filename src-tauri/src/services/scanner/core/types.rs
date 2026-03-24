use crate::database::models::ItemStatus;
use crate::services::scanner::core::walker;
use crate::services::scanner::deep_matcher::{
    Candidate, Confidence, MatchStatus, StagedMatchResult,
};
use serde::{Deserialize, Serialize};

use std::path::PathBuf;

/// Represents a folder naming collision discovered during sync or organize.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CollisionInfo {
    /// Unique ID for the collision (usually hash of target path)
    pub id: String,
    /// The mod folder we are trying to move/place
    pub source_path: String,
    /// The destination path that already exists
    pub target_path: String,
    /// Name of the object this mod belongs to
    pub object_name: String,
    /// ID of the mod already occupying the target path (if indexed)
    pub existing_mod_id: Option<String>,
}

/// User resolution choice for a folder collision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CollisionResolution {
    /// Rename the incoming mod (e.g. Mod (2))
    Rename,
    /// Skip this mod entirely
    Skip,
    /// Delete existing and replace with new
    Overwrite,
    /// Merge contents (if both are folders)
    Merge,
}

// ─── Event Types ───────────────────────────────────────────────────

/// Progress events streamed to frontend via `Channel<ScanEvent>`.
#[derive(Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ScanEvent {
    /// Scan has started, includes total folder count.
    #[serde(rename_all = "camelCase")]
    Started {
        #[specta(type = f64)]
        total_folders: usize,
    },
    /// One folder has been processed.
    #[serde(rename_all = "camelCase")]
    Progress {
        #[specta(type = f64)]
        current: usize,
        #[specta(type = f64)]
        total: usize,
        folder_name: String,
        #[specta(type = f64)]
        elapsed_ms: u64,
        #[specta(type = f64)]
        eta_ms: u64,
    },
    /// A match was found for a folder.
    #[serde(rename_all = "camelCase")]
    Matched {
        folder_name: String,
        object_name: String,
        confidence: String,
    },
    /// Scan is complete.
    #[serde(rename_all = "camelCase")]
    Finished {
        #[specta(type = f64)]
        matched: usize,
        #[specta(type = f64)]
        unmatched: usize,
    },
}

// ─── Result Types ──────────────────────────────────────────────────

/// A single scan result item returned per mod folder.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultItem {
    pub path: String,
    pub raw_name: String,
    pub display_name: String,
    pub is_disabled: bool,
    pub matched_object: Option<String>,
    pub match_level: String,
    pub confidence: String,
    pub confidence_score: u8,
    pub match_detail: Option<String>,
    pub detected_skin: Option<String>,
    /// Canonical folder name for this skin variant (first alias).
    pub skin_folder_name: Option<String>,
    pub thumbnail_path: Option<String>,
}

// ─── Helpers ───────────────────────────────────────────────────────

pub fn match_status_label(status: &MatchStatus) -> &'static str {
    match status {
        MatchStatus::AutoMatched => "AutoMatched",
        MatchStatus::NeedsReview => "NeedsReview",
        MatchStatus::NoMatch => "NoMatch",
    }
}

pub fn staged_confidence_label(result: &StagedMatchResult) -> &'static str {
    match result.status {
        MatchStatus::AutoMatched => result
            .best
            .as_ref()
            .or_else(|| result.candidates_topk.first())
            .map(|candidate| confidence_value_label(&candidate.confidence))
            .unwrap_or("High"),
        MatchStatus::NeedsReview => "Low",
        MatchStatus::NoMatch => "None",
    }
}

pub fn staged_primary_candidate(result: &StagedMatchResult) -> Option<&Candidate> {
    match result.status {
        MatchStatus::AutoMatched | MatchStatus::NeedsReview => result
            .best
            .as_ref()
            .or_else(|| result.candidates_topk.first()),
        MatchStatus::NoMatch => None,
    }
}

pub fn staged_match_detail(result: &StagedMatchResult) -> String {
    result.summary()
}

pub fn staged_auto_matched_object_name(result: &StagedMatchResult) -> Option<&str> {
    if result.status != MatchStatus::AutoMatched {
        return None;
    }

    staged_primary_candidate(result).map(|candidate| candidate.name.as_str())
}

pub fn build_result_item_from_staged(
    candidate: &walker::ModCandidate,
    match_result: &StagedMatchResult,
    thumb: Option<PathBuf>,
    detected_skin: Option<String>,
    skin_folder_name: Option<String>,
) -> ScanResultItem {
    ScanResultItem {
        path: candidate.path.to_string_lossy().to_string(),
        raw_name: candidate.raw_name.clone(),
        display_name: candidate.display_name.clone(),
        is_disabled: candidate.is_disabled,
        matched_object: staged_auto_matched_object_name(match_result)
            .map(std::string::ToString::to_string),
        match_level: match_status_label(&match_result.status).to_string(),
        confidence: staged_confidence_label(match_result).to_string(),
        confidence_score: match_result.confidence_score(),
        match_detail: Some(staged_match_detail(match_result)),
        detected_skin,
        skin_folder_name,
        thumbnail_path: thumb.map(|p| p.to_string_lossy().to_string()),
    }
}

fn confidence_value_label(confidence: &Confidence) -> &'static str {
    match confidence {
        Confidence::Excellent => "Excellent",
        Confidence::High => "High",
        Confidence::Medium => "Medium",
        Confidence::Low => "Low",
        Confidence::None => "None",
    }
}

#[cfg(test)]
#[path = "tests/types_tests.rs"]
mod tests;

/// Represents a row in the `objects` table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct GameObject {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub folder_path: String,
    pub folder_path_key: String,
    pub status: ItemStatus,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub tags: String,
    pub metadata: String,
    pub hash_db: Option<crate::database::models::HashDbPayload>,
    pub custom_skins: Option<crate::database::models::CustomSkinsPayload>,
    pub thumbnail_path: Option<String>,
    pub is_pinned: bool,
    pub is_auto_sync: bool,
    pub created_at: String,
}
