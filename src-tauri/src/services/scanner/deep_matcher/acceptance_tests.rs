use std::collections::HashMap;

use super::super::content::FolderSignals;
use super::super::stages::ObservedTokenBuckets;
use super::super::types::{Confidence, DbEntry, MatchMode, MatchStatus, Reason, ScoreState};
use super::super::MasterDb;
use super::{finalize_review, try_stage_accept, FinalizeConfig, StageAcceptConfig};

fn db_entry(name: &str, tags: &[&str], object_type: &str) -> DbEntry {
    DbEntry {
        name: name.to_string(),
        tags: tags.iter().map(|tag| tag.to_string()).collect(),
        object_type: object_type.to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hashes: vec![],
    }
}

fn score_state(score: f32, reasons: Vec<Reason>) -> ScoreState {
    ScoreState {
        score,
        reasons,
        overlap: 0,
        unique_overlap: 0,
    }
}

fn buckets(
    folder: &[&str],
    deep: &[&str],
    ini_section: &[&str],
    ini_content: &[&str],
) -> ObservedTokenBuckets {
    ObservedTokenBuckets {
        folder_tokens: folder.iter().map(|token| token.to_string()).collect(),
        deep_name_tokens: deep.iter().map(|token| token.to_string()).collect(),
        ini_section_tokens: ini_section.iter().map(|token| token.to_string()).collect(),
        ini_content_tokens: ini_content.iter().map(|token| token.to_string()).collect(),
    }
}

fn empty_signals() -> FolderSignals {
    FolderSignals::default()
}

// Covers: TC-2.2-Task13-01
#[test]
fn test_score_and_margin_pass_but_primary_evidence_gate_blocks_auto_match() {
    let db = MasterDb::new(vec![
        db_entry("Alpha Hero", &["alpha"], "Character"),
        db_entry("Beta Hero", &["beta"], "Character"),
    ]);
    let states: HashMap<usize, ScoreState> = [
        (
            0,
            score_state(20.0, vec![Reason::TokenOverlap { ratio: 0.9 }]),
        ),
        (
            1,
            score_state(8.0, vec![Reason::TokenOverlap { ratio: 0.3 }]),
        ),
    ]
    .into_iter()
    .collect();

    let stage = try_stage_accept(
        &db,
        &states,
        &empty_signals(),
        &ObservedTokenBuckets::default(),
        None,
        &StageAcceptConfig {
            mode: MatchMode::Quick,
            threshold: 12.0,
            margin: 4.0,
            review_min_score: 10.0,
            top_k: 5,
            best_confidence: Confidence::Medium,
        },
    );

    assert!(stage.is_none());

    let finalized = finalize_review(
        &db,
        &states,
        &empty_signals(),
        &ObservedTokenBuckets::default(),
        None,
        &FinalizeConfig {
            mode: MatchMode::Quick,
            review_min_score: 10.0,
            top_k: 5,
        },
    );
    assert_eq!(finalized.status, MatchStatus::NeedsReview);
}

// Covers: TC-2.2-Task13-02
#[test]
fn test_ultra_close_top2_with_primary_evidence_forces_review() {
    let db = MasterDb::new(vec![
        db_entry("Alpha Hero", &["alpha"], "Character"),
        db_entry("Beta Hero", &["beta"], "Character"),
    ]);
    let states: HashMap<usize, ScoreState> = [
        (
            0,
            score_state(
                18.0,
                vec![Reason::AliasStrict {
                    alias: "alphaprime".to_string(),
                }],
            ),
        ),
        (
            1,
            score_state(
                17.4,
                vec![Reason::HashOverlap {
                    overlap: 1,
                    unique_overlap: 0,
                }],
            ),
        ),
    ]
    .into_iter()
    .collect();

    let stage = try_stage_accept(
        &db,
        &states,
        &empty_signals(),
        &ObservedTokenBuckets::default(),
        None,
        &StageAcceptConfig {
            mode: MatchMode::Quick,
            threshold: 12.0,
            margin: 0.3,
            review_min_score: 10.0,
            top_k: 5,
            best_confidence: Confidence::High,
        },
    )
    .expect("stage decision");

    assert_eq!(stage.status, MatchStatus::NeedsReview);
}

// Covers: TC-2.2-Task13-03
#[test]
fn test_pack_multi_entity_primary_evidence_forces_review_even_with_margin_pass() {
    let db = MasterDb::new(vec![
        db_entry("Alpha Hero", &["alpha"], "Character"),
        db_entry("Beta Hero", &["beta"], "Character"),
    ]);
    let states: HashMap<usize, ScoreState> = [
        (
            0,
            score_state(
                30.0,
                vec![Reason::AliasStrict {
                    alias: "alphahero".to_string(),
                }],
            ),
        ),
        (
            1,
            score_state(
                18.0,
                vec![Reason::HashOverlap {
                    overlap: 1,
                    unique_overlap: 1,
                }],
            ),
        ),
    ]
    .into_iter()
    .collect();

    let stage = try_stage_accept(
        &db,
        &states,
        &empty_signals(),
        &ObservedTokenBuckets::default(),
        None,
        &StageAcceptConfig {
            mode: MatchMode::Quick,
            threshold: 12.0,
            margin: 4.0,
            review_min_score: 12.0,
            top_k: 5,
            best_confidence: Confidence::High,
        },
    )
    .expect("stage decision");

    assert_eq!(stage.status, MatchStatus::NeedsReview);
}

// Covers: TC-2.2-Task13-04
#[test]
fn test_negative_evidence_penalty_mixed_signal_forces_review() {
    let db = MasterDb::new(vec![
        db_entry("Alpha Prime", &["alphaonly"], "Character"),
        db_entry("Beta Prime", &["betaonly", "gammaonly"], "Character"),
    ]);
    let states: HashMap<usize, ScoreState> = [
        (
            0,
            score_state(
                18.0,
                vec![Reason::AliasStrict {
                    alias: "alphaprime".to_string(),
                }],
            ),
        ),
        (
            1,
            score_state(
                12.0,
                vec![Reason::AliasStrict {
                    alias: "betaprime".to_string(),
                }],
            ),
        ),
    ]
    .into_iter()
    .collect();

    let observed = buckets(&["alphaonly", "betaonly", "gammaonly"], &[], &[], &[]);
    let stage = try_stage_accept(
        &db,
        &states,
        &empty_signals(),
        &observed,
        None,
        &StageAcceptConfig {
            mode: MatchMode::FullScoring,
            threshold: 10.0,
            margin: 5.0,
            review_min_score: 12.0,
            top_k: 5,
            best_confidence: Confidence::High,
        },
    )
    .expect("stage decision");

    assert_eq!(stage.status, MatchStatus::NeedsReview);
    let best = stage.best.expect("best candidate");
    assert!(best
        .reasons
        .iter()
        .any(|reason| matches!(reason, Reason::NegativeEvidence { .. })));
}

// Covers: TC-2.2-Task13-05
#[test]
fn test_ultra_close_under_half_forces_review_even_without_top2_primary() {
    let db = MasterDb::new(vec![
        db_entry("Alpha Hero", &["alpha"], "Character"),
        db_entry("Beta Hero", &["beta"], "Character"),
    ]);
    let states: HashMap<usize, ScoreState> = [
        (
            0,
            score_state(
                20.0,
                vec![Reason::AliasStrict {
                    alias: "alphahero".to_string(),
                }],
            ),
        ),
        (
            1,
            score_state(19.6, vec![Reason::TokenOverlap { ratio: 0.8 }]),
        ),
    ]
    .into_iter()
    .collect();

    let stage = try_stage_accept(
        &db,
        &states,
        &empty_signals(),
        &ObservedTokenBuckets::default(),
        None,
        &StageAcceptConfig {
            mode: MatchMode::Quick,
            threshold: 12.0,
            margin: 0.2,
            review_min_score: 10.0,
            top_k: 5,
            best_confidence: Confidence::High,
        },
    )
    .expect("stage decision");

    assert_eq!(stage.status, MatchStatus::NeedsReview);
}

// Covers: TC-2.2-Task13-06
#[test]
fn test_object_type_mismatch_penalty_applies_when_context_exists() {
    let db = MasterDb::new(vec![
        db_entry("Character Entry", &["hero"], "Character"),
        db_entry("Weapon Entry", &["blade"], "Weapon"),
    ]);
    let states: HashMap<usize, ScoreState> = [
        (
            0,
            score_state(
                16.0,
                vec![Reason::AliasStrict {
                    alias: "character-entry".to_string(),
                }],
            ),
        ),
        (
            1,
            score_state(
                15.0,
                vec![Reason::AliasStrict {
                    alias: "weapon-entry".to_string(),
                }],
            ),
        ),
    ]
    .into_iter()
    .collect();

    let stage = try_stage_accept(
        &db,
        &states,
        &empty_signals(),
        &ObservedTokenBuckets::default(),
        Some("Weapon"),
        &StageAcceptConfig {
            mode: MatchMode::Quick,
            threshold: 12.0,
            margin: 0.5,
            review_min_score: 10.0,
            top_k: 5,
            best_confidence: Confidence::High,
        },
    )
    .expect("stage decision");

    assert_eq!(stage.status, MatchStatus::NeedsReview);
    assert_eq!(stage.best.expect("best").object_type, "Weapon");
}
