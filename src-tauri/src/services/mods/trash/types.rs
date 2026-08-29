//! Delete-result payloads.

use crate::domain::collection::CollectionReferenceImpact;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DeleteModResult {
    pub collection_impact: CollectionReferenceImpact,
    pub sync_warning: Option<crate::services::disk_reconcile::types::CommittedMutationSyncWarning>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_result_can_report_committed_sync_warning() {
        let result = DeleteModResult {
            collection_impact: CollectionReferenceImpact::default(),
            sync_warning: Some(
                crate::services::disk_reconcile::types::CommittedMutationSyncWarning {
                    kind: crate::services::disk_reconcile::types::CommittedMutationSyncWarningKind::ReconcileFailed,
                    message: "retry projection".to_string(),
                },
            ),
        };

        assert!(result.sync_warning.is_some());
    }
}
