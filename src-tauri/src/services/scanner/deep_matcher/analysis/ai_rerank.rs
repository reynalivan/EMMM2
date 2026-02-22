use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::analysis::scoring::cap_reasons;
use crate::services::scanner::deep_matcher::{
    sort_candidates_deterministic, Confidence, MatchMode, MatchStatus, Reason, StagedMatchResult,
};

pub const AI_ACCEPT_THRESHOLD: f32 = 0.7;
pub const AI_ACCEPT_MARGIN: f32 = 0.15;
const DEFAULT_DB_VERSION: &str = "db-version-unknown";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AiRerankCacheKey {
    pub signals_hash: String,
    pub db_version: String,
}

#[derive(Debug, Clone)]
pub struct AiRerankRequest {
    pub mode: MatchMode,
    pub cache_key: AiRerankCacheKey,
    pub candidate_entry_ids: Vec<usize>,
}

pub trait AiRerankProvider: Send + Sync {
    fn rerank(
        &self,
        request: &AiRerankRequest,
        signals: &FolderSignals,
        db: &crate::services::scanner::deep_matcher::MasterDb,
    ) -> Result<HashMap<usize, f32>, String>;
}

#[derive(Debug, Default)]
pub struct AiRerankCache {
    entries: Mutex<HashMap<AiRerankCacheKey, HashMap<usize, f32>>>,
}

impl AiRerankCache {
    pub fn get(&self, key: &AiRerankCacheKey) -> Option<HashMap<usize, f32>> {
        self.entries.lock().unwrap().get(key).cloned()
    }

    pub fn insert(&self, key: AiRerankCacheKey, scores: HashMap<usize, f32>) {
        self.entries.lock().unwrap().insert(key, scores);
    }
}

#[derive(Default)]
pub struct AiRerankConfig<'a> {
    pub ai_enabled: bool,
    pub db_version: Option<&'a str>,
    pub provider: Option<&'a dyn AiRerankProvider>,
    pub cache: Option<&'a AiRerankCache>,
}

pub fn build_ai_cache_key(
    signals: &FolderSignals,
    mode: MatchMode,
    db_version: &str,
) -> AiRerankCacheKey {
    let mut digest = blake3::Hasher::new();
    digest.update(mode.to_string().as_bytes());
    digest.update(&signals.scanned_ini_files.to_le_bytes());
    digest.update(&signals.scanned_name_items.to_le_bytes());
    digest.update(&signals.scanned_ini_bytes.to_le_bytes());
    update_string_vec(&mut digest, b"folder", &signals.folder_tokens);
    update_string_vec(&mut digest, b"deep", &signals.deep_name_tokens);
    update_string_vec(&mut digest, b"section", &signals.ini_section_tokens);
    update_string_vec(&mut digest, b"content", &signals.ini_content_tokens);
    update_string_vec(&mut digest, b"hash", &signals.ini_hashes);

    AiRerankCacheKey {
        signals_hash: digest.finalize().to_hex().to_string(),
        db_version: db_version.to_string(),
    }
}

pub fn maybe_apply_ai_rerank(
    result: StagedMatchResult,
    signals: &FolderSignals,
    db: &crate::services::scanner::deep_matcher::MasterDb,
    mode: MatchMode,
    config: &AiRerankConfig<'_>,
) -> StagedMatchResult {
    if !config.ai_enabled || result.status != MatchStatus::NeedsReview {
        return result;
    }

    let Some(provider) = config.provider else {
        return result;
    };

    let db_version = config
        .db_version
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_DB_VERSION);
    let cache_key = build_ai_cache_key(signals, mode, db_version);
    let request = AiRerankRequest {
        mode,
        cache_key: cache_key.clone(),
        candidate_entry_ids: result
            .candidates_topk
            .iter()
            .map(|candidate| candidate.entry_id)
            .collect(),
    };

    let ai_scores = resolve_ai_scores(config.cache, provider, &request, signals, db);
    let Some((best_entry_id, best_ai_score, second_ai_score)) =
        evaluate_ai_acceptance(&result, &ai_scores)
    else {
        return result;
    };

    if best_ai_score < AI_ACCEPT_THRESHOLD {
        return result;
    }
    if (best_ai_score - second_ai_score) < AI_ACCEPT_MARGIN {
        return result;
    }

    promote_to_auto_matched(result, best_entry_id, &ai_scores)
}

fn update_string_vec(hasher: &mut blake3::Hasher, label: &[u8], values: &[String]) {
    hasher.update(label);
    for value in values {
        let bytes = value.as_bytes();
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
}

fn resolve_ai_scores(
    cache: Option<&AiRerankCache>,
    provider: &dyn AiRerankProvider,
    request: &AiRerankRequest,
    signals: &FolderSignals,
    db: &crate::services::scanner::deep_matcher::MasterDb,
) -> HashMap<usize, f32> {
    if let Some(cache_ref) = cache {
        if let Some(cached) = cache_ref.get(&request.cache_key) {
            return cached;
        }
    }

    let scores: HashMap<usize, f32> = match provider.rerank(request, signals, db) {
        Ok(res) => res
            .into_iter()
            .map(|(id, score)| (id, score.clamp(0.0, 1.0)))
            .collect(),
        Err(e) => {
            log::error!("AI Rerank failed: {}", e);
            HashMap::new()
        }
    };

    if let Some(cache_ref) = cache {
        cache_ref.insert(request.cache_key.clone(), scores.clone());
    }

    scores
}

fn evaluate_ai_acceptance(
    result: &StagedMatchResult,
    ai_scores: &HashMap<usize, f32>,
) -> Option<(usize, f32, f32)> {
    if result.candidates_topk.is_empty() {
        return None;
    }

    let mut ranked: Vec<(usize, &str, f32)> = result
        .candidates_topk
        .iter()
        .map(|candidate| {
            (
                candidate.entry_id,
                candidate.name.as_str(),
                ai_scores.get(&candidate.entry_id).copied().unwrap_or(0.0),
            )
        })
        .collect();

    ranked.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.1.cmp(right.1))
            .then_with(|| left.0.cmp(&right.0))
    });

    let (best_entry_id, _, best_ai_score) = ranked.first().copied()?;
    let second_ai_score = ranked.get(1).map(|entry| entry.2).unwrap_or(0.0);
    Some((best_entry_id, best_ai_score, second_ai_score))
}

fn promote_to_auto_matched(
    mut result: StagedMatchResult,
    best_entry_id: usize,
    ai_scores: &HashMap<usize, f32>,
) -> StagedMatchResult {
    for candidate in &mut result.candidates_topk {
        let ai_score = ai_scores.get(&candidate.entry_id).copied().unwrap_or(0.0);
        candidate.score = (ai_score * 100.0).clamp(0.0, 100.0);

        if candidate.entry_id != best_entry_id {
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
        .find(|candidate| candidate.entry_id == best_entry_id)
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

#[cfg(test)]
#[path = "../tests/analysis/ai_rerank_tests.rs"]
mod ai_rerank_tests;
