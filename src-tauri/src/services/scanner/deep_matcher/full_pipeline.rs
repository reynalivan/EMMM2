use super::acceptance::{finalize_review, try_stage_accept, FinalizeConfig, StageAcceptConfig};
use super::ai_rerank::{maybe_apply_ai_rerank, AiRerankConfig};
use super::content::{collect_deep_signals, IniTokenizationConfig};
use super::scoring::{
    apply_alias_contribution, apply_deep_token_contribution,
    apply_direct_name_support_contribution, apply_hash_contribution, apply_ini_token_contribution,
    apply_token_overlap_contribution,
};
use super::stages::{
    replenish_candidates_if_needed, seed_candidates, ObservedTokenBuckets, DEFAULT_MIN_POOL,
    DEFAULT_SEED_CAP,
};
use super::types::{Confidence, MatchMode, Reason, ScoreState, StagedMatchResult};
use super::MasterDb;
use crate::services::scanner::normalizer;
use crate::services::scanner::walker::{FolderContent, ModCandidate};
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
) -> StagedMatchResult {
    let signals =
        collect_deep_signals(&candidate.path, content, MatchMode::FullScoring, ini_config);
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
    apply_alias_stage(db, &observed_buckets.folder_tokens, &mut states);
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
    apply_deep_stage(db, &observed_buckets, &mut states);
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

    maybe_apply_ai_rerank(
        result,
        &signals,
        MatchMode::FullScoring,
        &AiRerankConfig::default(),
    )
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

fn apply_alias_stage(
    db: &MasterDb,
    observed_tokens: &BTreeSet<String>,
    states: &mut HashMap<usize, ScoreState>,
) {
    for (entry_id, state) in states.iter_mut() {
        let entry = &db.entries[*entry_id];
        for skin in &entry.custom_skins {
            if let Some(alias) = skin.aliases.iter().find(|alias| {
                let alias_tokens = normalizer::preprocess_text(alias);
                !alias_tokens.is_empty()
                    && alias_tokens
                        .iter()
                        .all(|token| observed_tokens.contains(token))
            }) {
                apply_alias_contribution(state, alias, 12.0);
                break;
            }
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
                apply_alias_contribution(state, alias, 12.0);
                break;
            }
        }
    }
}

fn apply_deep_stage(
    db: &MasterDb,
    buckets: &ObservedTokenBuckets,
    states: &mut HashMap<usize, ScoreState>,
) {
    for (entry_id, state) in states.iter_mut() {
        let entry_tokens = entry_tokens(db, *entry_id);

        let deep_hits: Vec<String> = buckets
            .deep_name_tokens
            .iter()
            .filter(|token| entry_tokens.contains(*token))
            .cloned()
            .collect();
        let deep_ratio = (deep_hits.len() as f32) / (buckets.deep_name_tokens.len().max(1) as f32);
        apply_deep_token_contribution(state, &deep_hits, deep_ratio, 16.0, 1.0, 6.0);

        let section_hits: Vec<String> = buckets
            .ini_section_tokens
            .iter()
            .filter(|token| entry_tokens.contains(*token))
            .cloned()
            .collect();
        let content_hits: Vec<String> = buckets
            .ini_content_tokens
            .iter()
            .filter(|token| entry_tokens.contains(*token))
            .cloned()
            .collect();
        let ini_denominator =
            (buckets.ini_section_tokens.len() + buckets.ini_content_tokens.len()).max(1);
        let ini_ratio =
            ((section_hits.len() + content_hits.len()) as f32) / (ini_denominator as f32);
        apply_ini_token_contribution(state, &section_hits, &content_hits, ini_ratio, 8.0);
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
        let entry_tokens = entry_tokens(db, *entry_id);
        let overlap_weight: f32 = folder_tokens
            .iter()
            .filter(|token| entry_tokens.contains(*token))
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

fn entry_tokens<'a>(db: &'a MasterDb, entry_id: usize) -> &'a HashSet<String> {
    &db.keywords[entry_id].1
}

#[cfg(test)]
#[path = "full_pipeline_tests.rs"]
mod full_pipeline_tests;
