use std::collections::{BTreeSet, HashMap, HashSet};

use crate::services::scanner::normalizer;
use crate::services::scanner::walker::{FolderContent, ModCandidate};

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
use super::types::{Confidence, MatchMode, ScoreState, StagedMatchResult};
use super::MasterDb;

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
    ini_config: &IniTokenizationConfig,
) -> StagedMatchResult {
    let signals = collect_deep_signals(&candidate.path, content, MatchMode::Quick, ini_config);
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
            mode: MatchMode::Quick,
            threshold: T_HASH_QUICK,
            margin: M_HASH_QUICK,
            review_min_score: REVIEW_MIN_SCORE_QUICK,
            top_k: QUICK_TOP_K,
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
            mode: MatchMode::Quick,
            threshold: T_ALIAS_QUICK,
            margin: M_ALIAS_QUICK,
            review_min_score: REVIEW_MIN_SCORE_QUICK,
            top_k: QUICK_TOP_K,
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
            mode: MatchMode::Quick,
            threshold: T_DEEP_QUICK,
            margin: M_DEEP_QUICK,
            review_min_score: REVIEW_MIN_SCORE_QUICK,
            top_k: QUICK_TOP_K,
            best_confidence: Confidence::Medium,
        },
    ) {
        return accepted;
    }

    apply_token_overlap_stage(db, &observed_buckets.folder_tokens, &mut states);
    if let Some(accepted) = try_stage_accept(
        db,
        &states,
        &signals,
        &observed_buckets,
        None,
        &StageAcceptConfig {
            mode: MatchMode::Quick,
            threshold: T_TOKEN_QUICK,
            margin: M_TOKEN_QUICK,
            review_min_score: REVIEW_MIN_SCORE_QUICK,
            top_k: QUICK_TOP_K,
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
            mode: MatchMode::Quick,
            review_min_score: REVIEW_MIN_SCORE_QUICK,
            top_k: QUICK_TOP_K,
        },
    );

    maybe_apply_ai_rerank(
        result,
        &signals,
        MatchMode::Quick,
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
        let score_delta = if df <= 1 { 12.0 } else { 3.0 };
        let unique_overlap = if df <= 1 { 1 } else { 0 };

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
    folder_tokens: &BTreeSet<String>,
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
                        .all(|token| folder_tokens.contains(token))
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

fn apply_token_overlap_stage(
    db: &MasterDb,
    folder_tokens: &BTreeSet<String>,
    states: &mut HashMap<usize, ScoreState>,
) {
    for (entry_id, state) in states.iter_mut() {
        let entry_tokens = entry_tokens(db, *entry_id);
        let overlap = folder_tokens
            .iter()
            .filter(|token| entry_tokens.contains(*token))
            .count();
        let ratio = (overlap as f32) / (folder_tokens.len().max(1) as f32);
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

        apply_direct_name_support_contribution(state, &name_hits, &tag_hits, 4.0, 2.0, 10.0, 6.0);
    }
}

fn entry_tokens<'a>(db: &'a MasterDb, entry_id: usize) -> &'a HashSet<String> {
    &db.keywords[entry_id].1
}

#[cfg(test)]
#[path = "quick_pipeline_tests.rs"]
mod quick_pipeline_tests;
