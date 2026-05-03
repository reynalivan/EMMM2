use crate::services::disk_reconcile::types::{DiskReconcilePathKind, DiskReconcilePathUpdate};

pub(crate) fn push_path_update(
    path_updates: &mut Vec<DiskReconcilePathUpdate>,
    kind: DiskReconcilePathKind,
    from: &str,
    to: &str,
) {
    if from == to {
        return;
    }

    if path_updates
        .iter()
        .any(|entry| entry.kind == kind && entry.from == from && entry.to == to)
    {
        return;
    }

    path_updates.push(DiskReconcilePathUpdate {
        from: from.to_string(),
        to: to.to_string(),
        kind,
    });
}
