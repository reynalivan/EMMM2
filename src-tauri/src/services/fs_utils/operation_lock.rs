//! Global Operation Lock for destructive file operations.
//!
//! Prevents concurrent toggle/rename/import/delete operations
//! to avoid data corruption. Uses tokio::sync::Mutex with 30s timeout.
//!
//! # Covers: TRD §3.6, NC-5.1-04, EC-5.01

use crate::domain::errors::AppError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OwnedMutexGuard};

/// Global lock for destructive file operations.
/// Acquired at the Command layer to keep services reusable.
pub struct OperationLock {
    lock: Arc<Mutex<()>>,
}

impl OperationLock {
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Mutex::new(())),
        }
    }
}

impl Default for OperationLock {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationLock {
    /// Try to acquire the lock with a 500ms timeout.
    /// Every caller surfaces contention as the same `AppError::Io`.
    pub async fn acquire(&self) -> Result<OwnedMutexGuard<()>, AppError> {
        match tokio::time::timeout(Duration::from_millis(500), self.lock.clone().lock_owned()).await
        {
            Ok(guard) => Ok(guard),
            Err(_) => Err(AppError::Io(
                "Operation in progress. Please wait a moment and try again.".to_string(),
            )),
        }
    }
}

#[cfg(test)]
#[path = "tests/operation_lock_tests.rs"]
mod tests;
