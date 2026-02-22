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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// The conflicting hash value.
    pub hash: String,
    /// Section name where the hash was found.
    pub section_name: String,
    /// Paths of the mods that conflict.
    pub mod_paths: Vec<String>,
}

/// Entry tracking a single hash occurrence.
#[derive(Debug, Clone)]
struct HashEntry {
    hash: String,
    section: String,
    source_path: PathBuf,
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
pub fn detect_conflicts(ini_files: &[PathBuf]) -> Vec<ConflictInfo> {
    let mut hash_map: HashMap<String, Vec<HashEntry>> = HashMap::new();

    for ini_path in ini_files {
        let entries = parse_ini_hashes(ini_path);
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
                .map(|e| get_mod_root(&e.source_path))
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
                mod_paths: unique_paths,
            })
        })
        .collect()
}

/// Parse a single INI file for TextureOverride hash entries.
///
/// Looks for sections matching `[TextureOverride...]` and extracts `hash` values.
fn parse_ini_hashes(ini_path: &Path) -> Vec<HashEntry> {
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
            in_texture_override = current_section
                .to_lowercase()
                .starts_with("textureoverride");
            continue;
        }

        // Hash value within a TextureOverride section
        if in_texture_override {
            if let Some(hash_val) = parse_hash_line(trimmed) {
                entries.push(HashEntry {
                    hash: hash_val,
                    section: current_section.clone(),
                    source_path: ini_path.to_path_buf(),
                });
            }
        }
    }

    entries
}

/// Parse a line like `hash = abcd1234` and return the hash value.
fn parse_hash_line(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    if !lower.starts_with("hash") {
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

/// Get the mod root directory from an INI file path.
///
/// Assumes structure: `.../ModRoot/some/path/file.ini` → returns `ModRoot` path.
fn get_mod_root(ini_path: &Path) -> String {
    // Walk up parents to find a reasonable root
    // For simplicity, use the grandparent or parent depending on depth
    ini_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ini_path.to_string_lossy().to_string())
}

#[cfg(test)]
#[path = "tests/conflict_tests.rs"]
mod tests;
