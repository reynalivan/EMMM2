use std::collections::HashMap;

use super::{finalize_review, try_stage_accept, FinalizeConfig, StageAcceptConfig};
use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::pipeline::stages::ObservedTokenBuckets;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::{
    Confidence, DbEntry, MatchMode, MatchStatus, Reason, ScoreState,
};

fn db_entry(name: &str, tags: &[String], hashes: &[String]) -> DbEntry {
    DbEntry {
        name: name.to_string(),
        tags: tags.to_vec(),
        object_type: "Character".to_string(),
        custom_skins: vec![],
        thumbnail_path: None,
        metadata: None,
        hash_db: std::collections::HashMap::from([("Default".to_string(), hashes.to_vec())]),
    }
}

fn score_state(score: f32, reasons: Vec<Reason>) -> ScoreState {
    ScoreState {
        score,
        reasons,
        overlap: 0,
        unique_overlap: 0,
        max_confidence: Confidence::None,
    }
}

// Covers: TC-2.2-Task15-01
#[test]
fn test_review_result_assembles_deterministic_best_topk_and_evidence() {
    let db = MasterDb::new(vec![
        db_entry("Beta Hero", &[], &["cccc0003".to_string()]),
        db_entry("Alpha Hero", &[], &["aaaa0001".to_string()]),
        db_entry("Alpha Hero", &[], &["bbbb0002".to_string()]),
    ]);

    let signals = FolderSignals {
        ini_hashes: vec![
            "cccc0003".to_string(),
            "bbbb0002".to_string(),
            "aaaa0001".to_string(),
        ],
        ..FolderSignals::default()
    };

    let states_a: HashMap<usize, ScoreState> = HashMap::from([
        (
            0,
            score_state(
                18.0,
                vec![Reason::AliasStrict {
                    alias: "beta-hero".to_string(),
                }],
            ),
        ),
        (
            1,
            score_state(
                20.0,
                vec![Reason::AliasStrict {
                    alias: "alpha-main".to_string(),
                }],
            ),
        ),
        (
            2,
            score_state(
                20.0,
                vec![Reason::AliasStrict {
                    alias: "alpha-alt".to_string(),
                }],
            ),
        ),
    ]);

    let states_b: HashMap<usize, ScoreState> = HashMap::from([
        (
            2,
            score_state(
                20.0,
                vec![Reason::AliasStrict {
                    alias: "alpha-alt".to_string(),
                }],
            ),
        ),
        (
            0,
            score_state(
                18.0,
                vec![Reason::AliasStrict {
                    alias: "beta-hero".to_string(),
                }],
            ),
        ),
        (
            1,
            score_state(
                20.0,
                vec![Reason::AliasStrict {
                    alias: "alpha-main".to_string(),
                }],
            ),
        ),
    ]);

    let config = FinalizeConfig {
        mode: MatchMode::Quick,
        review_min_score: 10.0,
        top_k: 2,
    };

    let result_a = finalize_review(
        &db,
        &states_a,
        &signals,
        &ObservedTokenBuckets::default(),
        None,
        &config,
    );
    let result_b = finalize_review(
        &db,
        &states_b,
        &signals,
        &ObservedTokenBuckets::default(),
        None,
        &config,
    );

    assert_eq!(result_a.status, MatchStatus::NeedsReview);
    let ids_a: Vec<usize> = result_a
        .candidates_topk
        .iter()
        .map(|candidate| candidate.entry_id)
        .collect();
    let ids_b: Vec<usize> = result_b
        .candidates_topk
        .iter()
        .map(|candidate| candidate.entry_id)
        .collect();

    assert_eq!(ids_a, vec![1, 2]);
    assert_eq!(ids_b, ids_a);
    assert_eq!(
        result_a.best.as_ref().map(|candidate| candidate.entry_id),
        Some(1)
    );
    assert_eq!(
        result_b.best.as_ref().map(|candidate| candidate.entry_id),
        Some(1)
    );
    assert_eq!(
        result_a.evidence.matched_hashes,
        vec!["aaaa0001".to_string()]
    );
    assert_eq!(
        result_b.evidence.matched_hashes,
        vec!["aaaa0001".to_string()]
    );
}

// Covers: TC-2.2-Task15-02
#[test]
fn test_summary_text_is_deterministic_for_auto_review_and_no_match() {
    let db = MasterDb::new(vec![
        db_entry("Alpha Hero", &[], &["aaaa0001".to_string()]),
        db_entry("Beta Hero", &[], &["bbbb0002".to_string()]),
    ]);

    let auto_states = HashMap::from([(
        0,
        score_state(
            22.0,
            vec![Reason::HashOverlap {
                overlap: 1,
                unique_overlap: 1,
            }],
        ),
    )]);
    let auto_result = try_stage_accept(
        &db,
        &auto_states,
        &FolderSignals::default(),
        &ObservedTokenBuckets::default(),
        None,
        &StageAcceptConfig {
            mode: MatchMode::Quick,
            threshold: 10.0,
            margin: 4.0,
            review_min_score: 10.0,
            top_k: 5,
            best_confidence: Confidence::High,
        },
    )
    .expect("auto match result");
    assert_eq!(auto_result.status, MatchStatus::AutoMatched);
    assert_eq!(
        auto_result.summary(),
        "Auto-matched via 1 exact hash match(es)"
    );

    let review_states = HashMap::from([
        (
            0,
            score_state(
                15.0,
                vec![Reason::AliasStrict {
                    alias: "alpha".to_string(),
                }],
            ),
        ),
        (
            1,
            score_state(
                14.0,
                vec![Reason::AliasStrict {
                    alias: "beta".to_string(),
                }],
            ),
        ),
    ]);
    let review_result = finalize_review(
        &db,
        &review_states,
        &FolderSignals::default(),
        &ObservedTokenBuckets::default(),
        None,
        &FinalizeConfig {
            mode: MatchMode::Quick,
            review_min_score: 10.0,
            top_k: 5,
        },
    );
    assert_eq!(review_result.status, MatchStatus::NeedsReview);
    assert_eq!(
        review_result.summary(),
        "Ambiguous top matches: Alpha Hero vs Beta Hero"
    );

    let no_match_states = HashMap::from([(
        0,
        score_state(3.0, vec![Reason::TokenOverlap { ratio: 0.1 }]),
    )]);
    let no_match_result = finalize_review(
        &db,
        &no_match_states,
        &FolderSignals::default(),
        &ObservedTokenBuckets::default(),
        None,
        &FinalizeConfig {
            mode: MatchMode::Quick,
            review_min_score: 10.0,
            top_k: 5,
        },
    );
    assert_eq!(no_match_result.status, MatchStatus::NoMatch);
    assert_eq!(no_match_result.summary(), "No reliable match found");
}

// Covers: TC-2.2-Task15-03
#[test]
fn test_review_evidence_caps_are_deterministic_in_final_result() {
    let hashes: Vec<String> = (0..80).map(|idx| format!("{idx:08x}")).collect();
    let tokens: Vec<String> = (0..80).map(|idx| format!("token{idx:02}")).collect();
    let sections: Vec<String> = (0..80).map(|idx| format!("section{idx:02}")).collect();

    let mut tags = tokens.clone();
    tags.extend(sections.clone());

    let db = MasterDb::new(vec![db_entry("Evidence Hero", &tags, &hashes)]);

    let mut folder_tokens = tokens.clone();
    folder_tokens.reverse();
    let mut section_tokens = sections.clone();
    section_tokens.reverse();
    let mut ini_hashes = hashes.clone();
    ini_hashes.reverse();

    let signals = FolderSignals {
        folder_tokens,
        ini_section_tokens: section_tokens,
        ini_hashes,
        ..FolderSignals::default()
    };

    let states = HashMap::from([(
        0,
        score_state(
            21.0,
            vec![Reason::AliasStrict {
                alias: "evidence-hero".to_string(),
            }],
        ),
    )]);

    let result = finalize_review(
        &db,
        &states,
        &signals,
        &ObservedTokenBuckets::default(),
        None,
        &FinalizeConfig {
            mode: MatchMode::Quick,
            review_min_score: 10.0,
            top_k: 5,
        },
    );

    assert_eq!(result.status, MatchStatus::NeedsReview);
    assert_eq!(result.evidence.matched_hashes.len(), 50);
    assert_eq!(result.evidence.matched_tokens.len(), 50);
    assert_eq!(result.evidence.matched_sections.len(), 50);
    assert_eq!(result.evidence.matched_hashes[0], "00000000");
    assert_eq!(result.evidence.matched_tokens[0], "token00");
    assert_eq!(result.evidence.matched_sections[0], "section00");
}
