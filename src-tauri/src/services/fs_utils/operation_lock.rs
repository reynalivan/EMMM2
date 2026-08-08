//! Global Operation Lock for destructive file operations.
//!
//! Prevents concurrent toggle/rename/import/delete operations to avoid data
//! corruption. Wraps `tokio::sync::Mutex` with a short acquisition timeout so
//! contention surfaces as a retryable error instead of a hang.
//!
//! The lock is **not** reentrant: exactly one function in any call chain may
//! acquire it. Orchestrators that call an acquiring service must not acquire.
//!
//! # Covers: TRD §3.6, NC-5.1-04, EC-5.01

use crate::domain::errors::AppError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OwnedMutexGuard};

/// How long `acquire` waits before reporting contention.
const ACQUIRE_TIMEOUT: Duration = Duration::from_millis(500);

const CONTENTION_MESSAGE: &str = "Operation in progress. Please wait a moment and try again.";

/// Proof that the operation lock is held.
///
/// The field is private and only [`OperationLock::acquire`] constructs one, so
/// a service that takes `&OpGuard` cannot run without the lock — and cannot
/// re-acquire it, which is what used to deadlock: the lock is not reentrant,
/// and whether a callee acquired internally was knowable only by reading it.
/// Entry points (commands, hotkey handlers, queue workers) acquire; everything
/// below them takes the guard.
#[derive(Debug)]
pub struct OpGuard(#[allow(dead_code)] OwnedMutexGuard<()>);

/// Global lock for destructive file operations.
pub struct OperationLock {
    lock: Arc<Mutex<()>>,
}

impl OperationLock {
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Try to acquire the lock, giving up after [`ACQUIRE_TIMEOUT`].
    /// Every caller surfaces contention as the same `AppError::Io`.
    pub async fn acquire(&self) -> Result<OpGuard, AppError> {
        tokio::time::timeout(ACQUIRE_TIMEOUT, self.lock.clone().lock_owned())
            .await
            .map(OpGuard)
            .map_err(|_| AppError::Io(CONTENTION_MESSAGE.to_string()))
    }
}

impl Default for OperationLock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "tests/operation_lock_tests.rs"]
mod tests;
