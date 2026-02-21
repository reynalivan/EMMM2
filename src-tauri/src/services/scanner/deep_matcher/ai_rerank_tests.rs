use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{
    build_ai_cache_key, maybe_apply_ai_rerank, AiRerankCache, AiRerankConfig, AiRerankProvider,
    AiRerankRequest,
};
use crate::services::scanner::deep_matcher::content::FolderSignals;
use crate::services::scanner::deep_matcher::types::{
    Candidate, Confidence, MatchMode, MatchStatus, StagedMatchResult,
};

struct CountingProvider {
    calls: Arc<Mutex<usize>>,
    response: HashMap<usize, f32>,
}

impl CountingProvider {
    fn new(calls: Arc<Mutex<usize>>, response: HashMap<usize, f32>) -> Self {
        Self { calls, response }
    }
}

impl AiRerankProvider for CountingProvider {
    fn rerank(&self, _request: &AiRerankRequest) -> HashMap<usize, f32> {
        let mut calls = self.calls.lock().expect("lock call counter");
        *calls += 1;
        self.response.clone()
    }
}

fn review_result() -> StagedMatchResult {
    StagedMatchResult {
        status: MatchStatus::NeedsReview,
        best: Some(Candidate {
            entry_id: 0,
            name: "Alpha".to_string(),
            object_type: "Character".to_string(),
            score: 14.0,
            confidence: Confidence::Low,
            reasons: vec![],
        }),
        candidates_topk: vec![
            Candidate {
                entry_id: 0,
                name: "Alpha".to_string(),
                object_type: "Character".to_string(),
                score: 14.0,
                confidence: Confidence::Low,
                reasons: vec![],
            },
            Candidate {
                entry_id: 1,
                name: "Beta".to_string(),
                object_type: "Character".to_string(),
                score: 13.7,
                confidence: Confidence::Low,
                reasons: vec![],
            },
        ],
        evidence: Default::default(),
    }
}

fn signals(seed: &str) -> FolderSignals {
    FolderSignals {
        folder_tokens: vec![format!("folder-{seed}")],
        deep_name_tokens: vec![format!("deep-{seed}")],
        ini_section_tokens: vec![format!("section-{seed}")],
        ini_content_tokens: vec![format!("content-{seed}")],
        ini_hashes: vec![format!("hash{seed}")],
        scanned_ini_files: 1,
        scanned_name_items: 1,
        scanned_ini_bytes: 12,
    }
}

// Covers: TC-2.2-Task14-00
#[test]
fn test_ai_rerank_config_default_is_disabled() {
    let config = AiRerankConfig::default();
    assert!(!config.ai_enabled);
    assert!(config.db_version.is_none());
    assert!(config.provider.is_none());
    assert!(config.cache.is_none());
}

// Covers: TC-2.2-Task14-01
#[test]
fn test_ai_rerank_disabled_does_not_call_provider() {
    let calls = Arc::new(Mutex::new(0_usize));
    let provider = CountingProvider::new(calls.clone(), HashMap::new());

    let result = maybe_apply_ai_rerank(
        review_result(),
        &signals("disabled"),
        MatchMode::Quick,
        &AiRerankConfig {
            ai_enabled: false,
            db_version: Some("db-v1"),
            provider: Some(&provider),
            cache: None,
        },
    );

    assert_eq!(result.status, MatchStatus::NeedsReview);
    assert_eq!(*calls.lock().expect("lock call counter"), 0);
}

// Covers: TC-2.2-Task14-02
#[test]
fn test_ai_rerank_cache_key_includes_signals_hash_and_db_version() {
    let key_a = build_ai_cache_key(&signals("a"), MatchMode::FullScoring, "db-v42");
    let key_b = build_ai_cache_key(&signals("b"), MatchMode::FullScoring, "db-v42");

    assert_eq!(key_a.db_version, "db-v42");
    assert!(!key_a.signals_hash.is_empty());
    assert_ne!(key_a.signals_hash, key_b.signals_hash);
}

// Covers: TC-2.2-Task14-03
#[test]
fn test_ai_rerank_only_needs_review_path_can_invoke_provider() {
    let calls = Arc::new(Mutex::new(0_usize));
    let provider = CountingProvider::new(calls.clone(), HashMap::new());

    let auto_matched = StagedMatchResult {
        status: MatchStatus::AutoMatched,
        ..review_result()
    };
    let no_match = StagedMatchResult {
        status: MatchStatus::NoMatch,
        best: None,
        candidates_topk: vec![],
        evidence: Default::default(),
    };

    let _ = maybe_apply_ai_rerank(
        auto_matched,
        &signals("auto"),
        MatchMode::Quick,
        &AiRerankConfig {
            ai_enabled: true,
            db_version: Some("db-v2"),
            provider: Some(&provider),
            cache: None,
        },
    );
    let _ = maybe_apply_ai_rerank(
        no_match,
        &signals("none"),
        MatchMode::Quick,
        &AiRerankConfig {
            ai_enabled: true,
            db_version: Some("db-v2"),
            provider: Some(&provider),
            cache: None,
        },
    );
    let _ = maybe_apply_ai_rerank(
        review_result(),
        &signals("review"),
        MatchMode::Quick,
        &AiRerankConfig {
            ai_enabled: true,
            db_version: Some("db-v2"),
            provider: Some(&provider),
            cache: None,
        },
    );

    assert_eq!(*calls.lock().expect("lock call counter"), 1);
}

// Covers: TC-2.2-Task14-04
#[test]
fn test_ai_rerank_cache_reuses_cached_result_deterministically() {
    let calls = Arc::new(Mutex::new(0_usize));
    let provider = CountingProvider::new(
        calls.clone(),
        HashMap::from([(0_usize, 0.62_f32), (1_usize, 0.84_f32)]),
    );
    let cache = AiRerankCache::default();
    let config = AiRerankConfig {
        ai_enabled: true,
        db_version: Some("db-v3"),
        provider: Some(&provider),
        cache: Some(&cache),
    };

    let first = maybe_apply_ai_rerank(
        review_result(),
        &signals("reuse"),
        MatchMode::Quick,
        &config,
    );
    let second = maybe_apply_ai_rerank(
        review_result(),
        &signals("reuse"),
        MatchMode::Quick,
        &config,
    );

    assert_eq!(*calls.lock().expect("lock call counter"), 1);
    assert_eq!(first.status, MatchStatus::AutoMatched);
    assert_eq!(second.status, MatchStatus::AutoMatched);
    assert_eq!(
        first.best.as_ref().map(|candidate| candidate.entry_id),
        Some(1)
    );
    assert_eq!(
        second.best.as_ref().map(|candidate| candidate.entry_id),
        Some(1)
    );
}
