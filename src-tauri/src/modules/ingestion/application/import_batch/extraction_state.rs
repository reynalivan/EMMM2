use crate::shared::errors::AppError;
use crate::shared::sync::lock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

type OperationMap = Arc<Mutex<HashMap<String, Arc<ImportExtractionOperation>>>>;

#[derive(Default)]
pub struct ImportExtractionState {
    operations: OperationMap,
}

impl ImportExtractionState {
    pub fn acquire(&self, batch_id: &str) -> Result<ImportExtractionLease, AppError> {
        let mut operations = lock(&self.operations);
        if let Some(operation) = operations.get(batch_id) {
            if operation.cancel_token.load(Ordering::SeqCst) {
                return Err(AppError::Cancelled);
            }
            if operation.active.load(Ordering::Acquire) {
                return Err(AppError::Validation(format!(
                    "Import batch '{batch_id}' is already being analyzed"
                )));
            }
        }

        let operation = Arc::new(ImportExtractionOperation::active());
        operations.insert(batch_id.to_string(), operation.clone());
        Ok(ImportExtractionLease {
            batch_id: batch_id.to_string(),
            operation,
            operations: self.operations.clone(),
        })
    }

    pub async fn cancel_and_wait(&self, batch_id: &str) -> ImportCancellationLease {
        let operation = {
            let mut operations = lock(&self.operations);
            let operation = operations
                .entry(batch_id.to_string())
                .or_insert_with(|| Arc::new(ImportExtractionOperation::inactive()))
                .clone();
            operation.cancel_token.store(true, Ordering::SeqCst);
            operation
        };

        loop {
            let completed = operation.completed.notified();
            if !operation.active.load(Ordering::Acquire) {
                break;
            }
            completed.await;
        }

        ImportCancellationLease {
            batch_id: batch_id.to_string(),
            operation,
            operations: self.operations.clone(),
        }
    }
}

struct ImportExtractionOperation {
    active: AtomicBool,
    cancel_token: Arc<AtomicBool>,
    completed: Notify,
}

impl ImportExtractionOperation {
    fn active() -> Self {
        Self {
            active: AtomicBool::new(true),
            cancel_token: Arc::new(AtomicBool::new(false)),
            completed: Notify::new(),
        }
    }

    fn inactive() -> Self {
        Self {
            active: AtomicBool::new(false),
            cancel_token: Arc::new(AtomicBool::new(false)),
            completed: Notify::new(),
        }
    }
}

pub struct ImportExtractionLease {
    batch_id: String,
    operation: Arc<ImportExtractionOperation>,
    operations: OperationMap,
}

impl ImportExtractionLease {
    pub fn cancel_token(&self) -> Arc<AtomicBool> {
        self.operation.cancel_token.clone()
    }
}

impl Drop for ImportExtractionLease {
    fn drop(&mut self) {
        self.operation.active.store(false, Ordering::Release);
        self.operation.completed.notify_waiters();
        if !self.operation.cancel_token.load(Ordering::SeqCst) {
            remove_current_operation(&self.operations, &self.batch_id, &self.operation);
        }
    }
}

pub struct ImportCancellationLease {
    batch_id: String,
    operation: Arc<ImportExtractionOperation>,
    operations: OperationMap,
}

impl Drop for ImportCancellationLease {
    fn drop(&mut self) {
        remove_current_operation(&self.operations, &self.batch_id, &self.operation);
    }
}

fn remove_current_operation(
    operations: &OperationMap,
    batch_id: &str,
    expected: &Arc<ImportExtractionOperation>,
) {
    let mut operations = lock(operations);
    if operations
        .get(batch_id)
        .is_some_and(|current| Arc::ptr_eq(current, expected))
    {
        operations.remove(batch_id);
    }
}
