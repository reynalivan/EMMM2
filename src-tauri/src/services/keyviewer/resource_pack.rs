//! Thin adapter extracting hash data from existing `MasterDb` entries.
//!
//! The MasterDb JSON files (`gimi.json`, `srmi.json`, etc.) already contain
//! `hash_db` per entry with skin-keyed hash arrays. This module provides a
//! flattened view (`KvObjectEntry`) suitable for the KeyViewer matching pipeline.

use std::collections::HashMap;

use crate::services::scanner::deep_matcher::state::master_db::MasterDb;

/// A flattened view of one MasterDb entry for KeyViewer matching.
#[derive(Debug, Clone)]
pub struct KvObjectEntry {
    /// Display name (e.g. "Albedo").
    pub name: String,
    /// Object type: "Character", "Weapon", "UI", "Other".
    pub object_type: String,
    /// All hashes from `hash_db`, flattened across all skins. Deduplicated, lowercase.
    pub code_hashes: Vec<String>,
    /// Skin/variant name → associated hashes (original structure from `hash_db`).
    pub skin_hashes: HashMap<String, Vec<String>>,
    /// Search tags from the MasterDb entry.
    pub tags: Vec<String>,
    /// Optional thumbnail path (relative to resources dir).
    pub thumbnail_path: Option<String>,
}

/// Extract all entries with non-empty `hash_db` from a MasterDb as KeyViewer objects.
///
/// Entries with empty `hash_db` are skipped since they cannot be matched at runtime.
pub fn extract_kv_entries(db: &MasterDb) -> Vec<KvObjectEntry> {
    db.entries
        .iter()
        .filter(|entry| !entry.hash_db.is_empty())
        .map(|entry| {
            let skin_hashes: HashMap<String, Vec<String>> = entry
                .hash_db
                .iter()
                .map(|(skin, hashes)| {
                    let normalized: Vec<String> =
                        hashes.iter().map(|h| h.to_ascii_lowercase()).collect();
                    (skin.clone(), normalized)
                })
                .collect();

            // Flatten all hashes, dedup
            let mut all_hashes: Vec<String> = skin_hashes
                .values()
                .flat_map(|v| v.iter().cloned())
                .collect();
            all_hashes.sort();
            all_hashes.dedup();

            KvObjectEntry {
                name: entry.name.clone(),
                object_type: entry.object_type.clone(),
                code_hashes: all_hashes,
                skin_hashes,
                tags: entry.tags.clone(),
                thumbnail_path: entry.thumbnail_path.clone(),
            }
        })
        .collect()
}
