use super::*;
use crate::services::scanner::deep_matcher::{Evidence, Reason};

fn candidate(name: &str, confidence: Confidence) -> Candidate {
    Candidate {
        entry_id: 0,
        name: name.to_string(),
        object_type: "Character".to_string(),
        score: 12.0,
        confidence,
        reasons: vec![Reason::AliasStrict {
            alias: "sunset".to_string(),
        }],
    }
}

#[test]
fn test_staged_primary_candidate_is_status_aware() {
    let auto = StagedMatchResult {
        status: MatchStatus::AutoMatched,
        best: Some(candidate("Raiden", Confidence::High)),
        candidates_topk: Vec::new(),
        candidates_all: Vec::new(),
        evidence: Evidence::default(),
    };
    assert_eq!(
        staged_primary_candidate(&auto).map(|candidate| candidate.name.as_str()),
        Some("Raiden")
    );

    let review = StagedMatchResult {
        status: MatchStatus::NeedsReview,
        best: None,
        candidates_topk: vec![candidate("Amber", Confidence::Low)],
        candidates_all: Vec::new(),
        evidence: Evidence::default(),
    };
    assert_eq!(
        staged_primary_candidate(&review).map(|candidate| candidate.name.as_str()),
        Some("Amber")
    );

    let no_match = StagedMatchResult {
        status: MatchStatus::NoMatch,
        best: Some(candidate("Ignored", Confidence::High)),
        candidates_topk: Vec::new(),
        candidates_all: Vec::new(),
        evidence: Evidence::default(),
    };
    assert!(staged_primary_candidate(&no_match).is_none());
}

#[test]
fn test_staged_labels_and_detail_are_deterministic() {
    let auto = StagedMatchResult {
        status: MatchStatus::AutoMatched,
        best: Some(candidate("Raiden", Confidence::Medium)),
        candidates_topk: Vec::new(),
        candidates_all: Vec::new(),
        evidence: Evidence::default(),
    };
    assert_eq!(match_status_label(&auto.status), "AutoMatched");
    assert_eq!(staged_confidence_label(&auto), "Medium");
    assert_eq!(
        staged_match_detail(&auto),
        "Auto-matched via exact alias match ('sunset')"
    );

    let review = StagedMatchResult {
        status: MatchStatus::NeedsReview,
        best: Some(candidate("Amber", Confidence::High)),
        candidates_topk: vec![
            candidate("Amber", Confidence::High),
            candidate("Lisa", Confidence::High),
        ],
        candidates_all: Vec::new(),
        evidence: Evidence::default(),
    };
    assert_eq!(match_status_label(&review.status), "NeedsReview");
    assert_eq!(staged_confidence_label(&review), "Low");
    assert_eq!(
        staged_match_detail(&review),
        "Ambiguous top matches: Amber vs Lisa"
    );

    let no_match = StagedMatchResult::no_match();
    assert_eq!(match_status_label(&no_match.status), "NoMatch");
    assert_eq!(staged_confidence_label(&no_match), "None");
    assert_eq!(staged_match_detail(&no_match), "No reliable match found");
}
