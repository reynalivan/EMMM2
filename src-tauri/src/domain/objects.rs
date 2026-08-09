//! Object vocabulary shared by the repo, the services and the frontend.
//!
//! These carry `specta::Type`, so they are the TypeScript contract. They lived
//! in `repo::object_repo`, which made the data-access layer the owner of the
//! IPC surface. `sqlx::FromRow` stays on them: the row shape and the wire
//! shape genuinely coincide today, and a second type per table would be
//! ceremony until one of them actually diverges.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::models::ItemStatus;

/// `Default` is the unfiltered, safe-mode-off query. Callers spell out only
/// the axes they actually constrain — the full seven-field literal was written
/// out at twenty sites.
#[derive(Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct ObjectFilter {
    pub game_id: String,
    pub search_query: Option<String>,
    pub object_type: Option<String>,
    /// Derived server-side (`ConfigService::current_corridor`) at the command
    /// boundary. Serde/specta-skipped so the corridor cannot arrive over IPC.
    #[serde(skip)]
    #[specta(skip)]
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
    pub metadata: Option<serde_json::Value>,
    pub hash_db: Option<crate::domain::models::HashDbPayload>,
    pub custom_skins: Option<crate::domain::models::CustomSkinsPayload>,
    pub thumbnail_path: Option<String>,
    pub is_auto_sync: Option<bool>,
    pub is_pinned: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// Where the object being synced was identified.
///
/// Replaces `db_thumbnail.is_some()`, which the sync used as a stand-in for
/// "MasterDB matched this, so trust its name and type" — a rule that silently
/// changed meaning for any MasterDB entry without a thumbnail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchSource {
    /// Matched against the bundled MasterDB; its name and type are canonical.
    MasterDb,
    /// Discovered by walking the mods folder; the folder name is all we have.
    Disk,
}

impl MatchSource {
    /// Whether this source's `object_type` should overwrite what the row holds.
    ///
    /// This is a merge rule, not a column: it lives with the vocabulary rather
    /// than in the repo, which only knows how to run the UPDATE.
    pub fn type_is_authoritative(self) -> bool {
        matches!(self, Self::MasterDb)
    }
}

/// One folder on disk, as offered to identity resolution.
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
    pub source: MatchSource,
}
