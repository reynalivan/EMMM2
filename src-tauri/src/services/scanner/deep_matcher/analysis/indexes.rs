//! Deterministic token/hash indexes for staged matcher seeding and scoring.

use std::collections::{BTreeMap, HashSet};

use crate::services::scanner::deep_matcher::DbEntry;

/// Deterministic posting list keyed by normalized token/hash.
pub type PostingList = Vec<usize>;

/// Precomputed lookup structures used by staged matcher phases.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MatcherIndexes {
    pub token_index: BTreeMap<String, PostingList>,
    pub hash_index: BTreeMap<String, PostingList>,
    pub token_df: BTreeMap<String, usize>,
    pub hash_df: BTreeMap<String, usize>,
}

impl MatcherIndexes {
    /// Build deterministic token/hash indexes and document-frequency maps.
    pub fn build(entries: &[DbEntry], keywords: &[(usize, HashSet<String>)]) -> Self {
        let mut token_index: BTreeMap<String, PostingList> = BTreeMap::new();
        for (entry_id, tokens) in keywords {
            for token in tokens {
                if token.is_empty() {
                    continue;
                }
                token_index
                    .entry(token.clone())
                    .or_default()
                    .push(*entry_id);
            }
        }

        let mut hash_index: BTreeMap<String, PostingList> = BTreeMap::new();
        for (entry_id, entry) in entries.iter().enumerate() {
            for hash_list in entry.hash_db.values() {
                for raw_hash in hash_list {
                    let Some(hash) = normalize_hash(raw_hash) else {
                        continue;
                    };
                    hash_index.entry(hash).or_default().push(entry_id);
                }
            }
        }

        finalize_postings(&mut token_index);
        finalize_postings(&mut hash_index);

        let token_df = build_df(&token_index);
        let hash_df = build_df(&hash_index);

        Self {
            token_index,
            hash_index,
            token_df,
            hash_df,
        }
    }

    /// IDF-lite helper: ln((N+1)/(df+1)) + 1.
    pub fn token_idf(&self, token: &str, total_entries: usize) -> f32 {
        idf_lite(
            total_entries,
            self.token_df.get(token).copied().unwrap_or(0),
        )
    }

    /// IDF-lite helper for hash keys.
    pub fn hash_idf(&self, hash: &str, total_entries: usize) -> f32 {
        idf_lite(total_entries, self.hash_df.get(hash).copied().unwrap_or(0))
    }
}

/// Compute IDF-lite weight from corpus size and document frequency.
pub fn idf_lite(total_entries: usize, document_frequency: usize) -> f32 {
    let n = total_entries as f32;
    let df = document_frequency as f32;
    ((n + 1.0) / (df + 1.0)).ln() + 1.0
}

fn build_df(index: &BTreeMap<String, PostingList>) -> BTreeMap<String, usize> {
    index
        .iter()
        .map(|(token, posting)| (token.clone(), posting.len()))
        .collect()
}

fn finalize_postings(index: &mut BTreeMap<String, PostingList>) {
    for posting in index.values_mut() {
        posting.sort_unstable();
        posting.dedup();
    }
}

fn normalize_hash(raw: &str) -> Option<String> {
    let mut hash = raw.trim().to_lowercase();
    if let Some(stripped) = hash.strip_prefix("0x") {
        hash = stripped.to_string();
    }
    if hash.len() == 16 {
        hash = hash[8..].to_string();
    }
    if hash.len() != 8 || !hash.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    Some(hash)
}

#[cfg(test)]
#[path = "../tests/analysis/indexes_tests.rs"]
mod tests;
