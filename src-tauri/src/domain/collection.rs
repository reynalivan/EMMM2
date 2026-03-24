use serde::{Deserialize, Serialize};

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

impl CollectionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Named => "named",
            Self::UndoSnapshot => "undo_snapshot",
            Self::Unsaved => "unsaved",
        }
    }

    pub fn from_str(s: &str) -> Self {
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

    pub fn from_str(s: &str) -> Self {
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
    pub member_count: i32,
}

/// A single mod member of a collection (from `collection_mods`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct CollectionMod {
    pub kind: MemberKind,
    pub collection_id: String,
    pub mod_id: Option<String>,
    pub mod_path: String,
    pub mod_path_key: Option<String>,
    pub object_id: String,
    pub display_name: Option<String>,
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

/// Preview data for a collection.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CollectionPreview {
    pub collection: CollectionSummary,
    pub members: Vec<CollectionMember>,
    pub mods: Vec<CollectionMod>,
    pub objects: Vec<CollectionObject>,
    pub roots: Vec<CollectionRoot>,
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
}

/// Input for creating a new collection.
#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct CreateCollectionInput {
    pub game_id: String,
    pub name: String,
    pub is_safe: bool,
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
    pub target_mods: Vec<CollectionMod>,
    pub target_objects: Vec<CollectionObject>,
}
