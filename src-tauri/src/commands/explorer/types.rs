use serde::Serialize;
use std::collections::HashMap;

// ── Internal types ────────────────────────────────────────────────────────────

pub(crate) struct InfoAnalysis {
    pub(crate) has_info_json: bool,
    pub(crate) is_favorite: bool,
    pub(crate) is_misplaced: bool,
    pub(crate) is_safe: bool,
    pub(crate) metadata: Option<HashMap<String, String>>,
    pub(crate) category: Option<String>,
}

impl Default for InfoAnalysis {
    fn default() -> Self {
        Self {
            has_info_json: false,
            is_favorite: false,
            is_misplaced: false,
            is_safe: true,
            metadata: None,
            category: None,
        }
    }
}

// ── Public types ──────────────────────────────────────────────────────────────

/// Represents a single mod folder entry from the filesystem.
#[derive(Debug, Clone, Serialize)]
pub struct ModFolder {
    /// Database ID (UUID), if available
    pub id: Option<String>,
    /// Display name (without "DISABLED " prefix)
    pub name: String,
    /// Actual folder name on disk
    pub folder_name: String,
    /// Full absolute path
    pub path: String,
    /// Whether the mod is enabled (no "DISABLED " prefix)
    pub is_enabled: bool,
    /// Whether this entry is a directory (vs a file)
    pub is_directory: bool,
    /// Discovered thumbnail image path (if any)
    pub thumbnail_path: Option<String>,
    /// Last modified time (epoch seconds)
    pub modified_at: u64,
    /// Total size in bytes (shallow for directories)
    pub size_bytes: u64,
    /// Whether the folder contains an info.json file
    pub has_info_json: bool,
    /// Whether the mod is marked as favorite
    pub is_favorite: bool,
    /// Whether the mod appears to be in the wrong category (Basic Heuristic)
    pub is_misplaced: bool,
    /// Whether the mod is marked as safe (from info.json)
    pub is_safe: bool,
    /// Metadata from info.json (element, rarity, etc.)
    pub metadata: Option<HashMap<String, String>>,
    /// Category from info.json metadata
    pub category: Option<String>,
}
