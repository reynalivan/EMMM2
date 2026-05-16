use serde::{Deserialize, Serialize};

use crate::domain::workspace::WorkspacePathRewrite;

// ---------------------------------------------------------------------------
// Collection — A named loadout snapshot
// ---------------------------------------------------------------------------

/// The kind of collection. Replaces the `is_last_unsaved` boolean flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, specta::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CollectionKind {
    Named,
    UndoSnapshot,
    Unsaved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PreviewTreeNodeKind {
    Object,
    Folder,
    Mod,
}

impl CollectionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Named => "named",
            Self::UndoSnapshot => "undo_snapshot",
            Self::Unsaved => "unsaved",
        }
    }

    pub fn from_db_value(s: &str) -> Self {
        match s {
            "undo_snapshot" => Self::UndoSnapshot,
            "unsaved" => Self::Unsaved,
            _ => Self::Named,
        }
    }
}

/// The kind of member in a collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, specta::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MemberKind {
    Mod,
    Nested,
    Object,
    Root,
}

impl MemberKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Nested => "nested",
            Self::Object => "object",
            Self::Root => "root",
        }
    }

    pub fn from_db_value(s: &str) -> Self {
        match s {
            "nested" => Self::Nested,
            "object" => Self::Object,
            "root" => Self::Root,
            _ => Self::Mod,
        }
    }
}

/// Full collection row from the `collections` table.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Collection {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub name_key: String,
    pub is_safe: bool,
    pub is_unsaved: bool,
    pub is_last_unsaved: bool,
    pub last_active: bool,
    pub snapshot_json: Option<String>,
    pub signature: Option<String>,
    pub root_count: i32,
    pub display_mod_count: i32,
    pub member_count: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

/// Summary returned in list views.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CollectionSummary {
    pub id: String,
    pub name: String,
    pub is_safe: bool,
    pub is_unsaved: bool,
    pub is_active: bool,      // Derived from corridor_state
    pub is_undo_target: bool, // Derived from corridor_state
    pub signature: Option<String>,
    pub updated_at: String,
    pub raw_member_count: i32,
    pub mod_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ProjectedObjectState {
    pub object_id: String,
    pub display_name: String,
    pub path_key: String,
    pub is_enabled: bool,
    #[specta(type = f64)]
    pub active_root_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ProjectedActiveRoot {
    pub object_id: String,
    pub root_key: String,
    pub display_name: String,
    pub root_type: String,
    pub source_path: String,
    pub thumbnail_hint: Option<String>,
    pub warnings: Vec<String>,
    pub is_missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ProjectedStateSummary {
    #[specta(type = f64)]
    pub object_count: usize,
    #[specta(type = f64)]
    pub enabled_object_count: usize,
    #[specta(type = f64)]
    pub active_root_count: usize,
    #[specta(type = f64)]
    pub missing_root_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ProjectedCollectionState {
    pub object_states: Vec<ProjectedObjectState>,
    pub active_roots: Vec<ProjectedActiveRoot>,
    pub summary: ProjectedStateSummary,
}

/// A single mod member of a collection (from `collection_mods`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CollectionMod {
    pub kind: MemberKind,
    pub collection_id: String,
    pub mod_id: Option<String>,
    pub mod_path: String,
    pub mod_path_key: Option<String>,
    pub object_id: String,
    pub display_name: Option<String>,
    pub preview_path: Option<String>,
    pub node_type: Option<String>,
    pub warnings: Vec<String>,
    pub is_enabled: bool,
}

/// A single object member of a collection (from `collection_objects`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct CollectionObject {
    pub kind: MemberKind,
    pub collection_id: String,
    pub object_id: String,
    pub is_enabled: bool,
    pub display_name: Option<String>,
    pub path_key: Option<String>,
}

/// A root entry (from `collection_roots`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct CollectionRoot {
    pub kind: MemberKind,
    pub collection_id: String,
    pub root_path: String,
    pub root_path_key: String,
    pub display_name: String,
    pub display_name_key: String,
    pub object_id: Option<String>,
    pub object_name: Option<String>,
    pub object_type: Option<String>,
    pub root_kind: String,
    pub is_safe: bool,
    pub is_enabled: bool,
    pub thumbnail_hint: Option<String>,
    pub corridor_source: Option<String>,
}

/// A unified member type for collections.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CollectionMember {
    Mod(CollectionMod),
    Object(CollectionObject),
    Root(CollectionRoot),
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PreviewTreeNode {
    pub kind: PreviewTreeNodeKind,
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub object_id: Option<String>,
    pub node_type: Option<String>,
    pub is_enabled: bool,
    pub is_effectively_active: bool,
    pub inactive_reason: Option<String>,
    pub show_inactive_chip: bool,
    pub status_kind: Option<String>,
    pub collapse_children: bool,
    pub warnings: Vec<String>,
    #[specta(type = Option<f64>)]
    pub mod_count: Option<usize>,
    pub children: Vec<PreviewTreeNode>,
}

/// Preview data for a collection.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CollectionPreview {
    pub collection: CollectionSummary,
    pub members: Vec<CollectionMember>,
    pub mods: Vec<CollectionMod>,
    pub objects: Vec<CollectionObject>,
    pub roots: Vec<CollectionRoot>,
    pub tree_nodes: Vec<PreviewTreeNode>,
    pub projected_state: ProjectedCollectionState,
}

/// Result of applying a collection.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ApplyResult {
    pub success: bool,
    #[specta(type = f64)]
    pub mods_enabled: usize,
    #[specta(type = f64)]
    pub mods_disabled: usize,
    #[specta(type = f64)]
    pub objects_toggled: usize,
    pub undo_collection_id: Option<String>,
    pub new_signature: String,
    pub warnings: Vec<String>,
    pub final_state_name: Option<String>,
    pub final_mode: Option<String>,
    pub partial_apply: bool,
    pub skipped_missing_paths: Vec<String>,
    pub final_state_is_dirty: bool,
    pub runtime_path_rewrites: Vec<WorkspacePathRewrite>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct CollectionPathRewrite {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct CollectionReferenceImpact {
    #[specta(type = f64)]
    pub affected_collection_count: usize,
    pub affected_collection_names: Vec<String>,
    pub rewritten_paths: Vec<CollectionPathRewrite>,
    pub missing_paths: Vec<String>,
}

impl CollectionReferenceImpact {
    pub fn merge(&mut self, next: Self) {
        for name in next.affected_collection_names {
            if !self
                .affected_collection_names
                .iter()
                .any(|existing| existing == &name)
            {
                self.affected_collection_names.push(name);
            }
        }
        for path in next.missing_paths {
            if !self.missing_paths.iter().any(|existing| existing == &path) {
                self.missing_paths.push(path);
            }
        }
        self.rewritten_paths.extend(next.rewritten_paths);
        self.affected_collection_count = self.affected_collection_names.len();
    }
}

/// Input for creating a new collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CreateCollectionMode {
    SaveCurrentState,
    CloneSnapshot,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct CreateCollectionInput {
    pub game_id: String,
    pub name: String,
    pub is_safe: bool,
    pub save_mode: Option<CreateCollectionMode>,
    pub source_collection_id: Option<String>,
}

/// Input for updating an existing collection.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct UpdateCollectionInput {
    pub id: String,
    pub game_id: String,
    pub name: Option<String>,
}

/// Preview data for applying a collection (before → after).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ApplyPreview {
    pub collection_name: String,
    pub current_snapshot: Option<String>,
    pub current_mods: Vec<CollectionMod>,
    pub current_objects: Vec<CollectionObject>,
    pub current_tree_nodes: Vec<PreviewTreeNode>,
    pub target_mods: Vec<CollectionMod>,
    pub target_objects: Vec<CollectionObject>,
    pub target_tree_nodes: Vec<PreviewTreeNode>,
    pub current_state_name: Option<String>,
    pub current_state_is_unsaved: bool,
    pub current_projected_state: ProjectedCollectionState,
    pub target_projected_state: ProjectedCollectionState,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ApplyProgressSnapshot {
    pub game_id: String,
    pub is_safe: bool,
    pub phase: String,
    #[specta(type = f64)]
    pub completed: usize,
    #[specta(type = f64)]
    pub total: usize,
    pub current_item: Option<String>,
    pub warnings: Vec<String>,
    pub final_state_name: Option<String>,
    pub final_mode: Option<String>,
    pub success: bool,
}
