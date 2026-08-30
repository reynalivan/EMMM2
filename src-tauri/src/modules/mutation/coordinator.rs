use std::sync::Arc;
use crate::app::runtime::operation_journal::{OperationJournal, OperationStatus};
use crate::platform::fs::operation_lock::{OperationLock, OpGuard};
use crate::shared::errors::AppError;

pub struct MutationCoordinator {
    lock: OperationLock,
    journal: Arc<OperationJournal>,
}

impl MutationCoordinator {
    pub fn new(journal: Arc<OperationJournal>) -> Self {
        Self {
            lock: OperationLock::new(),
            journal,
        }
    }

    pub async fn acquire(&self) -> Result<MutationGuard, AppError> {
        let guard = self.lock.acquire().await?;
        Ok(MutationGuard {
            guard,
            journal: self.journal.clone(),
        })
    }

    pub fn inner_lock(&self) -> &OperationLock {
        &self.lock
    }
}

pub struct MutationGuard {
    guard: OpGuard,
    journal: Arc<OperationJournal>,
}

impl MutationGuard {
    pub async fn begin_operation(&self, plan: String) -> Result<String, AppError> {
        self.journal.write(plan, OperationStatus::Pending).await
    }
    
    pub async fn complete_operation(&self, operation_id: &str) -> Result<(), AppError> {
        self.journal.update_status(operation_id, OperationStatus::Completed).await
    }

    pub fn op_guard(&self) -> &OpGuard {
        &self.guard
    }
}
