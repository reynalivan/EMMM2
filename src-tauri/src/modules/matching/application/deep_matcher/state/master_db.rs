use std::collections::HashSet;

use crate::modules::matching::application::deep_matcher::analysis::indexes::MatcherIndexes;
use crate::modules::matching::application::deep_matcher::models::types::DbEntry;
use crate::modules::workspace::domain::normalizer;
use crate::shared::errors::ScannerError;

/// The Master DB containing all known objects for matching.
#[derive(Debug, Clone)]
pub struct MasterDb {
    pub entries: Vec<DbEntry>,
    /// Pre-computed: for each entry, the combined set of name + tags tokens.
    pub(crate) keywords: Vec<(usize, HashSet<String>)>,
    /// Pre-computed separately so direct-name support does not normalize every
    /// candidate's name and aliases during matching.
    pub(crate) direct_name_tokens: Vec<HashSet<String>>,
    pub(crate) direct_alias_tokens: Vec<HashSet<String>>,
    /// Deterministic indexes and document-frequency maps for staged matcher.
    pub(crate) indexes: MatcherIndexes,
}

impl MasterDb {
    /// Build a MasterDb from raw entries, pre-computing keyword sets.
    pub fn new(entries: Vec<DbEntry>) -> Self {
        let direct_name_tokens: Vec<HashSet<String>> = entries
            .iter()
            .map(|entry| normalizer::preprocess_text(&entry.name))
            .collect();
        let direct_alias_tokens: Vec<HashSet<String>> = entries
            .iter()
            .map(|entry| {
                entry
                    .aliases
                    .iter()
                    .flat_map(|alias| normalizer::preprocess_text(alias))
                    .collect()
            })
            .collect();
        let keywords: Vec<(usize, HashSet<String>)> = entries
            .iter()
            .zip(&direct_name_tokens)
            .zip(&direct_alias_tokens)
            .enumerate()
            .map(|(index, ((entry, name_tokens), alias_tokens))| {
                let tag_tokens = catalog_tags(entry)
                    .into_iter()
                    .flat_map(|tag| normalizer::preprocess_text(tag))
                    .collect::<HashSet<_>>();
                let keywords = name_tokens
                    .union(alias_tokens)
                    .cloned()
                    .collect::<HashSet<_>>();
                (index, keywords.union(&tag_tokens).cloned().collect())
            })
            .collect();

        let indexes = MatcherIndexes::build(&entries, &keywords);

        Self {
            entries,
            keywords,
            direct_name_tokens,
            direct_alias_tokens,
            indexes,
        }
    }

    /// Load from JSON string.
    /// Supports new object format `{"entries": [...], "hash_db": {...}}`.
    /// When hash_db is present, merges hashes into matching entries by name.
    pub fn from_json(json: &str) -> Result<Self, ScannerError> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        let (mut entries, hash_db) = match value {
            serde_json::Value::Object(ref map) if map.contains_key("entries") => {
                let mut entries_json = map["entries"].clone();
                preserve_catalog_tags(&mut entries_json);
                let entries: Vec<DbEntry> = serde_json::from_value(entries_json)?;
                let hash_db: std::collections::HashMap<String, Vec<String>> =
                    serde_json::from_value(map.get("hash_db").cloned().unwrap_or_default())
                        .unwrap_or_default();
                (entries, hash_db)
            }
            _ => {
                return Err(ScannerError::Parse {
                    what: "MasterDB".to_string(),
                    detail: "expected an object with an 'entries' key".to_string(),
                });
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

fn preserve_catalog_tags(entries: &mut serde_json::Value) {
    let Some(entries) = entries.as_array_mut() else {
        return;
    };
    for entry in entries {
        let Some(object) = entry.as_object_mut() else {
            continue;
        };
        let Some(tags) = object.remove("tags") else {
            continue;
        };
        let Some(tags) = tags.as_array() else {
            continue;
        };
        let tags = tags
            .iter()
            .filter_map(serde_json::Value::as_str)
            .filter(|tag| !tag.trim().is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if tags.is_empty() {
            continue;
        }
        let metadata = object
            .entry("metadata")
            .or_insert_with(|| serde_json::json!({}));
        let Some(metadata) = metadata.as_object_mut() else {
            continue;
        };
        metadata.insert("catalog_tags".to_string(), serde_json::json!(tags));
    }
}

fn catalog_tags(entry: &DbEntry) -> Vec<&str> {
    entry
        .metadata
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .and_then(|metadata| metadata.get("catalog_tags"))
        .and_then(serde_json::Value::as_array)
        .map(|tags| tags.iter().filter_map(serde_json::Value::as_str).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::MasterDb;
    use crate::modules::workspace::domain::normalizer;

    #[test]
    fn direct_support_tokens_match_runtime_normalization() {
        let entries = serde_json::from_value(serde_json::json!([
            {"name": "Raiden Shogun", "aliases": ["Electro Archon", "Ei"]}
        ]))
        .expect("fixture entries should deserialize");
        let db = MasterDb::new(entries);

        assert_eq!(
            db.direct_name_tokens[0],
            normalizer::preprocess_text("Raiden Shogun")
        );
        assert_eq!(
            db.direct_alias_tokens[0],
            normalizer::preprocess_text("Electro Archon")
                .union(&normalizer::preprocess_text("Ei"))
                .cloned()
                .collect()
        );
    }

    #[test]
    fn catalog_tags_contribute_keywords_without_becoming_direct_aliases() {
        let db =
            MasterDb::from_json(r#"{"entries":[{"name":"March 7th","tags":["Astral Express"]}]}"#)
                .expect("catalog tags should parse");

        assert!(db.direct_alias_tokens[0].is_empty());
        assert!(db.keywords[0].1.contains("astral"));
        assert!(db.keywords[0].1.contains("express"));
    }
}
