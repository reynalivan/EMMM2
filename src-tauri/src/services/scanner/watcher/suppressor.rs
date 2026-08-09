//! Watcher suppression.
//!
//! Two mechanisms:
//! - **Blanket** (`SuppressionGuard`, manual frontend flag): drops every event
//!   while held. For broad operations (deep scan, archive extraction) whose
//!   write set is unknown up front. Self-healing: those flows end with a full
//!   reconcile that re-reads disk anyway.
//! - **Path-scoped** (`PathSuppressionGuard`): drops only events under the
//!   registered paths, matched by identity key — so registering either the
//!   enabled or DISABLED spelling covers both sides of a toggle rename. On
//!   drop the entry lingers for `SUPPRESSION_TAIL` to absorb OS events that
//!   were already queued asynchronously (the old guard-dropped-too-early echo
//!   bug). External events elsewhere keep flowing during the operation.

use crate::common::sync::lock;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a scoped entry keeps suppressing after its guard drops.
/// ReadDirectoryChangesW delivers callbacks asynchronously; events for a
/// rename the app just performed can arrive well after the mutation returns.
const SUPPRESSION_TAIL: Duration = Duration::from_secs(2);

struct ScopedEntry {
    id: u64,
    /// Identity key of the suppressed root (see `path_key`).
    key: String,
    /// `None` while the guard is alive; a deadline once it dropped.
    expires_at: Option<Instant>,
}

pub struct WatcherSuppressor {
    guard_depth: AtomicUsize,
    manual_depth: AtomicUsize,
    next_id: AtomicU64,
    scoped: Mutex<Vec<ScopedEntry>>,
}

impl WatcherSuppressor {
    pub fn new(suppressed: bool) -> Self {
        Self {
            guard_depth: AtomicUsize::new(0),
            manual_depth: AtomicUsize::new(if suppressed { 1 } else { 0 }),
            next_id: AtomicU64::new(0),
            scoped: Mutex::new(Vec::new()),
        }
    }

    pub fn load(&self, ordering: Ordering) -> bool {
        self.guard_depth.load(ordering) + self.manual_depth.load(ordering) > 0
    }

    pub fn store(&self, suppressed: bool, ordering: Ordering) {
        if suppressed {
            self.manual_depth.fetch_add(1, ordering);
            return;
        }

        let _ = self
            .manual_depth
            .fetch_update(ordering, Ordering::Acquire, |current| {
                Some(current.saturating_sub(1))
            });
    }

    /// Clear manual (frontend-driven) suppression. Called when a new watcher
    /// session starts so a webview reload mid-operation cannot leave the
    /// watcher suppressed forever. Backend guards are unaffected.
    pub fn reset_manual(&self) {
        self.manual_depth.store(0, Ordering::Release);
    }

    /// Register paths the app is about to mutate. Events under them (in any
    /// prefix/case spelling) are dropped until the guard drops plus
    /// `SUPPRESSION_TAIL`.
    pub fn suppress_paths(
        self: &Arc<Self>,
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> PathSuppressionGuard {
        let mut ids = Vec::new();
        {
            let mut scoped = lock(&self.scoped);
            for path in paths {
                let key = crate::common::path_key::canonical_path_key_for_path(path.as_ref());
                if scoped
                    .iter()
                    .any(|entry| entry.expires_at.is_none() && entry.key == key)
                {
                    continue;
                }
                let id = self.next_id.fetch_add(1, Ordering::Relaxed);
                scoped.push(ScopedEntry {
                    id,
                    key,
                    expires_at: None,
                });
                ids.push(id);
            }
        }
        PathSuppressionGuard {
            suppressor: self.clone(),
            ids,
        }
    }

    /// Whether an event path falls under a live scoped registration.
    pub fn is_path_suppressed(&self, path: &Path) -> bool {
        let key = crate::common::path_key::canonical_path_key_for_path(path);
        let now = Instant::now();
        let mut scoped = lock(&self.scoped);
        scoped.retain(|entry| entry.expires_at.is_none_or(|deadline| deadline > now));
        scoped.iter().any(|entry| {
            key.len() >= entry.key.len()
                && key.starts_with(entry.key.as_str())
                && (key.len() == entry.key.len() || key.as_bytes()[entry.key.len()] == b'/')
        })
    }

    fn release_scoped(&self, ids: &[u64]) {
        let deadline = Instant::now() + SUPPRESSION_TAIL;
        let mut scoped = lock(&self.scoped);
        for entry in scoped.iter_mut() {
            if ids.contains(&entry.id) {
                entry.expires_at = Some(deadline);
            }
        }
    }

    fn increment(&self) {
        self.guard_depth.fetch_add(1, Ordering::AcqRel);
    }

    fn decrement(&self) {
        let _ = self
            .guard_depth
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.saturating_sub(1))
            });
    }
}

pub struct SuppressionGuard {
    suppressor: Arc<WatcherSuppressor>,
}

impl SuppressionGuard {
    pub fn new(suppressor: &Arc<WatcherSuppressor>) -> Self {
        suppressor.increment();
        Self {
            suppressor: suppressor.clone(),
        }
    }
}

impl Drop for SuppressionGuard {
    fn drop(&mut self) {
        self.suppressor.decrement();
    }
}

pub struct PathSuppressionGuard {
    suppressor: Arc<WatcherSuppressor>,
    ids: Vec<u64>,
}

impl Drop for PathSuppressionGuard {
    fn drop(&mut self) {
        self.suppressor.release_scoped(&self.ids);
    }
}
