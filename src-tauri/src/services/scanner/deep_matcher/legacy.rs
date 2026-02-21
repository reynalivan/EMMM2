//! Legacy matcher functions (L0-L5 pipeline stages).
//!
//! These functions implement the original deep matcher algorithm and are preserved
//! for backward compatibility during the staged refactor. They will eventually be
//! replaced by the new staged matcher implementation.

use std::collections::HashSet;

use crate::services::scanner::normalizer;
use crate::services::scanner::walker::{FolderContent, ModCandidate};

use super::types::{Confidence, DbEntry, MatchLevel, MatchResult};
use super::MasterDb;

/// Main entry point: runs the L0-L5 matcher pipeline.
///
/// Pipeline order:
/// L0 (Skin Alias) → L1 (Name) → L2 (Token) → L3 (Content) → L4 (AI/stub) → L5 (Fuzzy)
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
