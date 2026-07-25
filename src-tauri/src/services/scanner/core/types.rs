use crate::services::scanner::deep_matcher::{
    Candidate, Confidence, MatchStatus, StagedMatchResult,
};
use serde::{Deserialize, Serialize};

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

pub use crate::domain::models::GameObject;
