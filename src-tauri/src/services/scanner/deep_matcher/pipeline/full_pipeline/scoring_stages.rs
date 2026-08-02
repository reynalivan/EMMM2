//! Scoring stages specific to the full pipeline.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::common::normalizer;
use crate::services::scanner::deep_matcher::analysis::scoring::{
    apply_hash_contribution, apply_token_overlap_contribution,
};
use crate::services::scanner::deep_matcher::pipeline::stages::entry_tokens;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{Reason, ScoreState};

pub(super) fn apply_hash_stage(
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

pub(super) fn apply_alias_recheck_stage(
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

pub(super) fn apply_weighted_token_overlap_stage(
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
