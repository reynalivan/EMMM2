//! Staged matcher helpers for candidate pool preparation.

use std::collections::{BTreeSet, HashSet};

use super::content::FolderSignals;
use super::indexes::MatcherIndexes;

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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::services::scanner::deep_matcher::CustomSkin;

    use super::*;
    use crate::services::scanner::deep_matcher::indexes::MatcherIndexes;
    use crate::services::scanner::deep_matcher::types::DbEntry;
    use crate::services::scanner::normalizer;

    fn sample_indexes() -> MatcherIndexes {
        let entries = vec![
            DbEntry {
                name: "Raiden Shogun".to_string(),
                tags: vec!["electro".to_string()],
                object_type: "Character".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
                hashes: vec!["aaaaaaaa".to_string()],
            },
            DbEntry {
                name: "Jean".to_string(),
                tags: vec!["dandelion".to_string()],
                object_type: "Character".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
                hashes: vec!["aaaaaaaa".to_string()],
            },
            DbEntry {
                name: "Albedo".to_string(),
                tags: vec!["chalk".to_string()],
                object_type: "Character".to_string(),
                custom_skins: vec![CustomSkin {
                    name: "Default".to_string(),
                    aliases: vec![],
                    thumbnail_skin_path: None,
                    rarity: None,
                }],
                thumbnail_path: None,
                metadata: None,
                hashes: vec!["bbbbbbbb".to_string()],
            },
            DbEntry {
                name: "Diluc".to_string(),
                tags: vec![],
                object_type: "Character".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
                hashes: vec![],
            },
        ];

        let keywords: Vec<(usize, HashSet<String>)> = entries
            .iter()
            .enumerate()
            .map(|(entry_id, entry)| {
                let mut tokens = normalizer::preprocess_text(&entry.name);
                for tag in &entry.tags {
                    tokens.extend(normalizer::preprocess_text(tag));
                }
                (entry_id, tokens)
            })
            .collect();

        MatcherIndexes::build(&entries, &keywords)
    }

    // Covers: TC-2.2-Task9-01
    #[test]
    fn test_seed_candidates_rarity_first_with_seed_cap() {
        let indexes = sample_indexes();
        let observed_hashes = vec!["aaaaaaaa".to_string(), "bbbbbbbb".to_string()];
        let observed_tokens: HashSet<String> = ["chalk", "raiden", "electro"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let seeded = seed_candidates(&indexes, &observed_hashes, &observed_tokens, 2);

        // bbbbbbbb and chalk both map only to Albedo (entry 2), then rare electro maps to Raiden.
        assert_eq!(seeded, vec![0, 2]);
    }

    // Covers: TC-2.2-Task9-02
    #[test]
    fn test_seed_candidates_empty_signals_safe() {
        let indexes = sample_indexes();
        let seeded = seed_candidates(&indexes, &[], &HashSet::new(), 200);
        assert!(seeded.is_empty());
    }

    #[test]
    fn test_replenish_candidates_when_below_min_pool() {
        let indexes = sample_indexes();
        let buckets = ObservedTokenBuckets {
            folder_tokens: ["electro".to_string(), "chalk".to_string()]
                .into_iter()
                .collect(),
            deep_name_tokens: ["raiden".to_string()].into_iter().collect(),
            ini_section_tokens: BTreeSet::new(),
            ini_content_tokens: BTreeSet::new(),
        };

        let replenished = replenish_candidates_if_needed(&indexes, &[2], &buckets, 3, 4);
        assert_eq!(replenished, vec![0, 2]);
    }
}
