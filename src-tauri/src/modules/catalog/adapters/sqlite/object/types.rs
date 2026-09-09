use crate::modules::catalog::domain::objects::{
    CategoryCount, ObjectRuntimeDescriptor, ObjectSummary,
};
use crate::modules::games::domain::models::{CustomSkinsPayload, GameObject, HashDbPayload};

use crate::modules::games::domain::models::ItemStatus;
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row};

fn optional_json<T: serde::de::DeserializeOwned>(
    row: &SqliteRow,
    column: &'static str,
) -> Result<Option<T>, sqlx::Error> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|json| {
            serde_json::from_str(&json).map_err(|error| sqlx::Error::Decode(Box::new(error)))
        })
        .transpose()
}

impl<'r> FromRow<'r, SqliteRow> for ObjectSummary {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            folder_path: row.try_get("folder_path")?,
            matched_entry_key: row.try_get("matched_entry_key")?,
            matched_alias_name: row.try_get("matched_alias_name")?,
            matched_confidence: row.try_get("matched_confidence")?,
            matched_reason: row.try_get("matched_reason")?,
            matched_source: row.try_get("matched_source")?,
            object_type: row.try_get("object_type")?,
            sub_category: row.try_get("sub_category")?,
            status: row.try_get("status")?,
            metadata: row.try_get("metadata")?,
            tags: row.try_get("tags")?,
            hash_db: optional_json(row, "hash_db")?,
            custom_skins: optional_json(row, "custom_skins")?,
            is_pinned: row.try_get("is_pinned")?,
            is_auto_sync: row.try_get("is_auto_sync")?,
            thumbnail_path: row.try_get("thumbnail_path")?,
            created_at: row.try_get("created_at")?,
            mod_count: row.try_get("mod_count")?,
            enabled_count: row.try_get("enabled_count")?,
            safe_mod_count: row.try_get("safe_mod_count")?,
            unsafe_mod_count: row.try_get("unsafe_mod_count")?,
            unclassified_mod_count: row.try_get("unclassified_mod_count")?,
            is_object_disabled: row.try_get("is_object_disabled")?,
            has_naming_conflict: row.try_get("has_naming_conflict")?,
            active_mod_paths: row.try_get("active_mod_paths")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for ObjectRuntimeDescriptor {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            folder_path: row.try_get("folder_path")?,
            folder_path_key: row.try_get("folder_path_key")?,
            matched_entry_key: row.try_get("matched_entry_key")?,
            matched_alias_name: row.try_get("matched_alias_name")?,
            object_type: row.try_get("object_type")?,
            thumbnail_path: row.try_get("thumbnail_path")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for CategoryCount {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            object_type: row.try_get("object_type")?,
            count: row.try_get("count")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for GameObject {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            game_id: row.try_get("game_id")?,
            name: row.try_get("name")?,
            folder_path: row.try_get("folder_path")?,
            folder_path_key: row.try_get("folder_path_key")?,
            status: row.try_get("status")?,
            object_type: row.try_get("object_type")?,
            sub_category: row.try_get("sub_category")?,
            tags: row.try_get("tags")?,
            metadata: row.try_get("metadata")?,
            hash_db: optional_json::<HashDbPayload>(row, "hash_db")?,
            custom_skins: optional_json::<CustomSkinsPayload>(row, "custom_skins")?,
            thumbnail_path: row.try_get("thumbnail_path")?,
            is_pinned: row.try_get("is_pinned")?,
            is_auto_sync: row.try_get("is_auto_sync")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// An `ObjectSummary` plus the one column the listing needs but never returns.
///
/// Flattened rather than re-declared: the 24 columns were spelled out here and
/// then mapped across field by field, twice, in `listing.rs`.
#[derive(Clone, sqlx::FromRow)]
pub(super) struct ObjectSummaryRow {
    #[sqlx(flatten)]
    pub summary: ObjectSummary,
    /// 0 when `object_runtime_projection` has no row for this object yet.
    pub projection_available: i64,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ObjectCountCandidate {
    pub object_id: String,
    pub folder_path: String,
    pub actual_name: String,
    pub status: ItemStatus,
    pub is_safe: bool,
    pub safety_source: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TerminalDescriptor {
    pub display_path: String,
    pub display_segments: Vec<String>,
}

/// Object row shape consumed by disk reconcile.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReconcileObjectRow {
    pub id: String,
    pub name: String,
    pub folder_path: String,
    pub folder_path_key: String,
    pub status: crate::modules::games::domain::models::ItemStatus,
    pub object_type: String,
    pub filesystem_identity: Option<String>,
}

/// A page of objects plus the ids whose runtime projection is cold.
///
/// The repo cannot resolve those itself — filling them in reads the disk.
pub struct ObjectPage {
    pub objects: Vec<ObjectSummary>,
    pub cold_object_ids: Vec<String>,
}
