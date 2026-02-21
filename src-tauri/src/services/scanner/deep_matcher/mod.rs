//! Deep Matcher Pipeline — The "Brain" of EMMM2.
//!
//! Identifies mod categories by running the legacy L0-L5 pipeline while staged
//! refactor modules are integrated incrementally.
// Module structure
pub mod acceptance;
pub mod ai_rerank;
pub mod content;
pub mod full_pipeline;
pub mod indexes;
pub mod legacy;
pub mod quick_pipeline;
mod quick_pipeline_result;
mod result_summary;
pub mod scoring;
pub mod stages;
pub mod types;

// Test-only modules
#[cfg(test)]
pub mod golden_corpus;
#[cfg(test)]
mod required_tests;

// Re-export legacy types for backward compatibility
pub use types::{Confidence, CustomSkin, DbEntry, MatchLevel, MatchResult};

// Re-export legacy functions for backward compatibility
pub use full_pipeline::match_folder_full;
pub use indexes::{idf_lite, MatcherIndexes};
pub use legacy::{fuzzy_match, match_folder, name_match, skin_alias_match, token_match};
pub use quick_pipeline::match_folder_quick;
pub use scoring::{
    apply_alias_contribution, apply_deep_token_contribution,
    apply_direct_name_support_contribution, apply_hash_contribution, apply_ini_token_contribution,
    apply_token_overlap_contribution, cap_evidence, has_primary_evidence,
};

// Re-export new staged types (non-breaking addition)
pub use types::{
    sort_candidates_deterministic, Candidate, Evidence, MatchMode, MatchStatus, Reason, ScoreState,
    StagedMatchResult, MAX_EVIDENCE_HASHES, MAX_EVIDENCE_SECTIONS, MAX_EVIDENCE_TOKENS,
    MAX_REASONS_PER_CANDIDATE,
};

use std::collections::HashSet;

use super::normalizer;

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

    /// Load from JSON string (deserialized as Vec<DbEntry>).
    pub fn from_json(json: &str) -> Result<Self, String> {
        let entries: Vec<DbEntry> = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse MasterDb JSON: {}", e))?;
        Ok(Self::new(entries))
    }

    pub fn token_idf(&self, token: &str) -> f32 {
        self.indexes.token_idf(token, self.entries.len())
    }

    pub fn hash_idf(&self, hash: &str) -> f32 {
        self.indexes.hash_idf(hash, self.entries.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::scanner::walker::{FileInfo, FolderContent, ModCandidate};
    use std::path::PathBuf;

    fn test_db() -> MasterDb {
        MasterDb::new(vec![
            DbEntry {
                name: "Raiden Shogun".to_string(),
                tags: vec!["Raiden".to_string(), "Ei".to_string()],
                object_type: "Character".to_string(),
                custom_skins: vec![CustomSkin {
                    name: "Wish".to_string(),
                    aliases: vec!["RaidenWish".to_string(), "Raiden2".to_string()],
                    thumbnail_skin_path: None,
                    rarity: None,
                }],
                thumbnail_path: None,
                metadata: None,
                hashes: vec![],
            },
            DbEntry {
                name: "Albedo".to_string(),
                tags: vec![],
                object_type: "Character".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
                hashes: vec![],
            },
            DbEntry {
                name: "Lumine".to_string(),
                tags: vec!["Traveler".to_string()],
                object_type: "Character".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
                hashes: vec![],
            },
            DbEntry {
                name: "Jean".to_string(),
                tags: vec!["Jean Gunnhildr".to_string()],
                object_type: "Character".to_string(),
                custom_skins: vec![
                    CustomSkin {
                        name: "Gunnhildr's Heritage".to_string(),
                        aliases: vec!["JeanCN".to_string(), "Jean2".to_string()],
                        thumbnail_skin_path: Some(
                            "assets/thumbnails/gimi/skin/jean_cn.png".to_string(),
                        ),
                        rarity: Some("5".to_string()),
                    },
                    CustomSkin {
                        name: "Sea Breeze Dandelion".to_string(),
                        aliases: vec!["JeanSea".to_string(), "Jean3".to_string()],
                        thumbnail_skin_path: None,
                        rarity: Some("5".to_string()),
                    },
                ],
                thumbnail_path: Some("assets/thumbnails/gimi/char/jean.png".to_string()),
                metadata: None,
                hashes: vec![],
            },
            DbEntry {
                name: "Primordial Jade Winged-Spear".to_string(),
                tags: vec!["PJWS".to_string()],
                object_type: "Weapon".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
                hashes: vec![],
            },
        ])
    }

    fn empty_content() -> FolderContent {
        FolderContent {
            subfolder_names: vec![],
            files: vec![],
            ini_files: vec![],
        }
    }

    fn candidate(name: &str) -> ModCandidate {
        ModCandidate {
            path: PathBuf::from(format!("/mods/{name}")),
            raw_name: name.to_string(),
            display_name: name.to_string(),
            is_disabled: false,
        }
    }

    // Covers: TC-2.2-01 — Name Match (Exact)
    #[test]
    fn test_l1_name_match_exact() {
        let db = test_db();
        let c = candidate("[Mod] Raiden Shogun");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Raiden Shogun");
        assert_eq!(result.level, MatchLevel::L1Name);
        assert_eq!(result.confidence, Confidence::High);
    }

    #[test]
    fn test_l1_name_match_tag() {
        let db = test_db();
        let c = candidate("[Mod] The Great Ei");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Raiden Shogun");
        assert_eq!(result.level, MatchLevel::L1Name);
    }

    #[test]
    fn test_l2_token_match() {
        let db = test_db();
        let c = candidate("[Mod] PJWS Recolor");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Primordial Jade Winged-Spear");
        assert_eq!(result.level, MatchLevel::L2Token);
    }

    // Covers: TC-2.2-02 — Content Scan (Subfolder Name Match)
    #[test]
    fn test_l3_content_match_subfolder() {
        let db = test_db();
        let c = candidate("[Unknown] Mystery Mod");
        let content = FolderContent {
            subfolder_names: vec!["AlbedoBuffer".to_string()],
            files: vec![],
            ini_files: vec![],
        };
        let result = match_folder(&c, &db, &content);

        assert_eq!(result.object_name, "Albedo");
        assert_eq!(result.level, MatchLevel::L3Content);
        assert_eq!(result.confidence, Confidence::High);
    }

    #[test]
    fn test_l3_content_match_file() {
        let db = test_db();
        let c = candidate("[Unknown] Mystery Mod");
        let content = FolderContent {
            subfolder_names: vec![],
            files: vec![FileInfo {
                name: "Lumine_Head_IB.buf".to_string(),
                path: PathBuf::from("Lumine_Head_IB.buf"),
                extension: "buf".to_string(),
            }],
            ini_files: vec![],
        };
        let result = match_folder(&c, &db, &content);

        assert_eq!(result.object_name, "Lumine");
        assert_eq!(result.level, MatchLevel::L3Content);
    }

    // Covers: TC-2.2-03 — Fuzzy Match (Low Confidence)
    #[test]
    fn test_l5_fuzzy_match() {
        let db = test_db();
        let c = candidate("[Mod] Albato");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Albedo");
        assert_eq!(result.level, MatchLevel::L5Fuzzy);
        assert!(matches!(
            result.confidence,
            Confidence::Medium | Confidence::Low
        ));
    }

    #[test]
    fn test_unmatched() {
        let db = test_db();
        let c = candidate("[Mod] CompletelyUnknown");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.level, MatchLevel::Unmatched);
    }

    #[test]
    fn test_skin_alias_match() {
        let db = test_db();
        let c = candidate("[Mod] JeanCN");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Jean");
        assert_eq!(
            result.detected_skin,
            Some("Gunnhildr's Heritage".to_string())
        );
        assert_eq!(result.skin_folder_name, Some("JeanCN".to_string()));
    }

    #[test]
    fn test_skin_detection_via_name_match() {
        let db = test_db();
        let c = candidate("[Mod] Jean Sea Breeze");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Jean");
        assert_eq!(
            result.detected_skin,
            Some("Sea Breeze Dandelion".to_string())
        );
    }

    #[test]
    fn test_skin_detection_via_raiden_wish() {
        let db = test_db();
        let c = candidate("[Mod] Raiden Wish");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Raiden Shogun");
        assert_eq!(result.detected_skin, Some("Wish".to_string()));
        assert_eq!(result.skin_folder_name, Some("RaidenWish".to_string()));
    }

    #[test]
    fn test_skin_detection_via_content_subfolder() {
        let db = test_db();
        let c = candidate("[Unknown] Mystery");
        let content = FolderContent {
            subfolder_names: vec!["JeanSea".to_string()],
            files: vec![],
            ini_files: vec![],
        };
        let result = match_folder(&c, &db, &content);

        assert_eq!(result.object_name, "Jean");
        assert_eq!(
            result.detected_skin,
            Some("Sea Breeze Dandelion".to_string())
        );
    }

    #[test]
    fn test_masterdb_from_json() {
        let json = r#"[{"name":"Test","tags":[],"object_type":"Character","custom_skins":[],"thumbnail_path":null,"metadata":null}]"#;
        let db = MasterDb::from_json(json).unwrap();
        assert_eq!(db.entries.len(), 1);
        assert_eq!(db.entries[0].name, "Test");
    }

    #[test]
    fn test_keywords_precomputation() {
        let db = test_db();
        assert_eq!(db.keywords.len(), db.entries.len());
        let raiden_tokens = &db.keywords[0].1;
        assert!(raiden_tokens.contains("raiden"));
        assert!(raiden_tokens.contains("shogun"));
    }
}
