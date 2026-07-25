//! Forced scoring of explicitly requested candidate IDs (no early exits).

use std::collections::{HashMap, HashSet};

use crate::services::scanner::core::walker::{FolderContent, ModCandidate};
use crate::services::scanner::deep_matcher::analysis::ai_rerank::maybe_apply_ai_rerank;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::models::acceptance::{finalize_review, FinalizeConfig};
use crate::services::scanner::deep_matcher::pipeline::name_rescue;
use crate::services::scanner::deep_matcher::pipeline::stages::ObservedTokenBuckets;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{MatchMode, ScoreState, StagedMatchResult};

use super::scoring_stages::{
    apply_alias_recheck_stage, apply_direct_name_support_stage, apply_hash_stage,
    apply_weighted_token_overlap_stage,
};

/// Evaluates specific candidate IDs against all scoring stages without early exits.
/// Used for scoring requested specific items (e.g., for dropdown lazy loading).
pub fn score_forced_candidates(
    candidate: &ModCandidate,
    db: &MasterDb,
    content: &FolderContent,
    ini_config: &IniTokenizationConfig,
    ai_config: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig<'_>,
    cache: &mut crate::services::scanner::deep_matcher::state::signal_cache::SignalCache,
    forced_entry_ids: &[usize],
) -> StagedMatchResult {
    let signals = cache
        .get_or_compute(&candidate.path, content, MatchMode::FullScoring, ini_config)
        .clone();

    let observed_buckets = ObservedTokenBuckets::from_signals(&signals);
    let observed_tokens: HashSet<String> = observed_buckets.observed_tokens().into_iter().collect();

    let mut states: HashMap<usize, ScoreState> = forced_entry_ids
        .iter()
        .copied()
        .map(|entry_id| (entry_id, ScoreState::new()))
        .collect();

    if states.is_empty() {
        return StagedMatchResult::no_match();
    }

    apply_hash_stage(db, &signals.ini_hashes, &mut states);
    crate::services::scanner::deep_matcher::pipeline::stages::apply_alias_stage(
        db,
        &observed_buckets.folder_tokens,
        &mut states,
    );
    name_rescue::apply_substring_name_pass_a(db, &signals, &mut states);
    crate::services::scanner::deep_matcher::pipeline::stages::apply_deep_stage(
        db,
        &observed_buckets,
        &mut states,
    );
    name_rescue::apply_substring_name_pass_b(db, &signals, &mut states);
    apply_alias_recheck_stage(db, &observed_tokens, &mut states);
    apply_weighted_token_overlap_stage(db, &observed_buckets.folder_tokens, &mut states);
    apply_direct_name_support_stage(db, &observed_buckets.folder_tokens, &mut states);

    let mut result = finalize_review(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &FinalizeConfig {
            mode: MatchMode::FullScoring,
            review_min_score: 0.0, // Force everything into the result list
            top_k: forced_entry_ids.len(),
        },
    );

    result = maybe_apply_ai_rerank(result, &signals, db, MatchMode::FullScoring, ai_config);

    result
}
