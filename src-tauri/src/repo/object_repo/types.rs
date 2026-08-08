use crate::domain::objects::ObjectSummary;

use crate::domain::models::ItemStatus;

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
    pub folder_path: String,
    pub folder_path_key: String,
    pub status: crate::domain::models::ItemStatus,
    pub object_type: String,
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
    pub fn type_is_authoritative(self) -> bool {
        matches!(self, Self::MasterDb)
    }
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
    pub source: MatchSource,
}

/// A page of objects plus the ids whose runtime projection is cold.
///
/// The repo cannot resolve those itself — filling them in reads the disk.
pub struct ObjectPage {
    pub objects: Vec<ObjectSummary>,
    pub cold_object_ids: Vec<String>,
}
