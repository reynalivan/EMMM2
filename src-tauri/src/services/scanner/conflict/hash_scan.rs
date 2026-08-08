//! Shader & buffer conflict detection for 3DMigoto mods.
//!
//! Parses `.ini` files for `[TextureOverride...]` sections with `hash = xxxx`
//! and reports when 2+ mods share the same hash (potential in-game conflict).
//!
//! # Covers: US-2.Z, TC-2.4-01

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Information about a shader/buffer hash conflict.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ConflictInfo {
    /// The conflicting hash value.
    pub hash: String,
    /// Section name where the hash was found.
    pub section_name: String,
    /// Paths of the mods that conflict.
    pub mod_paths: Vec<String>,
    /// Whether at least two conflicting mods are currently enabled.
    pub is_active: bool,
}

/// Entry tracking a single hash occurrence.
#[derive(Debug, Clone)]
struct HashEntry {
    hash: String,
    section: String,
    mod_root: PathBuf,
}

/// Detect shader/buffer conflicts across multiple INI files.
///
/// Scans each `.ini` file for `[TextureOverride...]` sections containing
/// `hash = <value>`, then groups by hash to find conflicts.
///
/// # Returns
/// - A `Vec<ConflictInfo>` for each hash that appears in 2+ different mod paths.
///
/// # Covers: TC-2.4-01
pub fn detect_conflicts(ini_files: &[(PathBuf, PathBuf)]) -> Vec<ConflictInfo> {
    let mut hash_map: HashMap<String, Vec<HashEntry>> = HashMap::new();

    for (mod_root, ini_path) in ini_files {
        let entries = parse_ini_hashes(ini_path, mod_root);
        for entry in entries {
            hash_map.entry(entry.hash.clone()).or_default().push(entry);
        }
    }

    // Find conflicts: hashes with entries from 2+ different mod roots
    hash_map
        .into_iter()
        .filter_map(|(hash, entries)| {
            // Deduplicate by mod root path
            let unique_paths: Vec<String> = entries
                .iter()
                .map(|e| e.mod_root.to_string_lossy().to_string())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            if unique_paths.len() < 2 {
                return None;
            }

            let section_name = entries
                .first()
                .map(|e| e.section.clone())
                .unwrap_or_default();

            Some(ConflictInfo {
                hash,
                section_name,
                mod_paths: unique_paths.clone(),
                // Determine if active: 2+ mods must be enabled.
                // Note: We need a way to check status. In this pure-FS service,
                // we check if folder name doesn't start with "DISABLED ".
                is_active: unique_paths
                    .iter()
                    .filter(|p| {
                        !crate::common::normalizer::is_disabled_folder(
                            Path::new(p)
                                .file_name()
                                .unwrap_or_default()
                                .to_str()
                                .unwrap_or(""),
                        )
                    })
                    .count()
                    >= 2,
            })
        })
        .collect()
}

/// Parse a single INI file for TextureOverride hash entries.
///
/// Looks for sections matching `[TextureOverride...]` and extracts `hash` values.
fn parse_ini_hashes(ini_path: &Path, mod_root: &Path) -> Vec<HashEntry> {
    let content = match fs::read_to_string(ini_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read INI {}: {e}", ini_path.display());
            return Vec::new();
        }
    };

    // Skip empty files (EC-2.05)
    if content.trim().is_empty() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let mut current_section = String::new();
    let mut in_texture_override = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Section header
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len() - 1].to_string();
            let section_lower = current_section.to_lowercase();
            in_texture_override = section_lower.starts_with("textureoverride")
                || section_lower.starts_with("shaderoverride")
                || section_lower.starts_with("resource")
                || section_lower.starts_with("customshader");
            continue;
        }

        // Hash value within a TextureOverride section
        if in_texture_override {
            if let Some(hash_val) = parse_hash_line(trimmed) {
                entries.push(HashEntry {
                    hash: hash_val,
                    section: current_section.clone(),
                    mod_root: mod_root.to_path_buf(),
                });
            }
        }
    }

    entries
}

/// Parse a line like `hash = abcd1234` and return the hash value.
fn parse_hash_line(line: &str) -> Option<String> {
    // Prefix test without allocating a lowercased copy of every line in the file.
    if !line
        .get(..4)
        .is_some_and(|head| head.eq_ignore_ascii_case("hash"))
    {
        return None;
    }

    // Split on '=' and get the value part
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }

    let key = parts[0].trim().to_lowercase();
    if key != "hash" {
        return None;
    }

    let value = parts[1].trim().to_string();
    if value.is_empty() {
        return None;
    }
    Some(value)
}

#[cfg(test)]
#[path = "tests/conflict_tests.rs"]
mod tests;
