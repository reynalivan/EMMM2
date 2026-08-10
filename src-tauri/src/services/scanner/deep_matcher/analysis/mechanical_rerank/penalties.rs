//! Penalty detectors used by the points scorer.

use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{Candidate, Reason};

pub(super) fn count_foreign_strong_hits(
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
pub(super) fn is_multi_entity(_candidate: &Candidate) -> bool {
    false
}

/// Type mismatch detection: not yet tracked via Reason variants.
/// Placeholder for future `Reason::ObjectTypeMismatch`.
pub(super) fn has_type_mismatch(_candidate: &Candidate) -> bool {
    false
}

pub(super) fn is_rescue_only(candidate: &Candidate) -> bool {
    candidate.reasons.iter().all(|reason| {
        matches!(
            reason,
            Reason::FolderNameRescue { .. } | Reason::FuzzyName { .. }
        )
    }) && !candidate.reasons.is_empty()
}
