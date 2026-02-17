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

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "High"),
            Confidence::Medium => write!(f, "Medium"),
            Confidence::Low => write!(f, "Low"),
            Confidence::None => write!(f, "None"),
        }
    }
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

impl std::fmt::Display for MatchLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchLevel::L1Name => write!(f, "L1Name"),
            MatchLevel::L2Token => write!(f, "L2Token"),
            MatchLevel::L3Content => write!(f, "L3Content"),
            MatchLevel::L4Ai => write!(f, "L4Ai"),
            MatchLevel::L5Fuzzy => write!(f, "L5Fuzzy"),
            MatchLevel::Unmatched => write!(f, "Unmatched"),
        }
    }
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
    /// Canonical folder name for this skin variant (first alias).
    /// e.g. "JeanCN" for Gunnhildr's Heritage.
    pub skin_folder_name: Option<String>,
}

impl MatchResult {
    /// Create an "unmatched" result.
    pub fn unmatched() -> Self {
        Self {
            object_name: String::new(),
            object_type: "Other".to_string(),
            level: MatchLevel::Unmatched,
            confidence: Confidence::None,
            detail: "No match found".to_string(),
            detected_skin: None,
            skin_folder_name: None,
        }
    }
}

/// A named skin/outfit with aliases for matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSkin {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Relative path to the skin-specific thumbnail image.
    #[serde(default)]
    pub thumbnail_skin_path: Option<String>,
    /// Rarity tier of this skin (e.g. "4", "5").
    #[serde(default)]
    pub rarity: Option<String>,
}

/// A single entry from the Master DB (e.g. gimi.json, srmi.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbEntry {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub object_type: String,
    #[serde(default)]
    pub custom_skins: Vec<CustomSkin>,
    /// Relative path to the thumbnail image for this entry.
    #[serde(default)]
    pub thumbnail_path: Option<String>,
    /// Game-specific metadata (element, weapon, rarity, region, etc.).
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
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

    // L0: Skin Alias Match — check if folder name IS a skin alias (e.g. "JeanCN")
    // This must run before L1 because "JeanCN" won't contain "Jean" as a substring.
    if let Some(result) = skin_alias_match(&clean_name, db) {
        return result;
    }

    // L1: Direct Name Match
    if let Some(result) = name_match(&clean_name, db) {
        return with_skin_detection(result, &clean_name, db);
    }

    // L2: Token Match
    if let Some(result) = token_match(&clean_name, db) {
        return with_skin_detection(result, &clean_name, db);
    }

    // L3: Deep Content Scan (also checks skin aliases in subfolders)
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

/// L0: Skin Alias Match — check if folder name matches a skin alias directly.
///
/// Handles cases where the mod folder IS named after a skin alias (e.g. "JeanCN",
/// "JeanSea", "DilucRed"). These folders won't pass L1 name match because the
/// character's base name isn't a substring of the alias.
///
/// Returns the parent character as match, with detected_skin and skin_folder_name set.
pub fn skin_alias_match(folder_name: &str, db: &MasterDb) -> Option<MatchResult> {
    let folder_lower = folder_name.to_lowercase();
    let folder_tokens = normalizer::preprocess_text(folder_name);

    for entry in &db.entries {
        if entry.object_type != "Character" {
            continue;
        }

        for skin in &entry.custom_skins {
            for alias in &skin.aliases {
                let alias_lower = alias.to_lowercase();

                // Check 1: Folder name contains the alias as substring
                if alias.len() > 2 && folder_lower.contains(&alias_lower) {
                    let canonical = skin.aliases.first().map(|a| a.to_string());
                    return Some(MatchResult {
                        object_name: entry.name.clone(),
                        object_type: entry.object_type.clone(),
                        level: MatchLevel::L1Name,
                        confidence: Confidence::High,
                        detail: format!(
                            "Skin alias \"{}\" → {} ({})",
                            alias, entry.name, skin.name
                        ),
                        detected_skin: Some(skin.name.clone()),
                        skin_folder_name: canonical,
                    });
                }

                // Check 2: Token intersection (for multi-word aliases like "Jean Sea Breeze")
                let alias_tokens = normalizer::preprocess_text(alias);
                let long_tokens: Vec<_> = alias_tokens
                    .intersection(&folder_tokens)
                    .filter(|t| t.len() > 3)
                    .collect();

                if !long_tokens.is_empty() && long_tokens.len() == alias_tokens.len() {
                    let canonical = skin.aliases.first().map(|a| a.to_string());
                    return Some(MatchResult {
                        object_name: entry.name.clone(),
                        object_type: entry.object_type.clone(),
                        level: MatchLevel::L2Token,
                        confidence: Confidence::High,
                        detail: format!(
                            "Skin alias tokens \"{}\" → {} ({})",
                            alias, entry.name, skin.name
                        ),
                        detected_skin: Some(skin.name.clone()),
                        skin_folder_name: canonical,
                    });
                }
            }
        }
    }

    None
}

/// L1: Direct Name Match — Check if db.name or any tag appears in the folder name.
///
/// # Covers: TC-2.2-01
pub fn name_match(folder_name: &str, db: &MasterDb) -> Option<MatchResult> {
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
                skin_folder_name: None,
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
                    skin_folder_name: None,
                });
            }
        }
    }

    None
}

/// L2: Keyword Token Intersection — Match tokens with min 1 keyword > 3 chars.
pub fn token_match(folder_name: &str, db: &MasterDb) -> Option<MatchResult> {
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
            skin_folder_name: None,
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

        // Check skin aliases in subfolder names (e.g. subfolder "JeanCN")
        if let Some(mut result) = skin_alias_match(&clean, db) {
            result.level = MatchLevel::L3Content;
            result.confidence = Confidence::High;
            result.detail = format!("Skin alias via subfolder: {subfolder_name}");
            return Some(result);
        }

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

        // Check skin aliases in file names
        if let Some(mut result) = skin_alias_match(&file_stem, db) {
            result.level = MatchLevel::L3Content;
            result.confidence = Confidence::Medium;
            result.detail = format!("Skin alias via file: {}", file.name);
            return Some(result);
        }

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
pub fn fuzzy_match(folder_name: &str, db: &MasterDb) -> Option<MatchResult> {
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
        skin_folder_name: None,
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

    // Find the matching entry and check its custom_skins (name + aliases)
    if let Some(entry) = db.entries.iter().find(|e| e.name == result.object_name) {
        'skin_loop: for skin in &entry.custom_skins {
            // Check skin name tokens
            let name_tokens = normalizer::preprocess_text(&skin.name);
            if !name_tokens.is_disjoint(&folder_tokens) {
                result.detected_skin = Some(skin.name.clone());
                result.skin_folder_name = skin.aliases.first().cloned();
                break;
            }
            // Check alias tokens
            for alias in &skin.aliases {
                let alias_tokens = normalizer::preprocess_text(alias);
                if !alias_tokens.is_disjoint(&folder_tokens) {
                    result.detected_skin = Some(skin.name.clone());
                    result.skin_folder_name = skin.aliases.first().cloned();
                    break 'skin_loop;
                }
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
                custom_skins: vec![CustomSkin {
                    name: "Wish".to_string(),
                    aliases: vec!["RaidenWish".to_string(), "Raiden2".to_string()],
                    thumbnail_skin_path: None,
                    rarity: None,
                }],
                thumbnail_path: None,
                metadata: None,
            },
            DbEntry {
                name: "Albedo".to_string(),
                tags: vec![],
                object_type: "Character".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
            },
            DbEntry {
                name: "Lumine".to_string(),
                tags: vec!["Traveler".to_string()],
                object_type: "Character".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
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
            },
            DbEntry {
                name: "Primordial Jade Winged-Spear".to_string(),
                tags: vec!["PJWS".to_string()],
                object_type: "Weapon".to_string(),
                custom_skins: vec![],
                thumbnail_path: None,
                metadata: None,
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
        assert_eq!(result.object_type, "Other");
    }

    // Covers: Epic 2 §B.3 — Skin detection (by name)
    #[test]
    fn test_skin_detection_by_name() {
        let db = test_db();
        let c = candidate("Raiden Shogun Wish Skin");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Raiden Shogun");
        assert_eq!(result.detected_skin, Some("Wish".to_string()));
        assert_eq!(result.skin_folder_name, Some("RaidenWish".to_string()));
    }

    // Covers: Epic 2 §B.3 — Skin detection (by alias)
    #[test]
    fn test_skin_detection_by_alias() {
        let db = test_db();
        let c = candidate("Raiden Shogun RaidenWish");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Raiden Shogun");
        assert_eq!(result.detected_skin, Some("Wish".to_string()));
        assert_eq!(result.skin_folder_name, Some("RaidenWish".to_string()));
    }

    // Covers: Skin alias pipeline — folder IS the alias (no base name)
    #[test]
    fn test_skin_alias_match_direct() {
        let db = test_db();
        // "JeanCN" doesn't contain "Jean" as a full match at L1
        // (actually "Jean" IS a substring of "JeanCN"... let me think)
        // Wait: "jeancn".contains("jean") is TRUE.
        // So L0 skin_alias_match should fire first and identify the skin.
        let c = candidate("JeanCN");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Jean");
        assert_eq!(result.level, MatchLevel::L1Name);
        assert_eq!(
            result.detected_skin,
            Some("Gunnhildr's Heritage".to_string())
        );
        assert_eq!(result.skin_folder_name, Some("JeanCN".to_string()));
    }

    // Covers: Skin alias pipeline — second skin alias
    #[test]
    fn test_skin_alias_match_second_skin() {
        let db = test_db();
        let c = candidate("JeanSea");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Jean");
        assert_eq!(
            result.detected_skin,
            Some("Sea Breeze Dandelion".to_string())
        );
        assert_eq!(result.skin_folder_name, Some("JeanSea".to_string()));
    }

    // Covers: Skin alias — two folders with same skin resolve identically
    #[test]
    fn test_skin_alias_merge_detection() {
        let db = test_db();

        // "JeanCN" and "Jean2" both belong to Gunnhildr's Heritage
        let r1 = match_folder(&candidate("JeanCN"), &db, &empty_content());
        let r2 = match_folder(&candidate("Jean2"), &db, &empty_content());

        assert_eq!(r1.object_name, "Jean");
        assert_eq!(r2.object_name, "Jean");
        assert_eq!(r1.detected_skin, r2.detected_skin);
        // Both should have canonical folder name = first alias
        assert_eq!(r1.skin_folder_name, Some("JeanCN".to_string()));
        assert_eq!(r2.skin_folder_name, Some("JeanCN".to_string()));
    }

    // Covers: Base character folder (no skin detected)
    #[test]
    fn test_base_character_no_skin() {
        let db = test_db();
        let c = candidate("Jean");
        let result = match_folder(&c, &db, &empty_content());

        assert_eq!(result.object_name, "Jean");
        // skin_alias_match won't fire for base name "Jean" — it's a name match not alias
        // Actually "Jean" IS a substring match at L0 too via alias "JeanCN"...
        // Hmm, "jean" is contained in "jeancn"? No: folder is "Jean", so folder_lower = "jean".
        // alias = "JeanCN" → alias_lower = "jeancn". folder_lower.contains(&alias_lower)
        // → "jean".contains("jeancn") → FALSE. Good.
        // So L0 won't fire, L1 name_match fires for "Jean"
        assert_eq!(result.level, MatchLevel::L1Name);
        assert_eq!(result.detected_skin, None);
        assert_eq!(result.skin_folder_name, None);
    }

    #[test]
    fn test_masterdb_from_json() {
        let json = r#"[
            {"name": "Test Char", "tags": ["tc"], "object_type": "Character", "custom_skins": [{"name": "Outfit A", "aliases": ["A2"]}]},
            {"name": "Test Weapon", "tags": [], "object_type": "Weapon"}
        ]"#;

        let db = MasterDb::from_json(json).unwrap();
        assert_eq!(db.entries.len(), 2);
        assert_eq!(db.entries[0].name, "Test Char");
    }
}
