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

use crate::shared::errors::AppError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify, OwnedMutexGuard};

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
#[derive(Clone)]
pub struct OperationLock {
    lock: Arc<Mutex<()>>,
    foreground_waiters: Arc<AtomicUsize>,
    foreground_requested: Arc<Notify>,
}

impl OperationLock {
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Mutex::new(())),
            foreground_waiters: Arc::new(AtomicUsize::new(0)),
            foreground_requested: Arc::new(Notify::new()),
        }
    }

    /// Try to acquire the lock, giving up after [`ACQUIRE_TIMEOUT`].
    /// Every caller surfaces contention as the same `AppError::Io`.
    pub async fn acquire(&self) -> Result<OpGuard, AppError> {
        let _waiter = self.foreground_intent();
        tokio::time::timeout(ACQUIRE_TIMEOUT, self.lock.clone().lock_owned())
            .await
            .map(OpGuard)
            .map_err(|_| AppError::Io(CONTENTION_MESSAGE.to_string()))
    }

    pub async fn acquire_for_reconcile(&self) -> OpGuard {
        let _waiter = self.foreground_intent();
        OpGuard(self.lock.clone().lock_owned().await)
    }

    /// Non-blocking acquisition for best-effort background reconciliation.
    /// Foreground mutations retain priority: a busy or queued lock causes the
    /// background caller to skip its work instead of joining the wait queue.
    pub(crate) fn try_acquire_for_reconcile(&self) -> Option<OpGuard> {
        if self.foreground_waiters.load(Ordering::Acquire) > 0 {
            return None;
        }
        let guard = self.lock.clone().try_lock_owned().ok().map(OpGuard)?;
        if self.foreground_waiters.load(Ordering::Acquire) > 0 {
            return None;
        }
        Some(guard)
    }

    pub(crate) async fn wait_for_foreground_intent(&self) {
        loop {
            let requested = self.foreground_requested.notified();
            if self.foreground_waiters.load(Ordering::Acquire) > 0 {
                return;
            }
            requested.await;
        }
    }

    pub(crate) fn foreground_intent(&self) -> ForegroundIntentGuard {
        self.foreground_waiters.fetch_add(1, Ordering::AcqRel);
        self.foreground_requested.notify_waiters();
        ForegroundIntentGuard {
            waiters: Arc::clone(&self.foreground_waiters),
        }
    }
}

pub(crate) struct ForegroundIntentGuard {
    waiters: Arc<AtomicUsize>,
}

impl Drop for ForegroundIntentGuard {
    fn drop(&mut self) {
        self.waiters.fetch_sub(1, Ordering::AcqRel);
    }
}

impl Default for OperationLock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod background_reconcile_tests {
    use super::*;

    #[tokio::test]
    async fn background_reconcile_try_lock_skips_foreground_contention() {
        let lock = OperationLock::new();
        let _foreground = lock.acquire().await.expect("foreground lock");

        assert!(lock.try_acquire_for_reconcile().is_none());
    }

    #[tokio::test]
    async fn foreground_intent_interrupts_a_running_background_lease() {
        let lock = OperationLock::new();
        let background = lock.try_acquire_for_reconcile().expect("background lock");
        let foreground_lock = lock.clone();
        let foreground = tokio::spawn(async move { foreground_lock.acquire().await });

        tokio::time::timeout(
            Duration::from_millis(100),
            lock.wait_for_foreground_intent(),
        )
        .await
        .expect("foreground intent should wake background");
        drop(background);

        foreground
            .await
            .expect("foreground task")
            .expect("foreground lock");
    }
}

#[cfg(test)]
#[path = "tests/operation_lock_tests.rs"]
mod tests;
