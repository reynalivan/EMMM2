//! Scoring primitives for staged matcher refactor.

use crate::services::scanner::deep_matcher::{
    Evidence, Reason, ScoreState, MAX_EVIDENCE_HASHES, MAX_EVIDENCE_SECTIONS, MAX_EVIDENCE_TOKENS,
    MAX_REASONS_PER_CANDIDATE,
};

const SCORE_MIN: f32 = 0.0;
const SCORE_MAX: f32 = 100.0;

pub fn apply_hash_contribution(
    state: &mut ScoreState,
    overlap: u32,
    unique_overlap: u32,
    score_delta: f32,
) {
    if overlap == 0 {
        return;
    }

    state.overlap = state.overlap.saturating_add(overlap);
    state.unique_overlap = state.unique_overlap.saturating_add(unique_overlap);
    add_score(state, score_delta);
    upsert_hash_reason(state);
}

pub fn apply_alias_contribution(state: &mut ScoreState, alias: &str, score_delta: f32) {
    if alias.trim().is_empty() {
        return;
    }

    add_score(state, score_delta);
    push_reason_capped(
        state,
        Reason::AliasStrict {
            alias: alias.to_string(),
        },
    );
}

pub fn apply_token_overlap_contribution(state: &mut ScoreState, ratio: f32, scale: f32) {
    let bounded_ratio = ratio.clamp(0.0, 1.0);
    add_score(state, bounded_ratio * scale.max(0.0));
    push_reason_capped(
        state,
        Reason::TokenOverlap {
            ratio: bounded_ratio,
        },
    );
}

pub fn apply_deep_token_contribution(
    state: &mut ScoreState,
    matched_tokens: &[String],
    ratio: f32,
    ratio_weight: f32,
    per_token_boost: f32,
    token_boost_cap: f32,
) {
    let bounded_ratio = ratio.clamp(0.0, 1.0);
    let token_hits = sorted_unique_tokens(matched_tokens);
    let token_bonus =
        ((token_hits.len() as f32) * per_token_boost.max(0.0)).min(token_boost_cap.max(0.0));
    add_score(state, (bounded_ratio * ratio_weight.max(0.0)) + token_bonus);

    for token in token_hits {
        push_reason_capped(state, Reason::DeepNameToken { token });
    }
}

pub fn apply_ini_token_contribution(
    state: &mut ScoreState,
    section_tokens: &[String],
    content_tokens: &[String],
    ratio: f32,
    ratio_weight: f32,
) {
    let bounded_ratio = ratio.clamp(0.0, 1.0);
    add_score(state, bounded_ratio * ratio_weight.max(0.0));

    for token in sorted_unique_tokens(section_tokens) {
        push_reason_capped(state, Reason::IniSectionToken { token });
    }
    for token in sorted_unique_tokens(content_tokens) {
        push_reason_capped(state, Reason::IniContentToken { token });
    }
}

pub fn apply_direct_name_support_contribution(
    state: &mut ScoreState,
    name_tokens: &[String],
    tag_tokens: &[String],
    name_weight: f32,
    tag_weight: f32,
    name_cap: f32,
    tag_cap: f32,
) {
    let name_hits = sorted_unique_tokens(name_tokens);
    let tag_hits = sorted_unique_tokens(tag_tokens);

    let name_bonus = ((name_hits.len() as f32) * name_weight.max(0.0)).min(name_cap.max(0.0));
    let tag_bonus = ((tag_hits.len() as f32) * tag_weight.max(0.0)).min(tag_cap.max(0.0));
    add_score(state, name_bonus + tag_bonus);

    for token in name_hits.into_iter().chain(tag_hits) {
        push_reason_capped(state, Reason::DirectNameSupport { token });
    }
}

pub fn has_primary_evidence(reasons: &[Reason]) -> bool {
    reasons.iter().any(|reason| match reason {
        Reason::HashOverlap { overlap, .. } => *overlap >= 1,
        Reason::AliasStrict { .. }
        | Reason::DeepNameToken { .. }
        | Reason::IniSectionToken { .. }
        | Reason::IniContentToken { .. }
        | Reason::SubstringName { .. } => true,
        Reason::DirectNameSupport { .. }
        | Reason::TokenOverlap { .. }
        | Reason::AiRerank { .. }
        | Reason::NegativeEvidence { .. }
        | Reason::FolderNameRescue { .. } => false,
    })
}

pub fn cap_evidence(evidence: &mut Evidence) {
    evidence.matched_hashes = cap_tokens(&evidence.matched_hashes, MAX_EVIDENCE_HASHES);
    evidence.matched_tokens = cap_tokens(&evidence.matched_tokens, MAX_EVIDENCE_TOKENS);
    evidence.matched_sections = cap_tokens(&evidence.matched_sections, MAX_EVIDENCE_SECTIONS);
}

fn add_score(state: &mut ScoreState, delta: f32) {
    state.score = (state.score + delta).clamp(SCORE_MIN, SCORE_MAX);
}

fn upsert_hash_reason(state: &mut ScoreState) {
    let replacement = Reason::HashOverlap {
        overlap: state.overlap,
        unique_overlap: state.unique_overlap,
    };

    if let Some(index) = state
        .reasons
        .iter()
        .position(|reason| matches!(reason, Reason::HashOverlap { .. }))
    {
        state.reasons[index] = replacement;
    } else {
        state.reasons.push(replacement);
    }

    cap_reasons(&mut state.reasons);
}

pub(crate) fn push_reason_capped(state: &mut ScoreState, reason: Reason) {
    state.reasons.push(reason);
    cap_reasons(&mut state.reasons);
}

pub(crate) fn cap_reasons(reasons: &mut Vec<Reason>) {
    if reasons.len() <= MAX_REASONS_PER_CANDIDATE {
        return;
    }

    let mut prioritized: Vec<(usize, usize, Reason)> = reasons
        .drain(..)
        .enumerate()
        .map(|(index, reason)| (reason_priority(&reason), index, reason))
        .collect();

    prioritized.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    prioritized.truncate(MAX_REASONS_PER_CANDIDATE);
    prioritized.sort_by_key(|(_, index, _)| *index);

    *reasons = prioritized
        .into_iter()
        .map(|(_, _, reason)| reason)
        .collect();
}

fn reason_priority(reason: &Reason) -> usize {
    match reason {
        Reason::HashOverlap { .. } => 0,
        Reason::AliasStrict { .. } => 1,
        Reason::SubstringName { .. } => 2,
        Reason::NegativeEvidence { .. } => 3,
        Reason::TokenOverlap { .. } => 4,
        Reason::DeepNameToken { .. } => 5,
        Reason::IniSectionToken { .. } => 6,
        Reason::IniContentToken { .. } => 7,
        Reason::DirectNameSupport { .. } => 8,
        Reason::AiRerank { .. } => 9,
        Reason::FolderNameRescue { .. } => 10,
    }
}

fn cap_tokens(tokens: &[String], cap: usize) -> Vec<String> {
    let mut deduped = sorted_unique_tokens(tokens);
    if deduped.len() > cap {
        deduped.truncate(cap);
    }
    deduped
}

fn sorted_unique_tokens(tokens: &[String]) -> Vec<String> {
    let mut values = tokens.to_vec();
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
#[path = "../tests/analysis/scoring_tests.rs"]
mod tests;
