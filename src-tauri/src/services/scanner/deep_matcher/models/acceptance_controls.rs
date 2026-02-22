use std::collections::{BTreeSet, HashMap};

use crate::services::scanner::deep_matcher::analysis::scoring::cap_reasons;
use crate::services::scanner::deep_matcher::pipeline::quick_pipeline_result::collect_candidates;
use crate::services::scanner::deep_matcher::pipeline::stages::ObservedTokenBuckets;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{
    sort_candidates_deterministic, Candidate, MatchMode, Reason, ScoreState,
};

const OBJECT_TYPE_MISMATCH_PENALTY: f32 = 2.0;
const QUICK_NEGATIVE_PENALTY_PER_HIT: f32 = 1.5;
const QUICK_NEGATIVE_PENALTY_CAP: f32 = 8.0;
const FULL_NEGATIVE_PENALTY_PER_HIT: f32 = 2.5;
const FULL_NEGATIVE_PENALTY_CAP: f32 = 10.0;
const ULTRA_CLOSE_PRIMARY_MARGIN: f32 = 1.0;
const ULTRA_CLOSE_ABSOLUTE_MARGIN: f32 = 0.5;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AmbiguitySnapshot {
    pub margin_conflict: bool,
    pub ultra_close_primary: bool,
    pub ultra_close_any: bool,
    pub pack_multi_entity: bool,
}

pub(super) fn collect_candidates_with_controls(
    db: &MasterDb,
    states: &HashMap<usize, ScoreState>,
    observed_buckets: &ObservedTokenBuckets,
    object_type_context: Option<&str>,
    mode: MatchMode,
) -> Vec<Candidate> {
    let mut candidates = collect_candidates(db, states);
    apply_negative_evidence_penalties(db, observed_buckets, mode, &mut candidates);
    apply_object_type_mismatch_penalty(object_type_context, &mut candidates);
    sort_candidates_deterministic(&mut candidates);
    candidates
}

pub(super) fn primary_evidence_flags(
    db: &MasterDb,
    candidates: &[Candidate],
    observed_buckets: &ObservedTokenBuckets,
) -> Vec<bool> {
    candidates
        .iter()
        .map(|candidate| has_primary_evidence_for_candidate(db, candidate, observed_buckets))
        .collect()
}

pub(super) fn build_ambiguity_snapshot(
    candidates: &[Candidate],
    primary_flags: &[bool],
    review_min_score: f32,
    margin: Option<f32>,
) -> AmbiguitySnapshot {
    let primary_review_count = candidates
        .iter()
        .zip(primary_flags)
        .filter(|(candidate, is_primary)| **is_primary && candidate.score >= review_min_score)
        .count();
    let pack_multi_entity = primary_review_count >= 2;

    if candidates.len() < 2 {
        return AmbiguitySnapshot {
            pack_multi_entity,
            ..AmbiguitySnapshot::default()
        };
    }

    let margin_gap = candidates[0].score - candidates[1].score;
    let top2_primary = primary_flags[0] && primary_flags[1];

    AmbiguitySnapshot {
        margin_conflict: margin.is_some_and(|value| top2_primary && margin_gap < value),
        ultra_close_primary: top2_primary && margin_gap < ULTRA_CLOSE_PRIMARY_MARGIN,
        ultra_close_any: margin_gap < ULTRA_CLOSE_ABSOLUTE_MARGIN,
        pack_multi_entity,
    }
}

fn apply_negative_evidence_penalties(
    db: &MasterDb,
    observed_buckets: &ObservedTokenBuckets,
    mode: MatchMode,
    candidates: &mut [Candidate],
) {
    let strong_tokens = strong_observed_tokens(db, observed_buckets, mode);
    if strong_tokens.is_empty() {
        return;
    }

    for candidate in candidates.iter_mut() {
        let entry_tokens = &db.keywords[candidate.entry_id].1;
        let mut foreign_strong_hits = 0_u32;

        for (token, posting) in &strong_tokens {
            if entry_tokens.contains(token) {
                continue;
            }
            if posting
                .iter()
                .any(|entry_id| *entry_id != candidate.entry_id)
            {
                foreign_strong_hits = foreign_strong_hits.saturating_add(1);
            }
        }

        if foreign_strong_hits == 0 {
            continue;
        }

        let penalty = negative_penalty(mode, foreign_strong_hits);
        if penalty > 0.0 {
            candidate.score = (candidate.score - penalty).max(0.0);
            upsert_negative_reason(&mut candidate.reasons, foreign_strong_hits);
        }
    }
}

fn strong_observed_tokens(
    db: &MasterDb,
    observed_buckets: &ObservedTokenBuckets,
    mode: MatchMode,
) -> Vec<(String, Vec<usize>)> {
    let full_df_cap = (db.entries.len() / 200).max(3);

    observed_buckets
        .observed_tokens()
        .into_iter()
        .filter_map(|token| {
            let posting = db.indexes.token_index.get(&token)?;
            if posting.is_empty() {
                return None;
            }

            let is_strong = match mode {
                MatchMode::Quick => token.len() >= 5 && posting.len() <= 2,
                MatchMode::FullScoring => posting.len() <= full_df_cap,
            };
            if !is_strong {
                return None;
            }

            Some((token, posting.clone()))
        })
        .collect()
}

fn negative_penalty(mode: MatchMode, foreign_strong_hits: u32) -> f32 {
    let hits = foreign_strong_hits as f32;
    match mode {
        MatchMode::Quick => (hits * QUICK_NEGATIVE_PENALTY_PER_HIT).min(QUICK_NEGATIVE_PENALTY_CAP),
        MatchMode::FullScoring => {
            (hits * FULL_NEGATIVE_PENALTY_PER_HIT).min(FULL_NEGATIVE_PENALTY_CAP)
        }
    }
}

fn upsert_negative_reason(reasons: &mut Vec<Reason>, foreign_strong_hits: u32) {
    if let Some(existing) = reasons.iter_mut().find_map(|reason| {
        if let Reason::NegativeEvidence {
            foreign_strong_hits,
        } = reason
        {
            Some(foreign_strong_hits)
        } else {
            None
        }
    }) {
        *existing = foreign_strong_hits;
    } else {
        reasons.push(Reason::NegativeEvidence {
            foreign_strong_hits,
        });
    }
    cap_reasons(reasons);
}

fn apply_object_type_mismatch_penalty(
    object_type_context: Option<&str>,
    candidates: &mut [Candidate],
) {
    let Some(expected_type) = object_type_context
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    for candidate in candidates.iter_mut() {
        if candidate.object_type.eq_ignore_ascii_case(expected_type) {
            continue;
        }
        candidate.score = (candidate.score - OBJECT_TYPE_MISMATCH_PENALTY).max(0.0);
    }
}

fn has_primary_evidence_for_candidate(
    db: &MasterDb,
    candidate: &Candidate,
    observed_buckets: &ObservedTokenBuckets,
) -> bool {
    let entry_tokens = &db.keywords[candidate.entry_id].1;
    let deep_hits = count_bucket_hits(&observed_buckets.deep_name_tokens, entry_tokens);
    let ini_section_hits = count_bucket_hits(&observed_buckets.ini_section_tokens, entry_tokens);
    let ini_content_hits = count_bucket_hits(&observed_buckets.ini_content_tokens, entry_tokens);

    for reason in &candidate.reasons {
        match reason {
            Reason::HashOverlap { overlap, .. } if *overlap >= 1 => return true,
            Reason::AliasStrict { .. } | Reason::SubstringName { .. } => return true,
            Reason::DeepNameToken { .. }
            | Reason::IniSectionToken { .. }
            | Reason::IniContentToken { .. }
            | Reason::HashOverlap { .. }
            | Reason::TokenOverlap { .. }
            | Reason::DirectNameSupport { .. }
            | Reason::AiRerank { .. }
            | Reason::NegativeEvidence { .. }
            | Reason::FolderNameRescue { .. } => {}
        }
    }

    if (ini_section_hits + ini_content_hits) >= 1 {
        return true;
    }

    if deep_hits == 0 {
        return false;
    }

    let deep_ratio = (deep_hits as f32) / (observed_buckets.deep_name_tokens.len().max(1) as f32);
    deep_hits >= 2 || deep_ratio >= 0.12
}

fn count_bucket_hits(
    bucket: &BTreeSet<String>,
    entry_tokens: &std::collections::HashSet<String>,
) -> usize {
    bucket
        .iter()
        .filter(|token| entry_tokens.contains(*token))
        .count()
}
