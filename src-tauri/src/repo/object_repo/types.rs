use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::models::ItemStatus;

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct ObjectFilter {
    pub game_id: String,
    pub search_query: Option<String>,
    pub object_type: Option<String>,
    pub safe_mode: bool,
    pub meta_filters: Option<HashMap<String, Vec<String>>>,
    pub sort_by: Option<String>,
    pub status_filter: Option<ItemStatus>,
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct GetObjectsResult {
    pub objects: Vec<ObjectSummary>,
    pub lost_objects: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct ObjectSummary {
    pub id: String,
    pub name: String,
    pub folder_path: String,
    pub matched_entry_key: Option<String>,
    pub matched_alias_name: Option<String>,
    pub matched_confidence: Option<f64>,
    pub matched_reason: Option<String>,
    pub matched_source: Option<String>,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub status: ItemStatus, // 1: ENABLED, 0: DISABLED
    pub metadata: String,
    pub tags: String,
    pub hash_db: Option<crate::domain::models::HashDbPayload>,
    pub custom_skins: Option<crate::domain::models::CustomSkinsPayload>,
    pub is_pinned: bool,
    pub is_auto_sync: bool,
    pub thumbnail_path: Option<String>,
    pub created_at: Option<String>,
    #[specta(type = f64)]
    pub mod_count: i64,
    #[specta(type = f64)]
    pub enabled_count: i64,
    pub is_object_disabled: bool,
    pub has_naming_conflict: bool,
    pub active_mod_paths: Option<String>,
}

#[derive(Clone, sqlx::FromRow)]
pub(super) struct ObjectSummaryRow {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) folder_path: String,
    pub(super) matched_entry_key: Option<String>,
    pub(super) matched_alias_name: Option<String>,
    pub(super) matched_confidence: Option<f64>,
    pub(super) matched_reason: Option<String>,
    pub(super) matched_source: Option<String>,
    pub(super) object_type: String,
    pub(super) sub_category: Option<String>,
    pub(super) status: ItemStatus,
    pub(super) metadata: String,
    pub(super) tags: String,
    pub(super) hash_db: Option<crate::domain::models::HashDbPayload>,
    pub(super) custom_skins: Option<crate::domain::models::CustomSkinsPayload>,
    pub(super) is_pinned: bool,
    pub(super) is_auto_sync: bool,
    pub(super) thumbnail_path: Option<String>,
    pub(super) created_at: Option<String>,
    pub(super) mod_count: i64,
    pub(super) enabled_count: i64,
    pub(super) is_object_disabled: bool,
    pub(super) has_naming_conflict: bool,
    pub(super) active_mod_paths: Option<String>,
    pub(super) projection_available: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct ObjectRuntimeDescriptor {
    pub id: String,
    pub name: String,
    pub folder_path: String,
    pub folder_path_key: String,
    pub matched_entry_key: Option<String>,
    pub matched_alias_name: Option<String>,
    pub object_type: String,
    pub thumbnail_path: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
pub struct CategoryCount {
    pub object_type: String,
    #[specta(type = f64)]
    pub count: i64,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub(super) struct ObjectCountCandidate {
    pub(super) object_id: String,
    pub(super) folder_path: String,
    pub(super) actual_name: String,
    pub(super) status: ItemStatus,
}

#[derive(Clone, Debug)]
pub(super) struct TerminalDescriptor {
    pub(super) display_path: String,
    pub(super) display_segments: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct CreateObjectInput {
    pub game_id: String,
    pub name: String,
    pub folder_path: Option<String>,
    pub object_type: String,
    pub sub_category: Option<String>,
    pub status: Option<ItemStatus>,
    pub metadata: Option<serde_json::Value>,
    pub thumbnail_url: Option<String>,
    pub hash_db: Option<crate::domain::models::HashDbPayload>,
    pub custom_skins: Option<crate::domain::models::CustomSkinsPayload>,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct UpdateObjectInput {
    pub name: Option<String>,
    pub object_type: Option<String>,
    pub sub_category: Option<String>,
    pub status: Option<ItemStatus>,
    pub metadata: Option<serde_json::Value>,
    pub hash_db: Option<crate::domain::models::HashDbPayload>,
    pub custom_skins: Option<crate::domain::models::CustomSkinsPayload>,
    pub thumbnail_path: Option<String>,
    pub is_auto_sync: Option<bool>,
    pub is_pinned: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// Object row shape consumed by disk reconcile.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReconcileObjectRow {
    pub id: String,
    pub folder_path: String,
    pub folder_path_key: String,
    pub status: crate::domain::models::ItemStatus,
    pub object_type: String,
}

pub struct EnsureObjectInput<'a> {
    pub game_id: &'a str,
    pub folder_path: &'a str,
    pub obj_name: &'a str,
    pub obj_type: &'a str,
    pub db_thumbnail: Option<&'a str>,
    pub db_tags_json: &'a str,
    pub db_metadata_json: &'a str,
    pub db_hash_db_json: Option<&'a str>,
    pub db_custom_skins_json: Option<&'a str>,
}
