//! info.json lifecycle manager for mod folders.
//!
//! - Reads and parses existing info.json files.
//! - Creates default info.json when a new mod is detected.
//! - Updates specific fields (merge, not overwrite).
//!
//! # Covers: Epic 4 §C, DI-4.03 (info.json Lifecycle)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// The standard mod metadata structure stored in `info.json`.
///
/// Matches the default template from Epic 4 cross-cutting requirements:
/// `{ actual_name, author, description, version, tags, is_safe, is_favorite }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModInfo {
    #[serde(default)]
    pub actual_name: String,
    #[serde(default = "default_author")]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_true")]
    pub is_safe: bool,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub is_auto_sync: bool,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

fn default_author() -> String {
    "Unknown".to_string()
}
fn default_version() -> String {
    "1.0".to_string()
}
fn default_true() -> bool {
    true
}

impl ModInfo {
    /// Create a default ModInfo from a folder name.
    pub fn from_folder_name(name: &str) -> Self {
        Self {
            actual_name: name.to_string(),
            author: default_author(),
            description: String::new(),
            version: default_version(),
            tags: Vec::new(),
            is_safe: true,
            is_favorite: false,
            is_auto_sync: false,
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// Read and parse info.json from a mod folder.
///
/// Returns `None` if the file doesn't exist.
/// Returns `Err` if the file exists but is malformed.
pub fn read_info_json(mod_path: &Path) -> Result<Option<ModInfo>, String> {
    let info_path = mod_path.join("info.json");
    if !info_path.exists() {
        return Ok(None);
    }

    let content =
        fs::read_to_string(&info_path).map_err(|e| format!("Failed to read info.json: {e}"))?;

    let info: ModInfo =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse info.json: {e}"))?;

    Ok(Some(info))
}

/// Create a default info.json in the given mod folder.
///
/// Uses the folder's name as `actual_name`.
/// Does NOT overwrite if the file already exists.
pub fn create_default_info_json(mod_path: &Path) -> Result<ModInfo, String> {
    let info_path = mod_path.join("info.json");
    if info_path.exists() {
        return read_info_json(mod_path)?
            .ok_or_else(|| "info.json exists but is empty".to_string());
    }

    let folder_name = mod_path
        .file_name()
        .ok_or("Invalid folder path")?
        .to_string_lossy();

    // Strip "DISABLED " prefix if present
    let clean_name = folder_name
        .strip_prefix("DISABLED ")
        .unwrap_or(&folder_name);

    let info = ModInfo::from_folder_name(clean_name);

    let json = serde_json::to_string_pretty(&info)
        .map_err(|e| format!("Failed to serialize info.json: {e}"))?;
    fs::write(&info_path, json).map_err(|e| format!("Failed to write info.json: {e}"))?;

    log::info!("Created default info.json for '{}'", clean_name);
    Ok(info)
}

/// Partial update struct — only fields that are `Some` will be updated.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModInfoUpdate {
    pub actual_name: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tags_add: Option<Vec<String>>,
    pub tags_remove: Option<Vec<String>>,
    pub is_safe: Option<bool>,
    pub is_favorite: Option<bool>,
    pub is_auto_sync: Option<bool>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Update specific fields in an existing info.json (merge, not overwrite).
///
/// If info.json doesn't exist, creates a default first, then applies the update.
pub fn update_info_json(mod_path: &Path, update: &ModInfoUpdate) -> Result<ModInfo, String> {
    let mut info = match read_info_json(mod_path)? {
        Some(existing) => existing,
        None => create_default_info_json(mod_path)?,
    };

    // Apply partial updates
    if let Some(ref name) = update.actual_name {
        info.actual_name = name.clone();
    }
    if let Some(ref author) = update.author {
        info.author = author.clone();
    }
    if let Some(ref desc) = update.description {
        info.description = desc.clone();
    }
    if let Some(ref ver) = update.version {
        info.version = ver.clone();
    }

    // Tags logic: Set > Add > Remove
    if let Some(ref tags) = update.tags {
        info.tags = tags.clone();
    }
    if let Some(ref add) = update.tags_add {
        for t in add {
            if !info.tags.contains(t) {
                info.tags.push(t.clone());
            }
        }
    }
    if let Some(ref remove) = update.tags_remove {
        info.tags.retain(|t| !remove.contains(t));
    }

    if let Some(safe) = update.is_safe {
        info.is_safe = safe;
    }
    if let Some(fav) = update.is_favorite {
        info.is_favorite = fav;
    }
    if let Some(sync) = update.is_auto_sync {
        info.is_auto_sync = sync;
    }
    if let Some(ref meta) = update.metadata {
        // Merge metadata (overwrite existing keys, keep others)
        for (k, v) in meta {
            info.metadata.insert(k.clone(), v.clone());
        }
    }

    // Write back
    let info_path = mod_path.join("info.json");
    let json =
        serde_json::to_string_pretty(&info).map_err(|e| format!("Failed to serialize: {e}"))?;
    fs::write(&info_path, json).map_err(|e| format!("Failed to write info.json: {e}"))?;

    Ok(info)
}

#[cfg(test)]
#[path = "tests/info_json_tests.rs"]
mod tests;
