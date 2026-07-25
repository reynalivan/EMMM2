//! Entry point and winner promotion for the mechanical reranker.

use std::collections::HashMap;

use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::analysis::scoring::{
    cap_reasons, has_primary_evidence,
};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{
    sort_candidates_deterministic, Confidence, MatchStatus, Reason, StagedMatchResult,
};

use super::config::MechanicalRerankConfig;
use super::points::compute_points;

const SCORE_DIVISOR: f32 = 30.0;
const MIN_POINT_DELTA: f32 = 1.0;

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
        candidates_all: result.candidates_all,
        evidence: result.evidence,
    }
}
