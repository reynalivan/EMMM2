//! Mutable accumulators threaded through the projection write passes.

use std::collections::HashSet;

use crate::domain::collection::CollectionReferenceImpact;
use crate::services::disk_reconcile::change_summary::ChangeSummaryBuilder;
use crate::services::disk_reconcile::types::DiskReconcilePathUpdate;

pub(super) struct ProjectionWriteState<'a> {
    pub(super) path_updates: &'a mut Vec<DiskReconcilePathUpdate>,
    pub(super) collection_reference_impact: &'a mut CollectionReferenceImpact,
    pub(super) change_summary: &'a mut ChangeSummaryBuilder,
    pub(super) seen_object_keys: HashSet<String>,
    pub(super) seen_mod_keys: HashSet<String>,
    pub(super) deleted_object_keys: HashSet<String>,
    pub(super) touched_object_ids: HashSet<String>,
    pub(super) objects_changed: bool,
    pub(super) folders_changed: bool,
}
