use super::*;

// Covers: TC-2.2-Task10-01
#[test]
fn test_score_clamped_reason_capped_and_evidence_capped_deterministically() {
    let mut state = ScoreState::new();

    apply_hash_contribution(&mut state, 1, 1, 90.0);
    apply_alias_contribution(&mut state, "alias-hit", 30.0);
    apply_token_overlap_contribution(&mut state, 1.0, 25.0);

    let deep_tokens: Vec<String> = (0..20).map(|idx| format!("deep{idx:02}")).collect();
    apply_deep_token_contribution(&mut state, &deep_tokens, 1.0, 20.0, 1.0, 8.0);

    assert_eq!(state.score, 100.0);
    assert_eq!(state.reasons.len(), MAX_REASONS_PER_CANDIDATE);
    assert!(matches!(state.reasons[0], Reason::HashOverlap { .. }));
    assert!(state
        .reasons
        .iter()
        .any(|reason| matches!(reason, Reason::AliasStrict { .. })));

    let mut evidence = Evidence {
        matched_hashes: (0..80).rev().map(|idx| format!("{idx:08x}")).collect(),
        matched_tokens: (0..80).rev().map(|idx| format!("token{idx:02}")).collect(),
        matched_sections: (0..80)
            .rev()
            .map(|idx| format!("section{idx:02}"))
            .collect(),
        scanned_ini_files: 0,
        scanned_name_items: 0,
    };

    cap_evidence(&mut evidence);

    assert_eq!(evidence.matched_hashes.len(), MAX_EVIDENCE_HASHES);
    assert_eq!(evidence.matched_tokens.len(), MAX_EVIDENCE_TOKENS);
    assert_eq!(evidence.matched_sections.len(), MAX_EVIDENCE_SECTIONS);
    assert_eq!(evidence.matched_hashes[0], "00000000");
    assert_eq!(evidence.matched_tokens[0], "token00");
    assert_eq!(evidence.matched_sections[0], "section00");
}

// Covers: TC-2.2-Task10-02
#[test]
fn test_direct_name_support_only_is_not_primary_evidence() {
    let mut state = ScoreState::new();
    let name_tokens = vec!["raiden".to_string(), "shogun".to_string()];
    let tag_tokens = vec!["electro".to_string()];

    apply_direct_name_support_contribution(
        &mut state,
        &name_tokens,
        &tag_tokens,
        4.0,
        2.0,
        10.0,
        6.0,
    );

    assert!(state
        .reasons
        .iter()
        .all(|reason| matches!(reason, Reason::DirectNameSupport { .. })));
    assert!(!has_primary_evidence(&state.reasons));
}
