//! Delete-result payloads.

use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DeleteModResult {
    pub collection_impact: CollectionReferenceImpact,
    pub sync_warning: Option<crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_result_can_report_committed_sync_warning() {
        let result = DeleteModResult {
            collection_impact: CollectionReferenceImpact::default(),
            sync_warning: Some(
                crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning {
                    kind: crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarningKind::ReconcileFailed,
                    message: "retry projection".to_string(),
                },
            ),
        };

        assert!(result.sync_warning.is_some());
    }
}
