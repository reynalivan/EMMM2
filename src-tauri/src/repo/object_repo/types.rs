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

/// A page of objects plus the ids whose runtime projection is cold.
///
/// The repo cannot resolve those itself — filling them in reads the disk.
pub struct ObjectPage {
    pub objects: Vec<ObjectSummary>,
    pub cold_object_ids: Vec<String>,
}
