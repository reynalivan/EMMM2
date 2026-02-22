use std::collections::HashSet;

use crate::services::scanner::core::normalizer;
use crate::services::scanner::deep_matcher::analysis::indexes::MatcherIndexes;
use crate::services::scanner::deep_matcher::models::types::DbEntry;

/// The Master DB containing all known objects for matching.
#[derive(Debug, Clone)]
pub struct MasterDb {
    pub entries: Vec<DbEntry>,
    /// Pre-computed: for each entry, the combined set of name + tags tokens.
    pub(crate) keywords: Vec<(usize, HashSet<String>)>,
    /// Deterministic indexes and document-frequency maps for staged matcher.
    pub(crate) indexes: MatcherIndexes,
}

impl MasterDb {
    /// Build a MasterDb from raw entries, pre-computing keyword sets.
    pub fn new(entries: Vec<DbEntry>) -> Self {
        let keywords: Vec<(usize, HashSet<String>)> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let mut tokens = normalizer::preprocess_text(&entry.name);
                for tag in &entry.tags {
                    tokens.extend(normalizer::preprocess_text(tag));
                }
                (i, tokens)
            })
            .collect();

        let indexes = MatcherIndexes::build(&entries, &keywords);

        Self {
            entries,
            keywords,
            indexes,
        }
    }

    /// Load from JSON string.
    /// Supports both legacy array format `[{entry1}, {entry2}]`
    /// and new object format `{"entries": [...], "hash_db": {...}}`.
    /// When hash_db is present, merges hashes into matching entries by name.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse MasterDb JSON: {}", e))?;

        let (mut entries, hash_db) = match value {
            // New object format: {"entries": [...], "hash_db": {...}}
            serde_json::Value::Object(ref map) if map.contains_key("entries") => {
                let entries: Vec<DbEntry> = serde_json::from_value(map["entries"].clone())
                    .map_err(|e| format!("Failed to parse entries: {}", e))?;
                let hash_db: std::collections::HashMap<String, Vec<String>> =
                    serde_json::from_value(map.get("hash_db").cloned().unwrap_or_default())
                        .unwrap_or_default();
                (entries, hash_db)
            }
            // Legacy array format: [{entry1}, {entry2}]
            serde_json::Value::Array(_) => {
                let entries: Vec<DbEntry> = serde_json::from_value(value)
                    .map_err(|e| format!("Failed to parse entries array: {}", e))?;
                (entries, std::collections::HashMap::new())
            }
            _ => {
                return Err(
                    "Invalid MasterDb format: expected array or object with 'entries' key"
                        .to_string(),
                )
            }
        };

        // Merge hash_db into matching entries
        if !hash_db.is_empty() {
            for entry in &mut entries {
                if let Some(hashes) = hash_db.get(&entry.name) {
                    entry
                        .hash_db
                        .entry("Default".to_string())
                        .or_default()
                        .extend(hashes.iter().cloned());
                }
            }
        }

        Ok(Self::new(entries))
    }

    pub fn token_idf(&self, token: &str) -> f32 {
        self.indexes.token_idf(token, self.entries.len())
    }

    pub fn hash_idf(&self, hash: &str) -> f32 {
        self.indexes.hash_idf(hash, self.entries.len())
    }
}
