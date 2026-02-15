//! Deep Matcher Pipeline — The "Brain" of EMMM2.
//!
//! Identifies mod categories (Character, Weapon, UI, etc.) by running
//! a priority pipeline: L1 Name → L2 Token → L3 Content → L4 AI → L5 Fuzzy.
//!
//! # Covers: US-2.2 (The Deep Matcher Pipeline)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::normalizer;
use super::walker::{FolderContent, ModCandidate};

/// Matching confidence level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
    None,
}

/// Which pipeline level produced the match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchLevel {
    L1Name,
    L2Token,
    L3Content,
    L4Ai,
    L5Fuzzy,
    Unmatched,
}

/// Result of the matching pipeline for a single mod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub object_name: String,
    pub object_type: String,
    pub level: MatchLevel,
    pub confidence: Confidence,
    pub detail: String,
    pub detected_skin: Option<String>,
}

impl MatchResult {
    /// Create an "unmatched" result.
    pub fn unmatched() -> Self {
        Self {
            object_name: String::new(),
            object_type: "Uncategorized".to_string(),
            level: MatchLevel::Unmatched,
            confidence: Confidence::None,
            detail: "No match found".to_string(),
            detected_skin: None,
        }
    }
}

/// A single entry from the Master DB (db_char.json / db_other.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbEntry {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub object_type: String,
    #[serde(default)]
    pub official_skins: Vec<String>,
}

/// The Master DB containing all known objects for matching.
#[derive(Debug, Clone)]
pub struct MasterDb {
    pub entries: Vec<DbEntry>,
    /// Pre-computed: for each entry, the combined set of name + tags tokens.
    keywords: Vec<(usize, HashSet<String>)>,
}

impl MasterDb {
    /// Build a MasterDb from raw entries, pre-computing keyword sets.
    pub fn new(entries: Vec<DbEntry>) -> Self {
        let keywords = entries
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

        Self { entries, keywords }
    }

    /// Load from JSON string (for db_char.json / db_other.json).
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let entries: Vec<DbEntry> =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse DB JSON: {e}"))?;
        Ok(Self::new(entries))
    }
}

/// Run the full matching pipeline on a mod candidate.
///
/// Pipeline order:
/// 1. L1: Direct Name Match (contains)
/// 2. L2: Keyword Token Intersection
/// 3. L3: Deep Content Scan (subfolder/file names)
/// 4. L4: AI Match (stub, default OFF)
/// 5. L5: Fuzzy Match (Levenshtein)
///
/// # Covers: TC-2.2-01, TC-2.2-02, TC-2.2-03
pub fn match_folder(
    candidate: &ModCandidate,
    db: &MasterDb,
    content: &FolderContent,
) -> MatchResult {
    let clean_name = normalizer::strip_noise_prefixes(&candidate.display_name);

    // L1: Direct Name Match
    if let Some(result) = name_match(&clean_name, db) {
        return with_skin_detection(result, &clean_name, db);
    }

    // L2: Token Match
    if let Some(result) = token_match(&clean_name, db) {
        return with_skin_detection(result, &clean_name, db);
    }

    // L3: Deep Content Scan
    if let Some(result) = content_scan(content, db) {
        return with_skin_detection(result, &clean_name, db);
    }

    // L4: AI Match (stub — default OFF, skip)

    // L5: Fuzzy Match
    if let Some(result) = fuzzy_match(&clean_name, db) {
        return with_skin_detection(result, &clean_name, db);
    }

    MatchResult::unmatched()
}

/// L1: Direct Name Match — Check if db.name or any tag appears in the folder name.
///
/// # Covers: TC-2.2-01
fn name_match(folder_name: &str, db: &MasterDb) -> Option<MatchResult> {
    let folder_lower = folder_name.to_lowercase();

    for entry in &db.entries {
        // Check name
        if folder_lower.contains(&entry.name.to_lowercase()) {
            return Some(MatchResult {
                object_name: entry.name.clone(),
                object_type: entry.object_type.clone(),
                level: MatchLevel::L1Name,
                confidence: Confidence::High,
                detail: format!("Name contains \"{}\"", entry.name),
                detected_skin: None,
            });
        }

        // Check tags (aliases)
        for tag in &entry.tags {
            if tag.len() > 2 && folder_lower.contains(&tag.to_lowercase()) {
                return Some(MatchResult {
                    object_name: entry.name.clone(),
                    object_type: entry.object_type.clone(),
                    level: MatchLevel::L1Name,
                    confidence: Confidence::High,
                    detail: format!("Name contains tag \"{}\"", tag),
                    detected_skin: None,
                });
            }
        }
    }

    None
}

/// L2: Keyword Token Intersection — Match tokens with min 1 keyword > 3 chars.
fn token_match(folder_name: &str, db: &MasterDb) -> Option<MatchResult> {
    let folder_tokens = normalizer::preprocess_text(folder_name);

    let mut best_match: Option<(usize, usize)> = None; // (entry_idx, intersection_count)

    for (idx, db_tokens) in &db.keywords {
        let intersection: HashSet<_> = folder_tokens
            .intersection(db_tokens)
            .filter(|t| t.len() > 3) // Filter noise words (< 4 chars)
            .collect();

        if !intersection.is_empty() {
            let count = intersection.len();
            if best_match.is_none_or(|(_, best)| count > best) {
                best_match = Some((*idx, count));
            }
        }
    }

    best_match.map(|(idx, count)| {
        let entry = &db.entries[idx];
        MatchResult {
            object_name: entry.name.clone(),
            object_type: entry.object_type.clone(),
            level: MatchLevel::L2Token,
            confidence: Confidence::High,
            detail: format!("{count} keyword(s) matched"),
            detected_skin: None,
        }
    })
}

/// L3: Deep Content Scan — Apply L1/L2 on subfolder names and file names.
///
/// # Covers: TC-2.2-02
fn content_scan(content: &FolderContent, db: &MasterDb) -> Option<MatchResult> {
    // Check subfolder names first (higher confidence)
    for subfolder_name in &content.subfolder_names {
        let clean = normalizer::strip_noise_prefixes(subfolder_name);

        if let Some(mut result) = name_match(&clean, db) {
            result.level = MatchLevel::L3Content;
            result.confidence = Confidence::High;
            result.detail = format!("Matched via subfolder: {subfolder_name}");
            return Some(result);
        }

        if let Some(mut result) = token_match(&clean, db) {
            result.level = MatchLevel::L3Content;
            result.confidence = Confidence::Medium;
            result.detail = format!("Token match via subfolder: {subfolder_name}");
            return Some(result);
        }
    }

    // Check file names (medium confidence)
    for file in &content.files {
        let file_stem = std::path::Path::new(&file.name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        if let Some(mut result) = name_match(&file_stem, db) {
            result.level = MatchLevel::L3Content;
            result.confidence = Confidence::Medium;
            result.detail = format!("Matched via file: {}", file.name);
            return Some(result);
        }
    }

    None
}

/// L5: Fuzzy Match — Levenshtein distance using strsim crate.
///
/// Thresholds: > 0.8 = Medium, > 0.6 = Low
///
/// # Covers: TC-2.2-03
fn fuzzy_match(folder_name: &str, db: &MasterDb) -> Option<MatchResult> {
    let folder_lower = folder_name.to_lowercase();
    let mut best_score: f64 = 0.0;
    let mut best_entry: Option<&DbEntry> = None;

    for entry in &db.entries {
        let score = strsim::normalized_levenshtein(&folder_lower, &entry.name.to_lowercase());
        if score > best_score {
            best_score = score;
            best_entry = Some(entry);
        }

        // Also check tags
        for tag in &entry.tags {
            let tag_score = strsim::normalized_levenshtein(&folder_lower, &tag.to_lowercase());
            if tag_score > best_score {
                best_score = tag_score;
                best_entry = Some(entry);
            }
        }
    }

    if best_score < 0.6 {
        return None;
    }

    let confidence = if best_score > 0.8 {
        Confidence::Medium
    } else {
        Confidence::Low
    };

    best_entry.map(|entry| MatchResult {
        object_name: entry.name.clone(),
        object_type: entry.object_type.clone(),
        level: MatchLevel::L5Fuzzy,
        confidence,
        detail: format!("Fuzzy match: {:.0}% similarity", best_score * 100.0),
        detected_skin: None,
    })
}

/// Post-match: detect skin/variant if the object is a Character.
///
/// # Covers: Epic 2 §B.3
fn with_skin_detection(mut result: MatchResult, folder_name: &str, db: &MasterDb) -> MatchResult {
    if result.object_type != "Character" {
        return result;
    }

    let folder_tokens = normalizer::preprocess_text(folder_name);

    // Find the matching entry and check its official_skins
    if let Some(entry) = db.entries.iter().find(|e| e.name == result.object_name) {
        for skin in &entry.official_skins {
            let skin_tokens = normalizer::preprocess_text(skin);
            if !skin_tokens.is_disjoint(&folder_tokens) {
                result.detected_skin = Some(skin.clone());
                break;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::scanner::walker::FileInfo;
    use std::path::PathBuf;

    fn test_db() -> MasterDb {
        MasterDb::new(vec![
            DbEntry {
                name: "Raiden Shogun".to_string(),
                tags: vec!["Raiden".to_string(), "Ei".to_string()],
                object_type: "Character".to_string(),
                official_skins: vec!["Wish".to_string()],
            },
            DbEntry {
                name: "Albedo".to_string(),
                tags: vec![],
                object_type: "Character".to_string(),
                official_skins: vec![],
            },
            DbEntry {
                name: "Lumine".to_string(),
                tags: vec!["Traveler".to_string()],
                object_type: "Character".to_string(),
                official_skins: vec![],
            },
            DbEntry {
                name: "Primordial Jade Winged-Spear".to_string(),
                tags: vec!["PJWS".to_string()],
                object_type: "Weapon".to_string(),
                official_skins: vec![],
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

    // Covers: TC-2.2-01 — Name Match via tag
    #[test]
    fn test_l1_name_match_via_tag() {
        let db = test_db();
        let c = candidate("Raiden_skin_v2");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Raiden Shogun");
        assert_eq!(result.level, MatchLevel::L1Name);
    }

    // Covers: TC-2.2-02 — Content Match via INI filename
    #[test]
    fn test_l3_content_scan_via_file() {
        let db = test_db();
        let c = candidate("unknown_mod_123");
        let content = FolderContent {
            subfolder_names: vec![],
            files: vec![FileInfo {
                path: PathBuf::from("/mods/unknown_mod_123/albedo.ini"),
                name: "albedo.ini".to_string(),
                extension: "ini".to_string(),
            }],
            ini_files: vec![],
        };

        let result = match_folder(&c, &db, &content);

        assert_eq!(result.object_name, "Albedo");
        assert_eq!(result.level, MatchLevel::L3Content);
        assert_eq!(result.confidence, Confidence::Medium);
    }

    // Covers: TC-2.2-02 — Content Match via subfolder name
    #[test]
    fn test_l3_content_scan_via_subfolder() {
        let db = test_db();
        let c = candidate("abi");
        let content = FolderContent {
            subfolder_names: vec!["Lumine_mods123".to_string()],
            files: vec![],
            ini_files: vec![],
        };

        let result = match_folder(&c, &db, &content);

        assert_eq!(result.object_name, "Lumine");
        assert_eq!(result.level, MatchLevel::L3Content);
        assert_eq!(result.confidence, Confidence::High);
    }

    // Covers: TC-2.2-03 — Fuzzy Match
    #[test]
    fn test_l5_fuzzy_match() {
        let db = test_db();
        // "Albdeo" is a transposition of "Albedo" — NOT a substring, so L1/L2 won't catch it
        let c = candidate("Albdeo");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Albedo");
        assert_eq!(result.level, MatchLevel::L5Fuzzy);
        assert!(result.confidence == Confidence::Medium || result.confidence == Confidence::Low);
    }

    // Covers: NC-2.2-02 — No Match Found
    #[test]
    fn test_no_match() {
        let db = test_db();
        let c = candidate("XYZ_Random_Stuff");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.level, MatchLevel::Unmatched);
        assert_eq!(result.object_type, "Uncategorized");
    }

    // Covers: Epic 2 §B.3 — Skin detection
    #[test]
    fn test_skin_detection() {
        let db = test_db();
        let c = candidate("Raiden Shogun Wish Skin");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Raiden Shogun");
        assert_eq!(result.detected_skin, Some("Wish".to_string()));
    }

    #[test]
    fn test_masterdb_from_json() {
        let json = r#"[
            {"name": "Test Char", "tags": ["tc"], "object_type": "Character", "official_skins": []},
            {"name": "Test Weapon", "tags": [], "object_type": "Weapon"}
        ]"#;

        let db = MasterDb::from_json(json).unwrap();
        assert_eq!(db.entries.len(), 2);
        assert_eq!(db.entries[0].name, "Test Char");
    }
}
