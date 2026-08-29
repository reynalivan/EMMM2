//! Watcher suppression.
//!
//! Two mechanisms:
//! - **Blanket** (`SuppressionGuard`): drops every event while held. For broad
//!   operations (deep scan, archive extraction) whose
//!   write set is unknown up front. Self-healing: those flows end with a full
//!   reconcile that re-reads disk anyway.
//! - **Path-scoped** (`PathSuppressionGuard`): drops only events under the
//!   registered paths, matched by identity key — so registering either the
//!   enabled or DISABLED spelling covers both sides of a toggle rename.
//!   Suppression ends with the mutation guard; queued app echoes are handled
//!   by idempotent reconcile instead of creating a blind window for external
//!   changes.

use crate::shared::sync::lock;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct ScopedEntry {
    id: u64,
    /// Identity key of the suppressed root (see `path_key`).
    key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatcherSession {
    generation: u64,
    canonical_root: String,
}

impl WatcherSession {
    pub(super) fn new(generation: u64, root: &Path) -> Self {
        Self {
            generation,
            canonical_root: crate::shared::path_key::canonical_path_key_for_path(root),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WatcherRepairEvidence {
    owner: WatcherSession,
    through_generation: u64,
}

#[derive(Default)]
struct RepairLedger {
    owner: Option<WatcherSession>,
    dropped_generation: u64,
    repaired_generation: u64,
}

pub struct WatcherSuppressor {
    guard_depth: AtomicUsize,
    next_id: AtomicU64,
    repair: Mutex<RepairLedger>,
    scoped: Mutex<Vec<ScopedEntry>>,
}

impl WatcherSuppressor {
    pub fn new(suppressed: bool) -> Self {
        Self {
            guard_depth: AtomicUsize::new(usize::from(suppressed)),
            next_id: AtomicU64::new(0),
            repair: Mutex::new(RepairLedger::default()),
            scoped: Mutex::new(Vec::new()),
        }
    }

    pub fn load(&self, ordering: Ordering) -> bool {
        self.guard_depth.load(ordering) > 0
    }

    pub(crate) fn begin_session(&self, session: &WatcherSession) {
        *lock(&self.repair) = RepairLedger {
            owner: Some(session.clone()),
            ..RepairLedger::default()
        };
    }

    pub(crate) fn invalidate_session(&self, session: &WatcherSession) {
        let mut repair = lock(&self.repair);
        if repair.owner.as_ref() == Some(session) {
            *repair = RepairLedger::default();
        }
    }

    pub(crate) fn mark_blanket_event_dropped(&self, session: &WatcherSession) {
        let mut repair = lock(&self.repair);
        if repair.owner.as_ref() == Some(session) {
            repair.dropped_generation = repair.dropped_generation.saturating_add(1);
        }
    }

    pub fn has_unrepaired_drops(&self) -> bool {
        let repair = lock(&self.repair);
        repair.dropped_generation > repair.repaired_generation
    }

    pub(crate) fn pending_repair(&self, session: &WatcherSession) -> Option<WatcherRepairEvidence> {
        let repair = lock(&self.repair);
        (repair.owner.as_ref() == Some(session)
            && repair.dropped_generation > repair.repaired_generation)
            .then(|| WatcherRepairEvidence {
                owner: session.clone(),
                through_generation: repair.dropped_generation,
            })
    }

    pub(crate) fn mark_repaired_through(&self, evidence: &WatcherRepairEvidence) -> bool {
        let mut repair = lock(&self.repair);
        if repair.owner.as_ref() != Some(&evidence.owner) {
            return false;
        }
        repair.repaired_generation = repair.repaired_generation.max(evidence.through_generation);
        true
    }

    /// Register paths the app is about to mutate. Events under them (in any
    /// prefix/case spelling) are dropped until the guard drops.
    pub fn suppress_paths(
        self: &Arc<Self>,
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> PathSuppressionGuard {
        let mut ids = Vec::new();
        {
            let mut scoped = lock(&self.scoped);
            for path in paths {
                let key = crate::shared::path_key::canonical_path_key_for_path(path.as_ref());
                let id = self.next_id.fetch_add(1, Ordering::Relaxed);
                scoped.push(ScopedEntry { id, key });
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
        let key = crate::shared::path_key::canonical_path_key_for_path(path);
        let scoped = lock(&self.scoped);
        scoped.iter().any(|entry| {
            key.len() >= entry.key.len()
                && key.starts_with(entry.key.as_str())
                && (key.len() == entry.key.len() || key.as_bytes()[entry.key.len()] == b'/')
        })
    }

    fn release_scoped(&self, ids: &[u64]) {
        let mut scoped = lock(&self.scoped);
        scoped.retain(|entry| !ids.contains(&entry.id));
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
