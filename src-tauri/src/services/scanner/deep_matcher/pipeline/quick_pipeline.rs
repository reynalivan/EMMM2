use std::collections::{BTreeSet, HashMap, HashSet};

use crate::services::scanner::core::walker::{FolderContent, ModCandidate};

use crate::services::scanner::deep_matcher::analysis::ai_rerank::maybe_apply_ai_rerank;
use crate::services::scanner::deep_matcher::analysis::content::PreparedTokenFilters;
use crate::services::scanner::deep_matcher::analysis::scoring::{
    apply_hash_contribution, apply_token_overlap_contribution,
};
use crate::services::scanner::deep_matcher::models::acceptance::StageContext;
use crate::services::scanner::deep_matcher::pipeline::name_rescue;
use crate::services::scanner::deep_matcher::pipeline::stages::{
    apply_direct_name_support_stage, entry_tokens, replenish_candidates_if_needed, seed_candidates,
    ObservedTokenBuckets, DEFAULT_MIN_POOL, DEFAULT_SEED_CAP,
};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{
    Confidence, MatchMode, ScoreState, StagedMatchResult,
};

const QUICK_TOP_K: usize = 5;
const REVIEW_MIN_SCORE_QUICK: f32 = 10.0;

const T_HASH_QUICK: f32 = 10.0;
const M_HASH_QUICK: f32 = 6.0;
const T_ALIAS_QUICK: f32 = 12.0;
const M_ALIAS_QUICK: f32 = 6.0;
const T_DEEP_QUICK: f32 = 14.0;
const M_DEEP_QUICK: f32 = 4.0;
const T_TOKEN_QUICK: f32 = 12.0;
const M_TOKEN_QUICK: f32 = 4.0;

pub fn match_folder_quick(
    candidate: &ModCandidate,
    db: &MasterDb,
    content: &FolderContent,
    ini_filters: &PreparedTokenFilters,
    ai_config: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig<'_>,
) -> StagedMatchResult {
    let mut local_cache =
        crate::services::scanner::deep_matcher::state::signal_cache::SignalCache::new();
    match_folder_quick_cached(
        candidate,
        db,
        content,
        ini_filters,
        ai_config,
        &mut local_cache,
    )
}

pub fn match_folder_quick_cached(
    candidate: &ModCandidate,
    db: &MasterDb,
    content: &FolderContent,
    ini_filters: &PreparedTokenFilters,
    ai_config: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig<'_>,
    cache: &mut crate::services::scanner::deep_matcher::state::signal_cache::SignalCache,
) -> StagedMatchResult {
    let signals = cache
        .get_or_compute(&candidate.path, content, MatchMode::Quick, ini_filters)
        .clone();
    let observed_buckets = ObservedTokenBuckets::from_signals(&signals);
    let observed_tokens: HashSet<String> = observed_buckets.observed_tokens().into_iter().collect();

    let seeded = seed_candidates(
        &db.indexes,
        &signals.ini_hashes,
        &observed_tokens,
        DEFAULT_SEED_CAP,
    );
    let candidate_pool = replenish_candidates_if_needed(
        &db.indexes,
        &seeded,
        &observed_buckets,
        DEFAULT_MIN_POOL,
        DEFAULT_SEED_CAP,
    );

    let mut states: HashMap<usize, ScoreState> = candidate_pool
        .iter()
        .copied()
        .map(|entry_id| (entry_id, ScoreState::new()))
        .collect();

    let stage = StageContext {
        db,
        signals: &signals,
        buckets: &observed_buckets,
        mode: MatchMode::Quick,
        review_min_score: REVIEW_MIN_SCORE_QUICK,
        top_k: QUICK_TOP_K,
    };

    apply_hash_stage(db, &signals.ini_hashes, &mut states);
    if let Some(accepted) = stage.accept(&states, T_HASH_QUICK, M_HASH_QUICK, Confidence::High) {
        return accepted;
    }

    crate::services::scanner::deep_matcher::pipeline::stages::apply_alias_stage(
        db,
        &observed_buckets.folder_tokens,
        &mut states,
    );
    if let Some(accepted) = stage.accept(&states, T_ALIAS_QUICK, M_ALIAS_QUICK, Confidence::High) {
        return accepted;
    }

    // ★ F3A: SubstringName Pass A — file stems + subfolder names
    name_rescue::apply_substring_name_pass_a(db, &signals, &mut states);
    if let Some(accepted) = stage.accept(&states, T_ALIAS_QUICK, M_ALIAS_QUICK, Confidence::High) {
        return accepted;
    }

    crate::services::scanner::deep_matcher::pipeline::stages::apply_deep_stage(
        db,
        &observed_buckets,
        &mut states,
    );
    if let Some(accepted) = stage.accept(&states, T_DEEP_QUICK, M_DEEP_QUICK, Confidence::Medium) {
        return accepted;
    }

    // ★ F3B: SubstringName Pass B — INI-derived strings
    name_rescue::apply_substring_name_pass_b(db, &signals, &mut states);
    if let Some(accepted) = stage.accept(&states, T_DEEP_QUICK, M_DEEP_QUICK, Confidence::High) {
        return accepted;
    }

    apply_token_overlap_stage(db, &observed_buckets.folder_tokens, &mut states);
    if let Some(accepted) = stage.accept(&states, T_TOKEN_QUICK, M_TOKEN_QUICK, Confidence::Medium)
    {
        return accepted;
    }

    apply_direct_name_support_stage(
        db,
        &observed_buckets.folder_tokens,
        &mut states,
        4.0,
        2.0,
        10.0,
        6.0,
    );
    let result = stage.finalize(&states);

    maybe_apply_ai_rerank(result, &signals, db, MatchMode::Quick, ai_config)
}

fn apply_hash_stage(
    db: &MasterDb,
    observed_hashes: &[String],
    states: &mut HashMap<usize, ScoreState>,
) {
    for hash in observed_hashes {
        let Some(posting) = db.indexes.hash_index.get(hash) else {
            continue;
        };
        let df = db
            .indexes
            .hash_df
            .get(hash)
            .copied()
            .unwrap_or(posting.len());
        // `df` is `posting.len()` by construction and a posting list is never
        // empty, so `df == 1` means "this hash belongs to exactly one entry".
        // The full pipeline spells the same test the same way.
        let score_delta = if df == 1 { 12.0 } else { 3.0 };
        let unique_overlap = if df == 1 { 1 } else { 0 };

        for entry_id in posting {
            let Some(state) = states.get_mut(entry_id) else {
                continue;
            };
            apply_hash_contribution(state, 1, unique_overlap, score_delta);
        }
    }
}

fn apply_token_overlap_stage(
    db: &MasterDb,
    folder_tokens: &BTreeSet<String>,
    states: &mut HashMap<usize, ScoreState>,
) {
    for (entry_id, state) in states.iter_mut() {
        let et = entry_tokens(db, *entry_id);
        let overlap = folder_tokens
            .iter()
            .filter(|token| et.contains(*token))
            .count();
        let ratio = (overlap as f32) / (folder_tokens.len().max(1) as f32);
        apply_token_overlap_contribution(state, ratio, 12.0);
    }
}

#[cfg(test)]
#[path = "../tests/pipeline/quick_pipeline_tests.rs"]
mod quick_pipeline_tests;
