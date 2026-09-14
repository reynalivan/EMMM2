//! info.json lifecycle manager for mod folders.
//!
//! - Reads and parses existing info.json files.
//! - Creates default info.json when a new mod is detected.
//! - Updates specific fields (merge, not overwrite).
//!
//! # Covers: Epic 4 §C, DI-4.03 (info.json Lifecycle)

use crate::shared::errors::MetadataError;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Helper to parse either a string or an array of strings into a Vec<String>
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Value = Deserialize::deserialize(deserializer)?;
    match v {
        Value::String(s) => Ok(vec![s]),
        Value::Array(arr) => {
            let mut result = Vec::new();
            for item in arr {
                if let Value::String(s) = item {
                    result.push(s);
                }
            }
            Ok(result)
        }
        _ => Ok(Vec::new()), // Fallback for invalid types
    }
}

/// The standard mod metadata structure stored in `info.json`.
///
/// Matches the default template from Epic 4 cross-cutting requirements:
/// `{ actual_name, author, description, version, tags, is_safe, is_favorite }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
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
    pub is_pinned: bool,
    #[serde(default)]
    pub is_auto_sync: bool,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub preset_name: Vec<String>,
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
            is_pinned: false,
            is_auto_sync: false,
            preset_name: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

fn default_info_for_folder(mod_path: &Path) -> Result<ModInfo, MetadataError> {
    if !mod_path.is_dir() {
        return Err(MetadataError::NotFound(format!(
            "Mod folder does not exist: {}",
            mod_path.display()
        )));
    }

    let folder_name = mod_path
        .file_name()
        .ok_or_else(|| MetadataError::Validation("Invalid folder path".to_string()))?
        .to_string_lossy();
    let clean_name =
        crate::modules::workspace::domain::normalizer::normalize_display_name(&folder_name);

    Ok(ModInfo::from_folder_name(&clean_name))
}

fn write_info_json(mod_path: &Path, info: &ModInfo) -> Result<(), MetadataError> {
    let json = serde_json::to_string_pretty(info)
        .map_err(|error| MetadataError::Validation(format!("Failed to serialize: {error}")))?;
    let info_path = mod_path.join("info.json");
    crate::platform::fs::atomic_file::atomic_write(&info_path, json.as_bytes())
        .map_err(|error| MetadataError::Io(error.to_string()))
}

/// Read and parse info.json from a mod folder.
///
/// Returns `None` if the file doesn't exist.
/// Returns `Err` if the file exists but is malformed.
pub fn read_info_json(mod_path: &Path) -> Result<Option<ModInfo>, MetadataError> {
    let info_path = mod_path.join("info.json");
    if !info_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&info_path)?;
    let info: ModInfo = serde_json::from_str(&content)
        .map_err(|e| MetadataError::Validation(format!("Failed to parse info.json: {e}")))?;

    Ok(Some(info))
}

/// Create a default info.json in the given mod folder.
///
/// Uses the folder's name as `actual_name`.
/// Does NOT overwrite if the file already exists.
pub fn create_default_info_json(mod_path: &Path) -> Result<ModInfo, MetadataError> {
    let info_path = mod_path.join("info.json");
    if info_path.exists() {
        return read_info_json(mod_path)?
            .ok_or_else(|| MetadataError::Validation("info.json exists but is empty".to_string()));
    }

    let info = default_info_for_folder(mod_path)?;
    write_info_json(mod_path, &info)?;

    log::info!("Created default info.json for '{}'", info.actual_name);
    Ok(info)
}

/// Partial update struct — only fields that are `Some` will be updated.
#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
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
    pub is_pinned: Option<bool>,
    pub is_auto_sync: Option<bool>,
    pub preset_name_add: Option<Vec<String>>,
    pub preset_name_remove: Option<Vec<String>>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

fn apply_update(info: &mut ModInfo, update: &ModInfoUpdate) {
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
        for tag in add {
            if !info.tags.contains(tag) {
                info.tags.push(tag.clone());
            }
        }
    }
    if let Some(ref remove) = update.tags_remove {
        info.tags.retain(|tag| !remove.contains(tag));
    }

    if let Some(safe) = update.is_safe {
        info.is_safe = safe;
    }
    if let Some(favorite) = update.is_favorite {
        info.is_favorite = favorite;
    }
    if let Some(pinned) = update.is_pinned {
        info.is_pinned = pinned;
    }
    if let Some(sync) = update.is_auto_sync {
        info.is_auto_sync = sync;
    }

    if let Some(ref add) = update.preset_name_add {
        for preset_name in add {
            if !info.preset_name.contains(preset_name) {
                info.preset_name.push(preset_name.clone());
            }
        }
    }
    if let Some(ref remove) = update.preset_name_remove {
        info.preset_name
            .retain(|preset_name| !remove.contains(preset_name));
    }

    if let Some(ref metadata) = update.metadata {
        for (key, value) in metadata {
            info.metadata.insert(key.clone(), value.clone());
        }
    }
}

/// Update from bytes already read by the caller. This lets a bulk operation
/// retain the exact pre-write bytes for rollback without reading the file again.
/// Missing files get one final write containing the default plus the update.
pub fn update_info_json_from_snapshot(
    mod_path: &Path,
    previous: Option<&[u8]>,
    update: &ModInfoUpdate,
) -> Result<ModInfo, MetadataError> {
    let mut info = match previous {
        Some(bytes) => serde_json::from_slice(bytes).map_err(|error| {
            MetadataError::Validation(format!("Failed to parse info.json: {error}"))
        })?,
        None => default_info_for_folder(mod_path)?,
    };
    let original_info = info.clone();

    apply_update(&mut info, update);
    if previous.is_some() && info == original_info {
        return Ok(info);
    }

    write_info_json(mod_path, &info)?;
    Ok(info)
}

/// Update specific fields in an existing info.json (merge, not overwrite).
/// Missing files receive a default plus the requested update in one atomic write.
pub fn update_info_json(mod_path: &Path, update: &ModInfoUpdate) -> Result<ModInfo, MetadataError> {
    let info_path = mod_path.join("info.json");
    let previous = match fs::read(&info_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(MetadataError::Io(error.to_string())),
    };

    update_info_json_from_snapshot(mod_path, previous.as_deref(), update)
}

#[cfg(test)]
#[path = "tests/info_json_tests.rs"]
mod tests;
