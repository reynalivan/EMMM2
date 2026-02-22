//! Name Rescue stages for the FullScoring pipeline.
//!
//! F3: `apply_substring_name_stage` — early stage checking file stems + subfolder names
//!     via substring matching against DB entry names/tags/aliases.
//! F9: `apply_root_folder_rescue` — last resort checking root folder name only.

use std::collections::HashMap;

use crate::services::scanner::core::normalizer;
use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::analysis::scoring::push_reason_capped;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{
    sort_candidates_deterministic, Candidate, Confidence, MatchStatus, Reason, ScoreState,
    StagedMatchResult,
};

/// Minimum term length for substring matching (avoids false positives like "ai", "hu").
const MIN_TERM_LEN: usize = 3;

/// Score weights for different match types.
const SCORE_EXACT_NAME: f32 = 16.0;
const SCORE_ALIAS_SUBSTRING: f32 = 11.0;
const SCORE_NAME_SUBSTRING: f32 = 10.0;
const SCORE_TAG_SUBSTRING: f32 = 9.0;
const SCORE_FOLDER_RESCUE: f32 = 8.0;

const RESCUE_TOP_K: usize = 5;

// ==================== F3: EARLY STAGE ====================

/// Apply substring matching Pass A — file stems + subfolder names.
pub fn apply_substring_name_pass_a(
    db: &MasterDb,
    signals: &FolderSignals,
    states: &mut HashMap<usize, ScoreState>,
) {
    apply_substring_name_inner(db, &signals.deep_name_strings, "file", states);
}

/// Apply substring matching Pass B — INI-derived strings (section headers + path stems).
pub fn apply_substring_name_pass_b(
    db: &MasterDb,
    signals: &FolderSignals,
    states: &mut HashMap<usize, ScoreState>,
) {
    apply_substring_name_inner(db, &signals.ini_derived_strings, "ini", states);
}

/// Generic substring matching against a set of normalized strings.
fn apply_substring_name_inner(
    db: &MasterDb,
    source_strings: &[String],
    source_label: &str,
    states: &mut HashMap<usize, ScoreState>,
) {
    if source_strings.is_empty() {
        return;
    }

    for (entry_id, state) in states.iter_mut() {
        let entry = &db.entries[*entry_id];
        let entry_name_norm = normalizer::normalize_for_matching_default(&entry.name);

        for deep_str in source_strings {
            // Exact name match (highest score)
            if !entry_name_norm.is_empty()
                && entry_name_norm.len() >= MIN_TERM_LEN
                && *deep_str == entry_name_norm
            {
                state.score = (state.score + SCORE_EXACT_NAME).min(100.0);
                state.max_confidence = std::cmp::max(state.max_confidence, Confidence::Excellent);
                push_reason_capped(
                    state,
                    Reason::SubstringName {
                        matched_term: entry.name.clone(),
                        source: source_label.to_string(),
                    },
                );
                continue;
            }

            // Name substring: entry_name ⊂ deep_str OR deep_str ⊂ entry_name
            // Condense spaces for cross-word-boundary matching
            let deep_condensed = deep_str.replace(" ", "");
            let entry_condensed = entry_name_norm.replace(" ", "");
            if !entry_condensed.is_empty()
                && entry_condensed.len() >= MIN_TERM_LEN
                && (deep_condensed.contains(&entry_condensed)
                    || entry_condensed.contains(&deep_condensed))
            {
                state.score = (state.score + SCORE_NAME_SUBSTRING).min(100.0);
                state.max_confidence = std::cmp::max(state.max_confidence, Confidence::High);
                push_reason_capped(
                    state,
                    Reason::SubstringName {
                        matched_term: entry.name.clone(),
                        source: source_label.to_string(),
                    },
                );
                continue;
            }

            // Alias substring check (condensed)
            let alias_matched = entry.custom_skins.iter().any(|skin| {
                skin.aliases.iter().any(|alias| {
                    let alias_condensed =
                        normalizer::normalize_for_matching_default(alias).replace(" ", "");
                    !alias_condensed.is_empty()
                        && alias_condensed.len() >= MIN_TERM_LEN
                        && (deep_condensed.contains(&alias_condensed)
                            || alias_condensed.contains(&deep_condensed))
                })
            });
            if alias_matched {
                state.score = (state.score + SCORE_ALIAS_SUBSTRING).min(100.0);
                state.max_confidence = std::cmp::max(state.max_confidence, Confidence::High);
                push_reason_capped(
                    state,
                    Reason::SubstringName {
                        matched_term: entry.name.clone(),
                        source: format!("{source_label}_alias"),
                    },
                );
                continue;
            }

            // Tag substring check (condensed)
            let tag_matched = entry.tags.iter().any(|tag| {
                let tag_condensed =
                    normalizer::normalize_for_matching_default(tag).replace(" ", "");
                !tag_condensed.is_empty()
                    && tag_condensed.len() >= MIN_TERM_LEN
                    && (deep_condensed.contains(&tag_condensed)
                        || tag_condensed.contains(&deep_condensed))
            });
            if tag_matched {
                state.score = (state.score + SCORE_TAG_SUBSTRING).min(100.0);
                state.max_confidence = std::cmp::max(state.max_confidence, Confidence::High);
                push_reason_capped(
                    state,
                    Reason::SubstringName {
                        matched_term: entry.name.clone(),
                        source: format!("{source_label}_tag"),
                    },
                );
            }
        }
    }
}

// ==================== F9: LAST RESORT ====================

/// Last-resort match using root folder name ONLY.
///
/// Called when finalize_review returns NoMatch. Checks if the normalized root folder
/// name matches any DB entry via substring. Returns NeedsReview with Medium confidence.
pub fn apply_root_folder_rescue(db: &MasterDb, signals: &FolderSignals) -> StagedMatchResult {
    let folder_norm = &signals.folder_name_normalized;
    if folder_norm.is_empty() || folder_norm.len() < MIN_TERM_LEN {
        return StagedMatchResult::no_match();
    }

    let mut candidates: Vec<Candidate> = Vec::new();

    for (entry_id, entry) in db.entries.iter().enumerate() {
        let entry_name_norm = normalizer::normalize_for_matching_default(&entry.name);
        let mut matched = false;
        let mut score = 0.0_f32;

        // Name substring check
        let entry_condensed = entry_name_norm.replace(" ", "");
        let folder_condensed = folder_norm.replace(" ", "");
        if !entry_condensed.is_empty()
            && entry_condensed.len() >= MIN_TERM_LEN
            && (folder_condensed.contains(&entry_condensed)
                || entry_condensed.contains(&folder_condensed))
        {
            matched = true;
            score += SCORE_FOLDER_RESCUE;
        }

        // Alias substring check
        if !matched {
            for skin in &entry.custom_skins {
                if skin.aliases.iter().any(|alias| {
                    let alias_norm = normalizer::normalize_for_matching_default(alias);
                    !alias_norm.is_empty()
                        && alias_norm.len() >= MIN_TERM_LEN
                        && (folder_norm.contains(&alias_norm)
                            || alias_norm.contains(folder_norm.as_str()))
                }) {
                    matched = true;
                    score += SCORE_FOLDER_RESCUE;
                    break;
                }
            }
        }

        // Tag substring check
        if !matched {
            if entry.tags.iter().any(|tag| {
                let tag_norm = normalizer::normalize_for_matching_default(tag);
                !tag_norm.is_empty()
                    && tag_norm.len() >= MIN_TERM_LEN
                    && (folder_norm.contains(&tag_norm) || tag_norm.contains(folder_norm.as_str()))
            }) {
                matched = true;
                score += SCORE_FOLDER_RESCUE;
            }
        }

        if matched {
            candidates.push(Candidate {
                entry_id,
                name: entry.name.clone(),
                object_type: entry.object_type.clone(),
                score,
                confidence: Confidence::Medium,
                reasons: vec![Reason::FolderNameRescue {
                    matched_term: folder_norm.clone(),
                }],
            });
        }
    }

    if candidates.is_empty() {
        return StagedMatchResult::no_match();
    }

    sort_candidates_deterministic(&mut candidates);

    let candidates_all = candidates.clone();
    candidates.truncate(RESCUE_TOP_K);

    let best = candidates.first().cloned();
    StagedMatchResult {
        status: MatchStatus::NeedsReview,
        best,
        candidates_topk: candidates,
        candidates_all,
        evidence:
            crate::services::scanner::deep_matcher::pipeline::quick_pipeline_result::empty_evidence(
                signals,
            ),
    }
}

#[cfg(test)]
#[path = "../tests/pipeline/name_rescue_tests.rs"]
mod name_rescue_tests;
