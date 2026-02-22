use std::collections::HashMap;

use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::pipeline::quick_pipeline_result::{
    build_evidence, empty_evidence,
};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{
    sort_candidates_deterministic, Candidate, Confidence, MatchMode, MatchStatus, ScoreState,
    StagedMatchResult,
};

#[cfg(feature = "debug_matcher")]
use log::debug;

#[path = "acceptance_controls.rs"]
mod acceptance_controls;

use acceptance_controls::{
    build_ambiguity_snapshot, collect_candidates_with_controls, primary_evidence_flags,
};

#[derive(Debug, Clone)]
pub struct StageAcceptConfig {
    pub mode: MatchMode,
    pub threshold: f32,
    pub margin: f32,
    pub review_min_score: f32,
    pub top_k: usize,
    pub best_confidence: Confidence,
}

#[derive(Debug, Clone, Copy)]
pub struct FinalizeConfig {
    pub mode: MatchMode,
    pub review_min_score: f32,
    pub top_k: usize,
}

pub fn try_stage_accept(
    db: &MasterDb,
    states: &HashMap<usize, ScoreState>,
    signals: &FolderSignals,
    observed_buckets: &crate::services::scanner::deep_matcher::pipeline::stages::ObservedTokenBuckets,
    object_type_context: Option<&str>,
    config: &StageAcceptConfig,
) -> Option<StagedMatchResult> {
    let mut candidates = collect_candidates_with_controls(
        db,
        states,
        observed_buckets,
        object_type_context,
        config.mode,
    );
    let best = candidates.first()?.clone();
    if best.score < config.threshold {
        #[cfg(feature = "debug_matcher")]
        debug!(
            "[MATCHER_CALIBRATION] try_stage_accept: threshold_not_met | mode={:?} best_score={:.2} threshold={:.2}",
            config.mode, best.score, config.threshold
        );
        return None;
    }

    let primary_flags = primary_evidence_flags(db, &candidates, observed_buckets);
    if !primary_flags.first().copied().unwrap_or(false) {
        #[cfg(feature = "debug_matcher")]
        debug!(
            "[MATCHER_CALIBRATION] try_stage_accept: no_primary_evidence | mode={:?} best_score={:.2}",
            config.mode, best.score
        );
        return None;
    }

    let ambiguity = build_ambiguity_snapshot(
        &candidates,
        &primary_flags,
        config.review_min_score,
        Some(config.margin),
    );
    if ambiguity.margin_conflict {
        #[cfg(feature = "debug_matcher")]
        log_stage_decision(
            config.mode,
            &candidates,
            &primary_flags,
            signals,
            "margin_conflict_review",
        );
        return Some(build_review_result(
            db,
            signals,
            &mut candidates,
            config.top_k,
        ));
    }

    let second_score = candidates
        .get(1)
        .map(|candidate| candidate.score)
        .unwrap_or(0.0);
    if (best.score - second_score) < config.margin {
        #[cfg(feature = "debug_matcher")]
        debug!(
            "[MATCHER_CALIBRATION] try_stage_accept: margin_insufficient | mode={:?} best_score={:.2} second_score={:.2} margin={:.2}",
            config.mode, best.score, second_score, config.margin
        );
        return None;
    }

    if ambiguity.ultra_close_primary || ambiguity.ultra_close_any || ambiguity.pack_multi_entity {
        #[cfg(feature = "debug_matcher")]
        log_stage_decision(
            config.mode,
            &candidates,
            &primary_flags,
            signals,
            "ambiguity_forced_review",
        );
        return Some(build_review_result(
            db,
            signals,
            &mut candidates,
            config.top_k,
        ));
    }

    #[cfg(feature = "debug_matcher")]
    log_stage_decision(
        config.mode,
        &candidates,
        &primary_flags,
        signals,
        "auto_matched",
    );

    Some(build_auto_matched_result(
        db,
        signals,
        &mut candidates,
        config.top_k,
        &config.best_confidence,
    ))
}

pub fn finalize_review(
    db: &MasterDb,
    states: &HashMap<usize, ScoreState>,
    signals: &FolderSignals,
    observed_buckets: &crate::services::scanner::deep_matcher::pipeline::stages::ObservedTokenBuckets,
    object_type_context: Option<&str>,
    config: &FinalizeConfig,
) -> StagedMatchResult {
    let mut candidates = collect_candidates_with_controls(
        db,
        states,
        observed_buckets,
        object_type_context,
        config.mode,
    );

    let Some(best) = candidates.first().cloned() else {
        #[cfg(feature = "debug_matcher")]
        debug!(
            "[MATCHER_CALIBRATION] finalize_review: no_candidates | mode={:?} scanned_ini={} scanned_names={}",
            config.mode, signals.scanned_ini_files, signals.scanned_name_items
        );
        return no_match_result(signals);
    };

    let primary_flags = primary_evidence_flags(db, &candidates, observed_buckets);
    let ambiguity =
        build_ambiguity_snapshot(&candidates, &primary_flags, config.review_min_score, None);

    #[cfg(feature = "debug_matcher")]
    {
        let second_score = candidates.get(1).map(|c| c.score).unwrap_or(0.0);
        debug!(
            "[MATCHER_CALIBRATION] finalize_review: decision | mode={:?} best_score={:.2} second_score={:.2} margin={:.2} primary_evidence={} pack_multi={} scanned_ini={} scanned_names={}",
            config.mode, best.score, second_score, best.score - second_score,
            primary_flags.first().copied().unwrap_or(false), ambiguity.pack_multi_entity,
            signals.scanned_ini_files, signals.scanned_name_items
        );
    }

    if ambiguity.pack_multi_entity || best.score >= config.review_min_score {
        return build_review_result(db, signals, &mut candidates, config.top_k);
    }

    StagedMatchResult {
        status: MatchStatus::NoMatch,
        best: None,
        candidates_topk: Vec::new(),
        evidence: build_evidence(db, signals, &best),
    }
}

fn build_review_result(
    db: &MasterDb,
    signals: &FolderSignals,
    candidates: &mut Vec<Candidate>,
    top_k: usize,
) -> StagedMatchResult {
    assemble_ranked_result(
        db,
        signals,
        candidates,
        top_k,
        MatchStatus::NeedsReview,
        None,
    )
}

fn no_match_result(signals: &FolderSignals) -> StagedMatchResult {
    StagedMatchResult {
        status: MatchStatus::NoMatch,
        best: None,
        candidates_topk: Vec::new(),
        evidence: empty_evidence(signals),
    }
}

fn build_auto_matched_result(
    db: &MasterDb,
    signals: &FolderSignals,
    candidates: &mut Vec<Candidate>,
    top_k: usize,
    best_confidence: &Confidence,
) -> StagedMatchResult {
    assemble_ranked_result(
        db,
        signals,
        candidates,
        top_k,
        MatchStatus::AutoMatched,
        Some(best_confidence),
    )
}

fn assemble_ranked_result(
    db: &MasterDb,
    signals: &FolderSignals,
    candidates: &mut Vec<Candidate>,
    top_k: usize,
    status: MatchStatus,
    best_confidence: Option<&Confidence>,
) -> StagedMatchResult {
    sort_candidates_deterministic(candidates);
    candidates.truncate(top_k.max(1));

    if let Some(confidence) = best_confidence {
        if let Some(best_candidate) = candidates.first_mut() {
            best_candidate.confidence = std::cmp::max(best_candidate.confidence, *confidence);
        }
    }

    let best = candidates.first().cloned();
    let evidence = best
        .as_ref()
        .map(|candidate| build_evidence(db, signals, candidate))
        .unwrap_or_else(|| empty_evidence(signals));

    StagedMatchResult {
        status,
        best,
        evidence,
        candidates_topk: candidates.clone(),
    }
}

#[cfg(feature = "debug_matcher")]
fn log_stage_decision(
    mode: MatchMode,
    candidates: &[Candidate],
    primary_flags: &[bool],
    signals: &FolderSignals,
    decision: &str,
) {
    let best_score = candidates.first().map(|c| c.score).unwrap_or(0.0);
    let second_score = candidates.get(1).map(|c| c.score).unwrap_or(0.0);
    let margin = best_score - second_score;
    let best_has_primary = primary_flags.first().copied().unwrap_or(false);
    let second_has_primary = primary_flags.get(1).copied().unwrap_or(false);
    let foreign_hits = candidates
        .first()
        .and_then(|c| {
            c.reasons.iter().find_map(|r| {
                if let crate::services::scanner::deep_matcher::Reason::NegativeEvidence {
                    foreign_strong_hits,
                } = r
                {
                    Some(*foreign_strong_hits)
                } else {
                    None
                }
            })
        })
        .unwrap_or(0);

    debug!(
        "[MATCHER_CALIBRATION] stage_decision: {} | mode={:?} best={:.2} second={:.2} margin={:.2} primary=[{},{}] foreign_hits={} scanned_ini={} scanned_names={}",
        decision, mode, best_score, second_score, margin,
        best_has_primary, second_has_primary, foreign_hits,
        signals.scanned_ini_files, signals.scanned_name_items
    );
}

#[cfg(test)]
#[path = "../tests/models/acceptance_tests.rs"]
mod acceptance_tests;

#[cfg(test)]
#[path = "../tests/acceptance_result_tests.rs"]
mod acceptance_result_tests;
