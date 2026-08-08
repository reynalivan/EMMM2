//! Staged full-scoring match for a single mod folder.

use crate::services::scanner::core::walker::{FolderContent, ModCandidate};
use crate::services::scanner::deep_matcher::analysis::ai_rerank::maybe_apply_ai_rerank;
use crate::services::scanner::deep_matcher::analysis::content::PreparedTokenFilters;
use crate::services::scanner::deep_matcher::analysis::gamebanana::{self, GameBananaConfig};
use crate::services::scanner::deep_matcher::analysis::mechanical_rerank::{
    self, MechanicalRerankConfig,
};
use crate::services::scanner::deep_matcher::models::acceptance::StageContext;
use crate::services::scanner::deep_matcher::pipeline::name_rescue;
use crate::services::scanner::deep_matcher::pipeline::stages::{
    apply_direct_name_support_stage, replenish_candidates_if_needed, seed_candidates,
    ObservedTokenBuckets, DEFAULT_MIN_POOL, DEFAULT_SEED_CAP,
};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{
    Confidence, MatchMode, MatchStatus, ScoreState, StagedMatchResult,
};
use std::collections::{HashMap, HashSet};

use super::scoring_stages::{
    apply_alias_recheck_stage, apply_hash_stage, apply_weighted_token_overlap_stage,
};

const FULL_TOP_K: usize = 5;
const REVIEW_MIN_SCORE_FULL: f32 = 12.0;
const T_HASH_FULL: f32 = 10.0;
const M_HASH_FULL: f32 = 4.0;
const T_ALIAS_FULL: f32 = 12.0;
const M_ALIAS_FULL: f32 = 4.0;
const T_DEEP_FULL: f32 = 16.0;
const M_DEEP_FULL: f32 = 3.0;
const T_TOKEN_FULL: f32 = 14.0;
const M_TOKEN_FULL: f32 = 3.0;

pub fn match_folder_full(
    candidate: &ModCandidate,
    db: &MasterDb,
    content: &FolderContent,
    ini_filters: &PreparedTokenFilters,
    ai_config: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig<'_>,
    gb_config: &GameBananaConfig,
) -> StagedMatchResult {
    let mut local_cache =
        crate::services::scanner::deep_matcher::state::signal_cache::SignalCache::new();
    match_folder_full_cached(
        candidate,
        db,
        content,
        ini_filters,
        ai_config,
        gb_config,
        &mut local_cache,
    )
}

pub fn match_folder_full_cached(
    candidate: &ModCandidate,
    db: &MasterDb,
    content: &FolderContent,
    ini_filters: &PreparedTokenFilters,
    ai_config: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig<'_>,
    gb_config: &GameBananaConfig,
    cache: &mut crate::services::scanner::deep_matcher::state::signal_cache::SignalCache,
) -> StagedMatchResult {
    let signals = cache
        .get_or_compute(
            &candidate.path,
            content,
            MatchMode::FullScoring,
            ini_filters,
        )
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
        mode: MatchMode::FullScoring,
        review_min_score: REVIEW_MIN_SCORE_FULL,
        top_k: FULL_TOP_K,
    };

    apply_hash_stage(db, &signals.ini_hashes, &mut states);
    if let Some(accepted) = stage.accept(&states, T_HASH_FULL, M_HASH_FULL, Confidence::High) {
        return accepted;
    }

    crate::services::scanner::deep_matcher::pipeline::stages::apply_alias_stage(
        db,
        &observed_buckets.folder_tokens,
        &mut states,
    );
    if let Some(accepted) = stage.accept(&states, T_ALIAS_FULL, M_ALIAS_FULL, Confidence::High) {
        return accepted;
    }

    // ★ F3A: SubstringNameDeep Pass A — check file/subfolder names via substring matching
    name_rescue::apply_substring_name_pass_a(db, &signals, &mut states);
    if let Some(accepted) = stage.accept(&states, T_DEEP_FULL, M_DEEP_FULL, Confidence::High) {
        return accepted;
    }

    // F4: Deep token overlap
    crate::services::scanner::deep_matcher::pipeline::stages::apply_deep_stage(
        db,
        &observed_buckets,
        &mut states,
    );
    if let Some(accepted) = stage.accept(&states, T_DEEP_FULL, M_DEEP_FULL, Confidence::Medium) {
        return accepted;
    }

    // ★ F3B: SubstringNameDeep Pass B — INI-derived strings (section headers + path stems)
    name_rescue::apply_substring_name_pass_b(db, &signals, &mut states);
    if let Some(accepted) = stage.accept(&states, T_DEEP_FULL, M_DEEP_FULL, Confidence::High) {
        return accepted;
    }

    apply_alias_recheck_stage(db, &observed_tokens, &mut states);
    if let Some(accepted) = stage.accept(&states, T_ALIAS_FULL, M_ALIAS_FULL, Confidence::High) {
        return accepted;
    }

    apply_weighted_token_overlap_stage(db, &observed_buckets.folder_tokens, &mut states);
    if let Some(accepted) = stage.accept(&states, T_TOKEN_FULL, M_TOKEN_FULL, Confidence::Medium) {
        return accepted;
    }

    apply_direct_name_support_stage(
        db,
        &observed_buckets.folder_tokens,
        &mut states,
        2.0,
        1.0,
        6.0,
        4.0,
    );
    let result = stage.finalize(&states);

    let result = maybe_apply_ai_rerank(result, &signals, db, MatchMode::FullScoring, ai_config);

    // ★ GameBanana enrichment + mechanical rerank (independent of trait-based AI)
    let result = if result.status == MatchStatus::NeedsReview {
        let gb_result = if gb_config.enabled {
            let refs = gamebanana::detect_gamebanana_ids(&signals);
            if !refs.is_empty() {
                gamebanana::fetch_gamebanana_metadata(&refs, gb_config)
            } else {
                gamebanana::GameBananaResult::default()
            }
        } else {
            gamebanana::GameBananaResult::default()
        };
        let mech_config = MechanicalRerankConfig {
            gb_file_stems: gb_result.file_stems,
            gb_mod_name: gb_result.mod_name,
            gb_root_category: gb_result.root_category,
            gb_description_keywords: gb_result.description_keywords,
            ..MechanicalRerankConfig::default()
        };
        mechanical_rerank::mechanical_rerank(result, &signals, db, &mech_config)
    } else {
        result
    };

    // ★ F9: Root folder rescue — last resort when everything else fails
    if result.status == MatchStatus::NoMatch {
        return name_rescue::apply_root_folder_rescue(db, &signals);
    }

    result
}
