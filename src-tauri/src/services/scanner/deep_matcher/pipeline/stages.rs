//! Staged matcher helpers for candidate pool preparation and shared scoring stages.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::services::scanner::core::normalizer;
use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::analysis::indexes::MatcherIndexes;
use crate::services::scanner::deep_matcher::analysis::scoring::{
    apply_alias_contribution, apply_deep_token_contribution, apply_ini_token_contribution,
};
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use crate::services::scanner::deep_matcher::ScoreState;

pub const DEFAULT_SEED_CAP: usize = 200;
pub const DEFAULT_MIN_POOL: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObservedTokenBuckets {
    pub folder_tokens: BTreeSet<String>,
    pub deep_name_tokens: BTreeSet<String>,
    pub ini_section_tokens: BTreeSet<String>,
    pub ini_content_tokens: BTreeSet<String>,
}

impl ObservedTokenBuckets {
    pub fn from_signals(signals: &FolderSignals) -> Self {
        Self {
            folder_tokens: to_bucket_set(&signals.folder_tokens),
            deep_name_tokens: to_bucket_set(&signals.deep_name_tokens),
            ini_section_tokens: to_bucket_set(&signals.ini_section_tokens),
            ini_content_tokens: to_bucket_set(&signals.ini_content_tokens),
        }
    }

    pub fn observed_tokens(&self) -> BTreeSet<String> {
        self.folder_tokens
            .iter()
            .chain(self.deep_name_tokens.iter())
            .chain(self.ini_section_tokens.iter())
            .chain(self.ini_content_tokens.iter())
            .cloned()
            .collect()
    }
}

pub fn seed_candidates(
    indexes: &MatcherIndexes,
    observed_hashes: &[String],
    observed_tokens: &HashSet<String>,
    seed_cap: usize,
) -> Vec<usize> {
    if seed_cap == 0 {
        return Vec::new();
    }

    let sources = build_seed_sources(indexes, observed_hashes, observed_tokens);
    select_candidate_ids(sources, seed_cap)
}

pub fn replenish_candidates_if_needed(
    indexes: &MatcherIndexes,
    candidate_pool: &[usize],
    observed_buckets: &ObservedTokenBuckets,
    min_pool: usize,
    seed_cap: usize,
) -> Vec<usize> {
    let mut pool: BTreeSet<usize> = candidate_pool.iter().copied().collect();
    if pool.len() >= min_pool || pool.len() >= seed_cap {
        return pool.into_iter().collect();
    }

    let observed_tokens: HashSet<String> = observed_buckets.observed_tokens().into_iter().collect();
    let sources = build_token_sources(indexes, &observed_tokens);
    for source in sources {
        if pool.len() >= seed_cap {
            break;
        }

        for entry_id in source.posting {
            if pool.len() >= seed_cap {
                break;
            }
            pool.insert(*entry_id);
        }
    }

    pool.into_iter().collect()
}

#[derive(Debug)]
struct SeedSource<'a> {
    key: String,
    posting: &'a [usize],
}

fn build_seed_sources<'a>(
    indexes: &'a MatcherIndexes,
    observed_hashes: &[String],
    observed_tokens: &HashSet<String>,
) -> Vec<SeedSource<'a>> {
    let mut seen = HashSet::new();
    let mut sources = Vec::new();

    for hash in observed_hashes {
        let Some(posting) = indexes.hash_index.get(hash) else {
            continue;
        };
        if posting.is_empty() {
            continue;
        }
        if seen.insert(format!("h:{hash}")) {
            sources.push(SeedSource {
                key: hash.clone(),
                posting,
            });
        }
    }

    for token in observed_tokens {
        let Some(posting) = indexes.token_index.get(token) else {
            continue;
        };
        if posting.is_empty() {
            continue;
        }
        if seen.insert(format!("t:{token}")) {
            sources.push(SeedSource {
                key: token.clone(),
                posting,
            });
        }
    }

    sort_sources_by_rarity_and_lexical(&mut sources);
    sources
}

fn build_token_sources<'a>(
    indexes: &'a MatcherIndexes,
    observed_tokens: &HashSet<String>,
) -> Vec<SeedSource<'a>> {
    let mut sources = Vec::new();
    for token in observed_tokens {
        let Some(posting) = indexes.token_index.get(token) else {
            continue;
        };
        if posting.is_empty() {
            continue;
        }
        sources.push(SeedSource {
            key: token.clone(),
            posting,
        });
    }

    sort_sources_by_rarity_and_lexical(&mut sources);
    sources
}

fn sort_sources_by_rarity_and_lexical(sources: &mut [SeedSource<'_>]) {
    sources.sort_by(|a, b| {
        a.posting
            .len()
            .cmp(&b.posting.len())
            .then_with(|| a.key.cmp(&b.key))
    });
}

fn select_candidate_ids(sources: Vec<SeedSource<'_>>, seed_cap: usize) -> Vec<usize> {
    let mut seeded = BTreeSet::new();

    for source in sources {
        if seeded.len() >= seed_cap {
            break;
        }

        for entry_id in source.posting {
            if seeded.len() >= seed_cap {
                break;
            }
            seeded.insert(*entry_id);
        }
    }

    seeded.into_iter().collect()
}

fn to_bucket_set(tokens: &[String]) -> BTreeSet<String> {
    tokens.iter().cloned().collect()
}

// ── Shared scoring stages (used by both quick and full pipelines) ──

pub(super) fn entry_tokens(db: &MasterDb, entry_id: usize) -> &HashSet<String> {
    &db.keywords[entry_id].1
}

pub(super) fn apply_alias_stage(
    db: &MasterDb,
    folder_tokens: &BTreeSet<String>,
    states: &mut HashMap<usize, ScoreState>,
) {
    for (entry_id, state) in states.iter_mut() {
        let entry = &db.entries[*entry_id];
        for skin in &entry.custom_skins {
            if let Some(alias) = skin.aliases.iter().find(|alias| {
                let alias_tokens = normalizer::preprocess_text(alias);
                !alias_tokens.is_empty()
                    && alias_tokens
                        .iter()
                        .all(|token| folder_tokens.contains(token))
            }) {
                apply_alias_contribution(state, alias, 12.0);
                break;
            }
        }
    }
}

pub(super) fn apply_deep_stage(
    db: &MasterDb,
    buckets: &ObservedTokenBuckets,
    states: &mut HashMap<usize, ScoreState>,
) {
    for (entry_id, state) in states.iter_mut() {
        let et = entry_tokens(db, *entry_id);

        let deep_hits: Vec<String> = buckets
            .deep_name_tokens
            .iter()
            .filter(|token| et.contains(*token))
            .cloned()
            .collect();
        let deep_ratio = (deep_hits.len() as f32) / (buckets.deep_name_tokens.len().max(1) as f32);
        apply_deep_token_contribution(state, &deep_hits, deep_ratio, 16.0, 1.0, 6.0);

        let section_hits: Vec<String> = buckets
            .ini_section_tokens
            .iter()
            .filter(|token| et.contains(*token))
            .cloned()
            .collect();
        let content_hits: Vec<String> = buckets
            .ini_content_tokens
            .iter()
            .filter(|token| et.contains(*token))
            .cloned()
            .collect();
        let ini_denominator =
            (buckets.ini_section_tokens.len() + buckets.ini_content_tokens.len()).max(1);
        let ini_ratio =
            ((section_hits.len() + content_hits.len()) as f32) / (ini_denominator as f32);
        apply_ini_token_contribution(state, &section_hits, &content_hits, ini_ratio, 8.0);
    }
}

#[cfg(test)]
#[path = "../tests/pipeline/stages_tests.rs"]
mod tests;
