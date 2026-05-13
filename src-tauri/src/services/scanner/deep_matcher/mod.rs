//! Deep Matcher Pipeline — The "Brain" of EMMM.
//!
//! Identifies mod categories by running a staged Quick → FullScoring pipeline
//! with evidence-based scoring, acceptance gates, and optional AI reranking.
// Module structure
pub mod analysis;
pub mod models;
pub mod pipeline;
pub mod state;

// Test-only modules
#[cfg(test)]
#[path = "tests/alias_resolution_tests.rs"]
mod alias_resolution_tests;
#[cfg(test)]
pub mod golden_corpus;
#[cfg(test)]
#[path = "tests/required_tests.rs"]
mod required_tests;

// Public types and matcher entrypoints used by commands and sync services.
pub use analysis::skin_resolver::detect_skin_for_staged;
pub use models::types::{
    sort_candidates_deterministic, Candidate, Confidence, CustomSkin, DbEntry, Evidence, MatchMode,
    MatchStatus, Reason, ScoreState, StagedMatchResult, MAX_EVIDENCE_HASHES, MAX_EVIDENCE_SECTIONS,
    MAX_EVIDENCE_TOKENS, MAX_REASONS_PER_CANDIDATE,
};
pub use pipeline::full_pipeline::{match_folder_full, score_forced_candidates};
pub use pipeline::quick_pipeline::match_folder_quick;
pub use state::master_db::MasterDb;

use super::core::walker::{FolderContent, ModCandidate};
use analysis::content::IniTokenizationConfig;

/// Phased matcher: Try Quick first, then fall back to FullScoring if NeedsReview or NoMatch.
/// Preserves NeedsReview and NoMatch statuses without auto-apply.
pub fn match_folder_phased<'a>(
    candidate: &ModCandidate,
    db: &MasterDb,
    content: &FolderContent,
    ini_config: &IniTokenizationConfig,
    ai_config: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig<'a>,
) -> StagedMatchResult {
    let mut local_cache = state::signal_cache::SignalCache::new();
    match_folder_phased_cached(
        candidate,
        db,
        content,
        ini_config,
        ai_config,
        &mut local_cache,
    )
}

/// Same as `match_folder_phased` but accepts an external `SignalCache`
/// so signals are not recomputed across calls within a scan batch.
pub fn match_folder_phased_cached<'a>(
    candidate: &ModCandidate,
    db: &MasterDb,
    content: &FolderContent,
    ini_config: &IniTokenizationConfig,
    ai_config: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig<'a>,
    cache: &mut state::signal_cache::SignalCache,
) -> StagedMatchResult {
    let quick_result = pipeline::quick_pipeline::match_folder_quick_cached(
        candidate, db, content, ini_config, ai_config, cache,
    );

    // Only fall back to full scoring if quick didn't auto-match
    if quick_result.status == MatchStatus::AutoMatched {
        return quick_result;
    }

    // Fall back to full scoring for NeedsReview and NoMatch
    pipeline::full_pipeline::match_folder_full_cached(
        candidate,
        db,
        content,
        ini_config,
        ai_config,
        &crate::services::scanner::deep_matcher::analysis::gamebanana::GameBananaConfig::default(),
        cache,
    )
}
