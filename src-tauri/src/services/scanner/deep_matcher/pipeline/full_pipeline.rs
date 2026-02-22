use crate::services::scanner::core::normalizer;
use crate::services::scanner::core::walker::{FolderContent, ModCandidate};
use crate::services::scanner::deep_matcher::analysis::ai_rerank::maybe_apply_ai_rerank;
use crate::services::scanner::deep_matcher::analysis::content::IniTokenizationConfig;
use crate::services::scanner::deep_matcher::analysis::gamebanana::{self, GameBananaConfig};
use crate::services::scanner::deep_matcher::analysis::mechanical_rerank::{
    self, MechanicalRerankConfig,
};
use crate::services::scanner::deep_matcher::analysis::scoring::{
    apply_direct_name_support_contribution, apply_hash_contribution,
    apply_token_overlap_contribution,
};
use crate::services::scanner::deep_matcher::models::acceptance::{
    finalize_review, try_stage_accept, FinalizeConfig, StageAcceptConfig,
};
use crate::services::scanner::deep_matcher::pipeline::name_rescue;
use crate::services::scanner::deep_matcher::pipeline::stages::{
    entry_tokens, replenish_candidates_if_needed, seed_candidates, ObservedTokenBuckets,
    DEFAULT_MIN_POOL, DEFAULT_SEED_CAP,
};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{
    Confidence, MatchMode, MatchStatus, Reason, ScoreState, StagedMatchResult,
};
use std::collections::{BTreeSet, HashMap, HashSet};
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
    ini_config: &IniTokenizationConfig,
    ai_config: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig<'_>,
    gb_config: &GameBananaConfig,
) -> StagedMatchResult {
    let mut local_cache =
        crate::services::scanner::deep_matcher::state::signal_cache::SignalCache::new();
    match_folder_full_cached(
        candidate,
        db,
        content,
        ini_config,
        ai_config,
        gb_config,
        &mut local_cache,
    )
}

pub fn match_folder_full_cached(
    candidate: &ModCandidate,
    db: &MasterDb,
    content: &FolderContent,
    ini_config: &IniTokenizationConfig,
    ai_config: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankConfig<'_>,
    gb_config: &GameBananaConfig,
    cache: &mut crate::services::scanner::deep_matcher::state::signal_cache::SignalCache,
) -> StagedMatchResult {
    let signals = cache
        .get_or_compute(&candidate.path, content, MatchMode::FullScoring, ini_config)
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
    apply_hash_stage(db, &signals.ini_hashes, &mut states);
    if let Some(accepted) = try_stage_accept(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &StageAcceptConfig {
            mode: MatchMode::FullScoring,
            threshold: T_HASH_FULL,
            margin: M_HASH_FULL,
            review_min_score: REVIEW_MIN_SCORE_FULL,
            top_k: FULL_TOP_K,
            best_confidence: Confidence::High,
        },
    ) {
        return accepted;
    }
    crate::services::scanner::deep_matcher::pipeline::stages::apply_alias_stage(
        db,
        &observed_buckets.folder_tokens,
        &mut states,
    );
    if let Some(accepted) = try_stage_accept(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &StageAcceptConfig {
            mode: MatchMode::FullScoring,
            threshold: T_ALIAS_FULL,
            margin: M_ALIAS_FULL,
            review_min_score: REVIEW_MIN_SCORE_FULL,
            top_k: FULL_TOP_K,
            best_confidence: Confidence::High,
        },
    ) {
        return accepted;
    }
    // ★ F3A: SubstringNameDeep Pass A — check file/subfolder names via substring matching
    name_rescue::apply_substring_name_pass_a(db, &signals, &mut states);
    if let Some(accepted) = try_stage_accept(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &StageAcceptConfig {
            mode: MatchMode::FullScoring,
            threshold: T_DEEP_FULL,
            margin: M_DEEP_FULL,
            review_min_score: REVIEW_MIN_SCORE_FULL,
            top_k: FULL_TOP_K,
            best_confidence: Confidence::High,
        },
    ) {
        return accepted;
    }
    // F4: Deep token overlap
    crate::services::scanner::deep_matcher::pipeline::stages::apply_deep_stage(
        db,
        &observed_buckets,
        &mut states,
    );
    if let Some(accepted) = try_stage_accept(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &StageAcceptConfig {
            mode: MatchMode::FullScoring,
            threshold: T_DEEP_FULL,
            margin: M_DEEP_FULL,
            review_min_score: REVIEW_MIN_SCORE_FULL,
            top_k: FULL_TOP_K,
            best_confidence: Confidence::Medium,
        },
    ) {
        return accepted;
    }
    // ★ F3B: SubstringNameDeep Pass B — INI-derived strings (section headers + path stems)
    name_rescue::apply_substring_name_pass_b(db, &signals, &mut states);
    if let Some(accepted) = try_stage_accept(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &StageAcceptConfig {
            mode: MatchMode::FullScoring,
            threshold: T_DEEP_FULL,
            margin: M_DEEP_FULL,
            review_min_score: REVIEW_MIN_SCORE_FULL,
            top_k: FULL_TOP_K,
            best_confidence: Confidence::High,
        },
    ) {
        return accepted;
    }
    apply_alias_recheck_stage(db, &observed_tokens, &mut states);
    if let Some(accepted) = try_stage_accept(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &StageAcceptConfig {
            mode: MatchMode::FullScoring,
            threshold: T_ALIAS_FULL,
            margin: M_ALIAS_FULL,
            review_min_score: REVIEW_MIN_SCORE_FULL,
            top_k: FULL_TOP_K,
            best_confidence: Confidence::High,
        },
    ) {
        return accepted;
    }
    apply_weighted_token_overlap_stage(db, &observed_buckets.folder_tokens, &mut states);
    if let Some(accepted) = try_stage_accept(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &StageAcceptConfig {
            mode: MatchMode::FullScoring,
            threshold: T_TOKEN_FULL,
            margin: M_TOKEN_FULL,
            review_min_score: REVIEW_MIN_SCORE_FULL,
            top_k: FULL_TOP_K,
            best_confidence: Confidence::Medium,
        },
    ) {
        return accepted;
    }
    apply_direct_name_support_stage(db, &observed_buckets.folder_tokens, &mut states);
    let result = finalize_review(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &FinalizeConfig {
            mode: MatchMode::FullScoring,
            review_min_score: REVIEW_MIN_SCORE_FULL,
            top_k: FULL_TOP_K,
        },
    );

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
        let hash_weight = 1.0_f32 / ((df as f32) + 1.8).ln();
        let score_delta = (3.0 * hash_weight) + if df == 1 { 9.0 } else { 0.0 };
        let unique_overlap = if df == 1 { 1 } else { 0 };
        for entry_id in posting {
            let Some(state) = states.get_mut(entry_id) else {
                continue;
            };
            apply_hash_contribution(state, 1, unique_overlap, score_delta);
        }
    }
}

fn apply_alias_recheck_stage(
    db: &MasterDb,
    observed_tokens: &HashSet<String>,
    states: &mut HashMap<usize, ScoreState>,
) {
    for (entry_id, state) in states.iter_mut() {
        if state
            .reasons
            .iter()
            .any(|reason| matches!(reason, Reason::AliasStrict { .. }))
        {
            continue;
        }
        let entry = &db.entries[*entry_id];
        for skin in &entry.custom_skins {
            if let Some(alias) = skin.aliases.iter().find(|alias| {
                let alias_tokens = normalizer::preprocess_text(alias);
                !alias_tokens.is_empty()
                    && alias_tokens
                        .iter()
                        .all(|token| observed_tokens.contains(token))
            }) {
                crate::services::scanner::deep_matcher::analysis::scoring::apply_alias_contribution(
                    state, alias, 12.0,
                );
                break;
            }
        }
    }
}

fn apply_weighted_token_overlap_stage(
    db: &MasterDb,
    folder_tokens: &BTreeSet<String>,
    states: &mut HashMap<usize, ScoreState>,
) {
    let total_folder_weight: f32 = folder_tokens
        .iter()
        .map(|token| db.token_idf(token))
        .sum::<f32>()
        .max(f32::EPSILON);

    for (entry_id, state) in states.iter_mut() {
        let et = entry_tokens(db, *entry_id);
        let overlap_weight: f32 = folder_tokens
            .iter()
            .filter(|token| et.contains(*token))
            .map(|token| db.token_idf(token))
            .sum();

        let ratio = overlap_weight / total_folder_weight;
        apply_token_overlap_contribution(state, ratio, 12.0);
    }
}

fn apply_direct_name_support_stage(
    db: &MasterDb,
    folder_tokens: &BTreeSet<String>,
    states: &mut HashMap<usize, ScoreState>,
) {
    for (entry_id, state) in states.iter_mut() {
        let entry = &db.entries[*entry_id];
        let name_hits: Vec<String> = normalizer::preprocess_text(&entry.name)
            .into_iter()
            .filter(|token| folder_tokens.contains(token))
            .collect();
        let tag_hits: Vec<String> = entry
            .tags
            .iter()
            .flat_map(|tag| normalizer::preprocess_text(tag).into_iter())
            .filter(|token| folder_tokens.contains(token))
            .collect();
        apply_direct_name_support_contribution(state, &name_hits, &tag_hits, 2.0, 1.0, 6.0, 4.0);
    }
}

#[cfg(test)]
#[path = "../tests/pipeline/full_pipeline_tests.rs"]
mod full_pipeline_tests;
