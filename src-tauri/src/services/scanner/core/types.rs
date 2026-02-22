use crate::services::scanner::core::walker;
use crate::services::scanner::deep_matcher::{
    Candidate, Confidence, MatchStatus, StagedMatchResult,
};
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
    Progress {
        current: usize,
        total: usize,
        folder_name: String,
        elapsed_ms: u64,
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
    #[sqlx(default)]
    pub is_auto_sync: bool,
    pub created_at: String,
}
