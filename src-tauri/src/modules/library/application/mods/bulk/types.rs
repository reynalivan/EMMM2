//! Shared payload and result types for bulk mod operations.

use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct BulkProgressPayload {
    pub operation_id: String,
    pub cancellable: bool,
    pub label: String,
    #[specta(type = f64)]
    pub current: usize,
    #[specta(type = f64)]
    pub total: usize,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct BulkActionError {
    pub path: String,
    pub error: crate::shared::errors::AppError,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct BulkResult {
    pub success: Vec<String>,
    pub failures: Vec<BulkActionError>,
    pub cancelled: bool,
    #[specta(type = f64)]
    pub processed_count: usize,
    #[specta(type = f64)]
    pub unprocessed_count: usize,
    pub collection_impact: CollectionReferenceImpact,
    pub path_rewrites: Vec<WorkspacePathRewrite>,
    pub sync_warning: Option<crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning>,
    pub runtime_sync_generation: Option<u64>,
}

impl BulkResult {
    pub fn new(success: Vec<String>, failures: Vec<BulkActionError>) -> Self {
        let processed_count = success.len() + failures.len();
        Self {
            success,
            failures,
            cancelled: false,
            processed_count,
            unprocessed_count: 0,
            collection_impact: CollectionReferenceImpact::default(),
            path_rewrites: Vec::new(),
            sync_warning: None,
            runtime_sync_generation: None,
        }
    }

    pub fn with_collection_impact(
        success: Vec<String>,
        failures: Vec<BulkActionError>,
        collection_impact: CollectionReferenceImpact,
        path_rewrites: Vec<WorkspacePathRewrite>,
    ) -> Self {
        let processed_count = success.len() + failures.len();
        Self {
            success,
            failures,
            cancelled: false,
            processed_count,
            unprocessed_count: 0,
            collection_impact,
            path_rewrites,
            sync_warning: None,
            runtime_sync_generation: None,
        }
    }

    pub fn with_execution_state(
        mut self,
        cancelled: bool,
        processed_count: usize,
        total_count: usize,
    ) -> Self {
        self.cancelled = cancelled;
        self.processed_count = processed_count;
        self.unprocessed_count = total_count.saturating_sub(processed_count);
        self
    }

    pub fn with_runtime_sync_generation(mut self, generation: u64) -> Self {
        self.runtime_sync_generation = Some(generation);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_bulk_result_has_no_sync_warning_before_terminal_reconcile() {
        assert!(BulkResult::new(Vec::new(), Vec::new())
            .sync_warning
            .is_none());
    }

    #[test]
    fn execution_state_reports_the_unprocessed_tail_after_cancellation() {
        let result =
            BulkResult::new(vec!["done".to_string()], Vec::new()).with_execution_state(true, 1, 3);

        assert!(result.cancelled);
        assert_eq!(result.processed_count, 1);
        assert_eq!(result.unprocessed_count, 2);
    }
}
