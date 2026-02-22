//! Deterministic mechanical AI reranker for the deep matcher pipeline.
//!
//! Runs after the standard pipeline + optional GameBanana enrichment when
//! the result is still `NeedsReview`. Uses a points-based system to score
//! each candidate and decide acceptance.
//!
//! **Independent**: Works with or without GB enrichment data. Works with or
//! without the trait-based AI provider. This is a fallback that runs even
//! if both are disabled.

use std::collections::HashMap;

use crate::services::scanner::core::normalizer;
use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::analysis::scoring::{
    cap_reasons, has_primary_evidence,
};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{
    sort_candidates_deterministic, Candidate, Confidence, MatchStatus, Reason, StagedMatchResult,
};

// ── Config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MechanicalRerankConfig {
    pub enabled: bool,
    pub gb_file_stems: Vec<String>,
    pub gb_mod_name: Option<String>,
    pub gb_root_category: Option<String>,
    pub gb_description_keywords: Vec<String>,
    pub ai_accept_min: f32, // required min AI score (0-1) to auto-accept
    pub ai_accept_gap: f32, // gap required between #1 and #2 to auto-accept
}

impl Default for MechanicalRerankConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            gb_file_stems: Vec::new(),
            gb_mod_name: None,
            gb_root_category: None,
            gb_description_keywords: Vec::new(),
            ai_accept_min: 0.85,
            ai_accept_gap: 0.15,
        }
    }
}

// ── Point Constants ──────────────────────────────────────────────────

const PT_EXACT_NAME: f32 = 20.0;
const PT_EXACT_ALIAS: f32 = 18.0;
const PT_UNIQUE_HASH: f32 = 18.0;
const PT_NAME_SUBSTR_SPACED: f32 = 14.0;
const PT_NAME_SUBSTR_COMPACT: f32 = 10.0;
const PT_ALIAS_SUBSTR: f32 = 15.0;
const PT_ALIAS_COMPACT: f32 = 11.0;
const PT_NAME_WORD: f32 = 8.0;
const PT_NAME_WORD_COMPACT: f32 = 6.0;
const PT_TAG_SUBSTR: f32 = 6.0;
const PT_GB_EXACT_NAME: f32 = 6.0;
const PT_GB_SUBSTR_NAME: f32 = 4.0;
const PT_GB_SUBSTR_ALIAS: f32 = 5.0;
const PT_GB_SUBSTR_TAG: f32 = 2.0;
const GB_CAP: f32 = 8.0;
const PT_GB_EXACT_MOD_NAME: f32 = 25.0; // Huge bonus for exact mod name match
const PT_GB_DESC_KEYWORD: f32 = 2.0;
const GB_DESC_CAP: f32 = 6.0;

const PT_DEEP_RATIO_HIGH: f32 = 10.0;
const PT_DEEP_RATIO_MED: f32 = 7.0;
const PT_DEEP_RATIO_LOW: f32 = 4.0;
const PT_INI_HITS_2: f32 = 6.0;
const PT_INI_HITS_1: f32 = 3.0;
const PENALTY_FOREIGN: f32 = -3.0;
const PENALTY_FOREIGN_CAP: f32 = -12.0;
const PENALTY_MULTI_ENTITY: f32 = -4.0;
const PENALTY_TYPE_MISMATCH: f32 = -2.0;
const PENALTY_RESCUE_ONLY: f32 = -8.0;
const PENALTY_GB_CATEGORY_MISMATCH: f32 = -15.0; // Heavy penalty for wrong object category

const SCORE_DIVISOR: f32 = 30.0;
const MIN_POINT_DELTA: f32 = 1.0;
const MAX_NAME_WORD_HITS: usize = 2;
const MAX_TAG_HITS: usize = 2;

// ── Entry Point ──────────────────────────────────────────────────────

/// Run the mechanical points-based reranker.
///
/// Only processes `NeedsReview` results. Returns the result unchanged if
/// disabled or if the accept gate fails.
pub fn mechanical_rerank(
    result: StagedMatchResult,
    signals: &FolderSignals,
    db: &MasterDb,
    config: &MechanicalRerankConfig,
) -> StagedMatchResult {
    if !config.enabled || result.status != MatchStatus::NeedsReview {
        return result;
    }
    if result.candidates_topk.is_empty() {
        return result;
    }

    // Score each candidate
    let scores: Vec<(usize, f32)> = result
        .candidates_topk
        .iter()
        .map(|c| {
            let pts = compute_points(c, signals, db, config);
            (c.entry_id, pts)
        })
        .collect();

    let ai_scores: HashMap<usize, f32> = scores
        .iter()
        .map(|(id, pts)| (*id, (*pts / SCORE_DIVISOR).clamp(0.0, 1.0)))
        .collect();

    // Find best and second-best
    let mut sorted = scores.clone();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (best_id, best_pts) = sorted[0];
    let second_pts = sorted.get(1).map(|s| s.1).unwrap_or(0.0);
    let best_ai = (best_pts / SCORE_DIVISOR).clamp(0.0, 1.0);
    let second_ai = (second_pts / SCORE_DIVISOR).clamp(0.0, 1.0);

    // Accept gate: all must pass
    if best_ai < config.ai_accept_min {
        return result;
    }
    if (best_ai - second_ai) < config.ai_accept_gap {
        return result;
    }
    if (best_pts - second_pts) < MIN_POINT_DELTA {
        return result;
    }

    // Check primary evidence for winner
    let winner = result
        .candidates_topk
        .iter()
        .find(|c| c.entry_id == best_id);
    if let Some(w) = winner {
        if !has_primary_evidence(&w.reasons) {
            return result;
        }
    } else {
        return result;
    }

    // Promote to AutoMatched
    promote_winner(result, best_id, &ai_scores)
}

// ── Scoring ──────────────────────────────────────────────────────────

fn compute_points(
    candidate: &Candidate,
    signals: &FolderSignals,
    db: &MasterDb,
    config: &MechanicalRerankConfig,
) -> f32 {
    let entry = &db.entries[candidate.entry_id];
    let name_norm = normalizer::normalize_for_matching_default(&entry.name);
    let name_condensed = name_norm.replace(' ', "");

    let mut pts = 0.0_f32;

    // ── Exact hits ───────────────────────────────────────────────
    let all_strings: Vec<&String> = signals
        .deep_name_strings
        .iter()
        .chain(signals.ini_derived_strings.iter())
        .collect();

    let has_exact_name = all_strings.iter().any(|s| {
        let s_norm = normalizer::normalize_for_matching_default(s);
        !s_norm.is_empty() && s_norm == name_norm
    });
    if has_exact_name {
        pts += PT_EXACT_NAME;
    }

    let has_exact_alias = entry.custom_skins.iter().any(|skin| {
        skin.aliases.iter().any(|alias| {
            let a_norm = normalizer::normalize_for_matching_default(alias);
            !a_norm.is_empty()
                && all_strings
                    .iter()
                    .any(|s| normalizer::normalize_for_matching_default(s) == a_norm)
        })
    });
    if has_exact_alias {
        pts += PT_EXACT_ALIAS;
    }

    // Hash match — detected from candidate's existing reasons
    let has_hash = candidate
        .reasons
        .iter()
        .any(|r| matches!(r, Reason::HashOverlap { unique_overlap, .. } if *unique_overlap > 0));
    if has_hash {
        pts += PT_UNIQUE_HASH;
    }

    // ── Substring hits ───────────────────────────────────────────
    if !has_exact_name && !name_norm.is_empty() {
        let has_spaced = all_strings
            .iter()
            .any(|s| s.contains(&name_norm) || name_norm.contains(s.as_str()));
        if has_spaced {
            pts += PT_NAME_SUBSTR_SPACED;
        } else {
            let has_compact = all_strings.iter().any(|s| {
                let sc = s.replace(' ', "");
                sc.contains(&name_condensed) || name_condensed.contains(sc.as_str())
            });
            if has_compact {
                pts += PT_NAME_SUBSTR_COMPACT;
            }
        }
    }

    // Alias substring
    if !has_exact_alias {
        let alias_sub = entry.custom_skins.iter().any(|skin| {
            skin.aliases.iter().any(|alias| {
                let a_norm = normalizer::normalize_for_matching_default(alias);
                if a_norm.is_empty() {
                    return false;
                }
                all_strings
                    .iter()
                    .any(|s| s.contains(&a_norm) || a_norm.contains(s.as_str()))
            })
        });
        if alias_sub {
            pts += PT_ALIAS_SUBSTR;
        } else {
            let alias_compact = entry.custom_skins.iter().any(|skin| {
                skin.aliases.iter().any(|alias| {
                    let a_c = normalizer::normalize_for_matching_default(alias).replace(' ', "");
                    if a_c.is_empty() {
                        return false;
                    }
                    all_strings.iter().any(|s| {
                        let sc = s.replace(' ', "");
                        sc.contains(&a_c) || a_c.contains(sc.as_str())
                    })
                })
            });
            if alias_compact {
                pts += PT_ALIAS_COMPACT;
            }
        }
    }

    // Name word hits (max 2)
    let name_words: std::collections::HashSet<String> = normalizer::preprocess_text(&entry.name);
    let word_hits: usize = name_words
        .iter()
        .filter(|w| w.len() >= 3 && signals.deep_name_tokens.iter().any(|t| t == *w))
        .take(MAX_NAME_WORD_HITS)
        .count();
    if word_hits > 0 {
        pts += PT_NAME_WORD * word_hits as f32;
    } else {
        let compact_word_hits: usize = name_words
            .iter()
            .filter(|w| w.len() >= 3 && signals.ini_content_tokens.iter().any(|t| t == *w))
            .take(MAX_NAME_WORD_HITS)
            .count();
        if compact_word_hits > 0 {
            pts += PT_NAME_WORD_COMPACT * compact_word_hits as f32;
        }
    }

    // Tag substring (max 2)
    let tag_hits: usize = entry
        .tags
        .iter()
        .filter(|tag| {
            let t_c = normalizer::normalize_for_matching_default(tag).replace(' ', "");
            !t_c.is_empty()
                && t_c.len() >= 3
                && all_strings.iter().any(|s| {
                    let sc = s.replace(' ', "");
                    sc.contains(&t_c) || t_c.contains(sc.as_str())
                })
        })
        .take(MAX_TAG_HITS)
        .count();
    pts += PT_TAG_SUBSTR * tag_hits as f32;

    // ── GameBanana evidence ──────────────────────────────────────
    let mut gb_pts = 0.0_f32;
    for stem in &config.gb_file_stems {
        if stem == &name_norm {
            gb_pts += PT_GB_EXACT_NAME;
        } else if stem.contains(&name_norm) || name_norm.contains(stem.as_str()) {
            gb_pts += PT_GB_SUBSTR_NAME;
        }

        // alias check
        let alias_hit = entry.custom_skins.iter().any(|skin| {
            skin.aliases.iter().any(|alias| {
                let a = normalizer::normalize_for_matching_default(alias);
                !a.is_empty() && (stem.contains(&a) || a.contains(stem.as_str()))
            })
        });
        if alias_hit {
            gb_pts += PT_GB_SUBSTR_ALIAS;
        }

        // tag check
        let tag_hit = entry.tags.iter().any(|tag| {
            let t = normalizer::normalize_for_matching_default(tag);
            !t.is_empty() && t.len() >= 3 && (stem.contains(&t) || t.contains(stem.as_str()))
        });
        if tag_hit {
            gb_pts += PT_GB_SUBSTR_TAG;
        }
    }
    pts += gb_pts.min(GB_CAP);

    // ── GameBanana Supplemental Enrichment ───────────────────────

    // 1. Exact Mod Name Overlap (First-Class Signal simulation)
    if let Some(ref gb_name) = config.gb_mod_name {
        let gb_name_norm = normalizer::normalize_for_matching_default(gb_name);
        if !gb_name_norm.is_empty() {
            let mut name_match = gb_name_norm == name_norm;
            if !name_match {
                // Check aliases
                name_match = entry.custom_skins.iter().any(|skin| {
                    skin.aliases.iter().any(|alias| {
                        let a_norm = normalizer::normalize_for_matching_default(alias);
                        !a_norm.is_empty() && a_norm == gb_name_norm
                    })
                });
            }
            if name_match {
                pts += PT_GB_EXACT_MOD_NAME;
            }
        }
    }

    // 2. Description Keyword Overlap
    if !config.gb_description_keywords.is_empty() {
        let mut desc_pts = 0.0_f32;
        let et_tokens = &db.keywords[candidate.entry_id].1; // Using entry tokens which contain name, alias, tags

        for kw in &config.gb_description_keywords {
            if et_tokens.contains(kw) || name_words.contains(kw) {
                desc_pts += PT_GB_DESC_KEYWORD;
            }
        }
        pts += desc_pts.min(GB_DESC_CAP);
    }

    // ── Token evidence ───────────────────────────────────────────
    let et = &db.keywords[candidate.entry_id].1;
    let deep_hits = signals
        .deep_name_tokens
        .iter()
        .filter(|t| et.contains(*t))
        .count();
    let deep_total = signals.deep_name_tokens.len().max(1);
    let deep_ratio = deep_hits as f32 / deep_total as f32;

    if deep_ratio >= 0.20 {
        pts += PT_DEEP_RATIO_HIGH;
    } else if deep_ratio >= 0.12 {
        pts += PT_DEEP_RATIO_MED;
    } else if deep_ratio >= 0.08 {
        pts += PT_DEEP_RATIO_LOW;
    }

    let ini_hits = signals
        .ini_section_tokens
        .iter()
        .chain(signals.ini_content_tokens.iter())
        .filter(|t| et.contains(*t))
        .count();
    if ini_hits >= 2 {
        pts += PT_INI_HITS_2;
    } else if ini_hits >= 1 {
        pts += PT_INI_HITS_1;
    }

    // ── Penalties ────────────────────────────────────────────────
    let foreign_strong = count_foreign_strong_hits(candidate, signals, db);
    pts += (PENALTY_FOREIGN * foreign_strong as f32).max(PENALTY_FOREIGN_CAP);

    if is_multi_entity(candidate) {
        pts += PENALTY_MULTI_ENTITY;
    }

    if has_type_mismatch(candidate) {
        pts += PENALTY_TYPE_MISMATCH;
    }

    if is_rescue_only(candidate) {
        pts += PENALTY_RESCUE_ONLY;
    }

    // 3. Category Mismatch Penalty
    if let Some(ref gb_cat) = config.gb_root_category {
        let cat_lower = gb_cat.to_lowercase();
        let obj_type_lower = entry.object_type.to_lowercase();

        if cat_lower == "skins" {
            // "Skins" should generally only map to Character/Avatar/NPC
            let is_character_type = obj_type_lower.contains("character")
                || obj_type_lower.contains("avatar")
                || obj_type_lower.contains("npc")
                || obj_type_lower.contains("monster");

            if !is_character_type {
                pts += PENALTY_GB_CATEGORY_MISMATCH;
            }
        }
        // Could expand to "Weapons" -> contains("weapon"), etc.
    }

    pts
}

// ── Penalty Helpers ──────────────────────────────────────────────────

fn count_foreign_strong_hits(
    candidate: &Candidate,
    _signals: &FolderSignals,
    _db: &MasterDb,
) -> usize {
    candidate
        .reasons
        .iter()
        .filter(|r| matches!(r, Reason::NegativeEvidence { .. }))
        .count()
}

/// Multi-entity detection: not yet tracked via Reason variants.
/// Placeholder for future `Reason::AmbiguityNote`.
fn is_multi_entity(_candidate: &Candidate) -> bool {
    false
}

/// Type mismatch detection: not yet tracked via Reason variants.
/// Placeholder for future `Reason::ObjectTypeMismatch`.
fn has_type_mismatch(_candidate: &Candidate) -> bool {
    false
}

fn is_rescue_only(candidate: &Candidate) -> bool {
    candidate
        .reasons
        .iter()
        .all(|r| matches!(r, Reason::FolderNameRescue { .. }))
        && !candidate.reasons.is_empty()
}

// ── Promotion ────────────────────────────────────────────────────────

fn promote_winner(
    mut result: StagedMatchResult,
    best_id: usize,
    ai_scores: &HashMap<usize, f32>,
) -> StagedMatchResult {
    for candidate in &mut result.candidates_topk {
        let ai_score = ai_scores.get(&candidate.entry_id).copied().unwrap_or(0.0);
        candidate.score = (ai_score * 100.0).clamp(0.0, 100.0);

        if candidate.entry_id != best_id {
            continue;
        }

        candidate.confidence = Confidence::High;
        candidate.reasons.push(Reason::AiRerank { ai_score });
        cap_reasons(&mut candidate.reasons);
    }

    sort_candidates_deterministic(&mut result.candidates_topk);
    let best = result
        .candidates_topk
        .iter()
        .find(|c| c.entry_id == best_id)
        .cloned()
        .or_else(|| result.candidates_topk.first().cloned());

    StagedMatchResult {
        status: MatchStatus::AutoMatched,
        best,
        candidates_topk: result.candidates_topk,
        evidence: result.evidence,
    }
}

#[cfg(test)]
#[path = "../tests/analysis/mechanical_rerank_tests.rs"]
mod tests;
