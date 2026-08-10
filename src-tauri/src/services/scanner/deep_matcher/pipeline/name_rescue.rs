//! Name Rescue stages for the FullScoring pipeline.
//!
//! F3: `apply_substring_name_stage` — early stage checking file stems + subfolder names
//!     via substring matching against DB entry names/tags/aliases.
//! F9: `apply_root_folder_rescue` — last resort checking root folder name only.

use std::collections::HashMap;

use crate::common::normalizer;
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
const SCORE_FUZZY_RESCUE: f32 = 6.0;
const MIN_FUZZY_LEN: usize = 5;
const FUZZY_MIN_SIMILARITY: f64 = 0.80;

const RESCUE_TOP_K: usize = 5;

/// Normalize and space-condense a set of terms, dropping ones too short to be
/// a meaningful substring. Hoisted out of the matching loops — these forms
/// depend only on the DB entry, never on the folder being scored.
fn condensed_terms<'a>(terms: impl Iterator<Item = &'a String>) -> Vec<String> {
    terms
        .map(|term| normalizer::normalize_for_matching_default(term).replace(' ', ""))
        .filter(|term| term.len() >= MIN_TERM_LEN)
        .collect()
}

/// Substring match in either direction, which is what every condensed check here wants.
fn overlaps(term: &str, other: &str) -> bool {
    !term.is_empty() && (other.contains(term) || term.contains(other))
}

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

    // Condensed forms are invariant across the entry loop, so build them once
    // instead of once per (entry × source string) pair.
    let source_condensed: Vec<String> = source_strings
        .iter()
        .map(|deep_str| deep_str.replace(' ', ""))
        .collect();

    for (entry_id, state) in states.iter_mut() {
        let entry = &db.entries[*entry_id];
        let entry_name_norm = normalizer::normalize_for_matching_default(&entry.name);
        // Invariant across the source-string loop below.
        let entry_condensed = entry_name_norm.replace(' ', "");
        let alias_condensed = condensed_terms(entry.custom_skins.iter().flat_map(|s| &s.aliases));
        let tag_condensed = condensed_terms(entry.tags.iter());

        for (deep_str, deep_condensed) in source_strings.iter().zip(&source_condensed) {
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
            if entry_condensed.len() >= MIN_TERM_LEN && overlaps(&entry_condensed, deep_condensed) {
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
            let alias_matched = alias_condensed
                .iter()
                .any(|alias| overlaps(alias, deep_condensed));
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
            let tag_matched = tag_condensed
                .iter()
                .any(|tag| overlaps(tag, deep_condensed));
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
        if !matched
            && entry.tags.iter().any(|tag| {
                let tag_norm = normalizer::normalize_for_matching_default(tag);
                !tag_norm.is_empty()
                    && tag_norm.len() >= MIN_TERM_LEN
                    && (folder_norm.contains(&tag_norm) || tag_norm.contains(folder_norm.as_str()))
            })
        {
            matched = true;
            score += SCORE_FOLDER_RESCUE;
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
            continue;
        }

        let similarity = best_windowed_similarity(&folder_condensed, &entry_condensed);
        if similarity >= FUZZY_MIN_SIMILARITY {
            candidates.push(Candidate {
                entry_id,
                name: entry.name.clone(),
                object_type: entry.object_type.clone(),
                score: SCORE_FUZZY_RESCUE,
                confidence: Confidence::Low,
                reasons: vec![Reason::FuzzyName {
                    matched_term: folder_norm.clone(),
                    similarity: similarity as f32,
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

fn best_windowed_similarity(source: &str, entry: &str) -> f64 {
    if source.len() < MIN_FUZZY_LEN || entry.len() < MIN_FUZZY_LEN {
        return 0.0;
    }
    if source.len() <= entry.len() {
        return strsim::normalized_levenshtein(source, entry);
    }

    source
        .as_bytes()
        .windows(entry.len())
        .filter_map(|window| std::str::from_utf8(window).ok())
        .map(|window| strsim::normalized_levenshtein(window, entry))
        .fold(0.0, f64::max)
}

#[cfg(test)]
#[path = "../tests/pipeline/name_rescue_tests.rs"]
mod name_rescue_tests;
