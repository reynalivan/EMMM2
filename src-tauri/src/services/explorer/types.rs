use serde::Serialize;
use std::collections::HashMap;

// ── Internal types ────────────────────────────────────────────────────────────

pub struct InfoAnalysis {
    pub has_info_json: bool,
    pub is_favorite: bool,
    pub is_misplaced: bool,
    pub is_safe: bool,
    pub metadata: Option<HashMap<String, String>>,
    pub category: Option<String>,
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
    /// Classification: "ContainerFolder" | "ModPackRoot" | "VariantContainer" | "InternalAssets"
    pub node_type: String,
    /// Short diagnostic reasons for the classification (debug/tooltips)
    pub classification_reasons: Vec<String>,
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
    /// Conflict group ID (stable hash of parent + base_name), if in conflict
    pub conflict_group_id: Option<String>,
    /// Conflict state: "EnabledDisabledBothPresent" when both X and DISABLED X exist
    pub conflict_state: Option<String>,
}

/// A single member of a conflict group (for the Resolve dialog).
#[derive(Debug, Clone, Serialize)]
pub struct ConflictMember {
    /// Full absolute path
    pub path: String,
    /// Actual folder name on disk
    pub folder_name: String,
    /// Whether the folder is enabled
    pub is_enabled: bool,
    /// Last modified time (epoch seconds)
    pub modified_at: u64,
    /// Total size in bytes
    pub size_bytes: u64,
}

/// A group of folders sharing the same base name in the same parent directory.
/// Created when both "X" and "DISABLED X" exist on disk.
#[derive(Debug, Clone, Serialize)]
pub struct ConflictGroup {
    /// Stable identifier: hash(parent_dir + base_name)
    pub group_id: String,
    /// The normalized base name (without disabled prefix)
    pub base_name: String,
    /// All member folders in this conflict
    pub members: Vec<ConflictMember>,
}

/// Response payload for `FolderGrid` navigation. Includes both
/// the children folders and data about the navigated folder itself.
#[derive(Debug, Clone, Serialize)]
pub struct FolderGridResponse {
    pub self_node_type: Option<String>,
    pub self_is_mod: bool,
    pub self_is_enabled: bool,
    pub self_classification_reasons: Vec<String>,
    pub children: Vec<ModFolder>,
    /// Conflict groups detected in children (empty if none)
    pub conflicts: Vec<ConflictGroup>,
}
