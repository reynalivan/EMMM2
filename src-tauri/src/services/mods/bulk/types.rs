//! Shared payload and result types for bulk mod operations.

use crate::domain::collection::CollectionReferenceImpact;
use crate::domain::workspace::WorkspacePathRewrite;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct BulkProgressPayload {
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
    pub error: crate::domain::errors::AppError,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct BulkResult {
    pub success: Vec<String>,
    pub failures: Vec<BulkActionError>,
    pub collection_impact: CollectionReferenceImpact,
    pub path_rewrites: Vec<WorkspacePathRewrite>,
}

impl BulkResult {
    pub fn new(success: Vec<String>, failures: Vec<BulkActionError>) -> Self {
        Self {
            success,
            failures,
            collection_impact: CollectionReferenceImpact::default(),
            path_rewrites: Vec::new(),
        }
    }

    pub fn with_collection_impact(
        success: Vec<String>,
        failures: Vec<BulkActionError>,
        collection_impact: CollectionReferenceImpact,
        path_rewrites: Vec<WorkspacePathRewrite>,
    ) -> Self {
        Self {
            success,
            failures,
            collection_impact,
            path_rewrites,
        }
    }
}
