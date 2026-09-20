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
//!   Trusted rename echoes may be consumed only with session-, path-, and
//!   filesystem-identity evidence; every other observation remains dirty.

use crate::shared::sync::lock;
use notify::event::{ModifyKind, RenameMode};
use notify::EventKind;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TRUSTED_ECHO_TTL: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatcherSession {
    generation: u64,
    canonical_root: String,
    canonical_runtime_config: Option<String>,
}

impl WatcherSession {
    pub(super) fn new_with_runtime_config(
        generation: u64,
        root: &Path,
        runtime_config_path: Option<&Path>,
    ) -> Self {
        Self {
            generation,
            canonical_root: crate::shared::path_key::canonical_path_key_for_path(root),
            canonical_runtime_config: runtime_config_path
                .map(crate::shared::path_key::canonical_path_key_for_path),
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn covers_root(&self, root: &Path) -> bool {
        self.canonical_root == crate::shared::path_key::canonical_path_key_for_path(root)
    }

    pub(crate) fn covers(&self, root: &Path, runtime_config_path: Option<&Path>) -> bool {
        self.covers_root(root)
            && self.canonical_runtime_config
                == runtime_config_path.map(crate::shared::path_key::canonical_path_key_for_path)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ExpectedRenameEcho {
    pub(crate) old_path: PathBuf,
    pub(crate) new_path: PathBuf,
    pub(crate) expected_identity: String,
}

#[derive(Clone, Debug)]
pub(crate) struct TrustedWatcherMutationEvidence {
    id: u64,
    game_id: String,
    owner: WatcherSession,
}

#[derive(Debug)]
struct PendingRenameEcho {
    evidence_id: u64,
    game_id: String,
    owner: WatcherSession,
    old_key: String,
    new_key: String,
    expected_identity: String,
    new_path: PathBuf,
    saw_old_path: bool,
    saw_new_path: bool,
    expires_at: Instant,
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
    next_evidence_id: AtomicU64,
    repair: Mutex<RepairLedger>,
    /// Canonical suppressed roots and the number of live guards using each.
    /// Duplicate registrations are folded so bulk mutations do not make every
    /// watcher event scan the same path repeatedly.
    scoped: Mutex<HashMap<String, usize>>,
    expected_rename_echoes: Mutex<Vec<PendingRenameEcho>>,
}

impl WatcherSuppressor {
    pub fn new(suppressed: bool) -> Self {
        Self {
            guard_depth: AtomicUsize::new(usize::from(suppressed)),
            next_evidence_id: AtomicU64::new(0),
            repair: Mutex::new(RepairLedger::default()),
            scoped: Mutex::new(HashMap::new()),
            expected_rename_echoes: Mutex::new(Vec::new()),
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
        let mut keys = paths
            .into_iter()
            .map(|path| crate::shared::path_key::canonical_path_key_for_path(path.as_ref()))
            .collect::<Vec<_>>();
        keys.sort_by_key(String::len);
        keys.dedup();
        let mut minimal_keys = Vec::with_capacity(keys.len());
        for key in keys {
            let covered_by_parent = minimal_keys.iter().any(|parent: &String| {
                key.len() > parent.len()
                    && key.starts_with(parent)
                    && key.as_bytes()[parent.len()] == b'/'
            });
            if !covered_by_parent {
                minimal_keys.push(key);
            }
        }
        {
            let mut scoped = lock(&self.scoped);
            for key in &minimal_keys {
                *scoped.entry(key.clone()).or_insert(0) += 1;
            }
        }
        PathSuppressionGuard {
            suppressor: self.clone(),
            keys: minimal_keys,
        }
    }

    /// Whether an event path falls under a live scoped registration.
    pub fn is_path_suppressed(&self, path: &Path) -> bool {
        let key = crate::shared::path_key::canonical_path_key_for_path(path);
        let scoped = lock(&self.scoped);
        scoped.keys().any(|root| {
            key.len() >= root.len()
                && key.starts_with(root.as_str())
                && (key.len() == root.len() || key.as_bytes()[root.len()] == b'/')
        })
    }

    pub(crate) fn expect_rename_echoes(
        &self,
        game_id: &str,
        owner: &WatcherSession,
        renames: impl IntoIterator<Item = ExpectedRenameEcho>,
    ) -> TrustedWatcherMutationEvidence {
        let id = self
            .next_evidence_id
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        let expires_at = Instant::now() + TRUSTED_ECHO_TTL;
        let mut pending = lock(&self.expected_rename_echoes);
        pending.retain(|echo| echo.expires_at > Instant::now());
        pending.extend(renames.into_iter().map(|rename| PendingRenameEcho {
            evidence_id: id,
            game_id: game_id.to_string(),
            owner: owner.clone(),
            old_key: crate::shared::path_key::canonical_path_key_for_path(&rename.old_path),
            new_key: crate::shared::path_key::canonical_path_key_for_path(&rename.new_path),
            expected_identity: rename.expected_identity,
            new_path: rename.new_path,
            saw_old_path: false,
            saw_new_path: false,
            expires_at,
        }));
        TrustedWatcherMutationEvidence {
            id,
            game_id: game_id.to_string(),
            owner: owner.clone(),
        }
    }

    pub(crate) fn discard_expected_rename_echoes(&self, evidence: &TrustedWatcherMutationEvidence) {
        lock(&self.expected_rename_echoes).retain(|echo| {
            echo.evidence_id != evidence.id
                || echo.game_id != evidence.game_id
                || echo.owner != evidence.owner
        });
    }

    pub(crate) fn retain_expected_rename_echoes<'a>(
        &self,
        evidence: &TrustedWatcherMutationEvidence,
        renames: impl IntoIterator<Item = (&'a Path, &'a Path)>,
    ) {
        let retained = renames
            .into_iter()
            .map(|(old_path, new_path)| {
                (
                    crate::shared::path_key::canonical_path_key_for_path(old_path),
                    crate::shared::path_key::canonical_path_key_for_path(new_path),
                )
            })
            .collect::<HashSet<_>>();
        lock(&self.expected_rename_echoes).retain(|echo| {
            echo.evidence_id != evidence.id
                || echo.game_id != evidence.game_id
                || echo.owner != evidence.owner
                || retained.contains(&(echo.old_key.clone(), echo.new_key.clone()))
        });
    }

    pub(crate) fn consume_expected_rename_echo(
        &self,
        game_id: &str,
        owner: &WatcherSession,
        kind: &EventKind,
        paths: &[PathBuf],
    ) -> bool {
        let now = Instant::now();
        let mut pending = lock(&self.expected_rename_echoes);
        pending.retain(|echo| echo.expires_at > now);

        enum RenameObservation {
            Both { old_key: String, new_key: String },
            Old(String),
            New(String),
            Either(String),
        }

        let observation = match kind {
            EventKind::Modify(ModifyKind::Name(RenameMode::Both | RenameMode::Any))
                if paths.len() == 2 =>
            {
                RenameObservation::Both {
                    old_key: crate::shared::path_key::canonical_path_key_for_path(&paths[0]),
                    new_key: crate::shared::path_key::canonical_path_key_for_path(&paths[1]),
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) if paths.len() == 1 => {
                RenameObservation::Old(crate::shared::path_key::canonical_path_key_for_path(
                    &paths[0],
                ))
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) if paths.len() == 1 => {
                RenameObservation::New(crate::shared::path_key::canonical_path_key_for_path(
                    &paths[0],
                ))
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)) if paths.len() == 1 => {
                RenameObservation::Either(crate::shared::path_key::canonical_path_key_for_path(
                    &paths[0],
                ))
            }
            _ => return false,
        };

        let Some(index) = pending.iter().position(|echo| {
            let path_matches = match &observation {
                RenameObservation::Both { old_key, new_key } => {
                    echo.old_key == *old_key && echo.new_key == *new_key
                }
                RenameObservation::Old(key) => echo.old_key == *key,
                RenameObservation::New(key) => echo.new_key == *key,
                RenameObservation::Either(key) => echo.old_key == *key || echo.new_key == *key,
            };
            path_matches
                && echo.game_id == game_id
                && echo.owner == *owner
                && crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&echo.new_path)
                    .as_deref()
                    == Some(echo.expected_identity.as_str())
        }) else {
            return false;
        };

        let complete_pair = match observation {
            RenameObservation::Both { .. } => {
                pending.swap_remove(index);
                true
            }
            RenameObservation::Old(_) => {
                pending[index].saw_old_path = true;
                false
            }
            RenameObservation::New(_) => {
                pending[index].saw_new_path = true;
                false
            }
            RenameObservation::Either(key) => {
                pending[index].saw_old_path |= pending[index].old_key == key;
                pending[index].saw_new_path |= pending[index].new_key == key;
                false
            }
        };
        if !complete_pair
            && pending
                .get(index)
                .is_some_and(|echo| echo.saw_old_path && echo.saw_new_path)
        {
            pending.swap_remove(index);
        }
        true
    }

    fn release_scoped(&self, keys: &[String]) {
        let mut scoped = lock(&self.scoped);
        for key in keys {
            let Some(ref_count) = scoped.get_mut(key) else {
                continue;
            };
            *ref_count = ref_count.saturating_sub(1);
            if *ref_count == 0 {
                scoped.remove(key);
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
    keys: Vec<String>,
}

impl Drop for PathSuppressionGuard {
    fn drop(&mut self) {
        self.suppressor.release_scoped(&self.keys);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn releasing_one_guard_keeps_overlapping_and_duplicate_registrations_live() {
        let suppressor = Arc::new(WatcherSuppressor::new(false));
        let first =
            suppressor.suppress_paths([Path::new("C:/Mods/Alice"), Path::new("C:/Mods/Bob")]);
        let second = suppressor
            .suppress_paths([Path::new("C:/Mods/Alice/Blue"), Path::new("C:/Mods/Alice")]);

        drop(first);
        assert!(suppressor.is_path_suppressed(Path::new("C:/Mods/Alice/Blue/preview.png")));
        assert!(!suppressor.is_path_suppressed(Path::new("C:/Mods/Bob/mod.ini")));

        drop(second);
        assert!(!suppressor.is_path_suppressed(Path::new("C:/Mods/Alice/Blue/preview.png")));
    }

    #[test]
    fn one_guard_collapses_duplicate_and_nested_paths() {
        let suppressor = Arc::new(WatcherSuppressor::new(false));
        let guard = suppressor.suppress_paths([
            Path::new("C:/Mods/Alice"),
            Path::new("C:/Mods/Alice"),
            Path::new("C:/Mods/Alice/Blue"),
        ]);

        assert!(suppressor.is_path_suppressed(Path::new("C:/Mods/Alice/Blue/preview.png")));
        drop(guard);
        assert!(!suppressor.is_path_suppressed(Path::new("C:/Mods/Alice/Blue/preview.png")));
    }

    #[test]
    fn partial_batch_keeps_echo_evidence_only_for_applied_renames() {
        let temp = tempfile::tempdir().expect("tempdir");
        let first_old = temp.path().join("DISABLED Alice");
        let first_new = temp.path().join("Alice");
        let second_old = temp.path().join("DISABLED Bob");
        let second_new = temp.path().join("Bob");
        std::fs::create_dir(&first_old).expect("first source");
        std::fs::create_dir(&second_old).expect("second source");
        let first_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&first_old)
            .expect("first identity");
        let second_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&second_old)
            .expect("second identity");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let evidence = suppressor.expect_rename_echoes(
            "game-1",
            &session,
            [
                ExpectedRenameEcho {
                    old_path: first_old.clone(),
                    new_path: first_new.clone(),
                    expected_identity: first_identity,
                },
                ExpectedRenameEcho {
                    old_path: second_old.clone(),
                    new_path: second_new.clone(),
                    expected_identity: second_identity,
                },
            ],
        );
        suppressor
            .retain_expected_rename_echoes(&evidence, [(first_old.as_path(), first_new.as_path())]);
        std::fs::rename(&first_old, &first_new).expect("first rename");
        std::fs::rename(&second_old, &second_new).expect("second rename");
        let kind = EventKind::Modify(ModifyKind::Name(RenameMode::Both));

        assert!(suppressor.consume_expected_rename_echo(
            "game-1",
            &session,
            &kind,
            &[first_old, first_new],
        ));
        assert!(!suppressor.consume_expected_rename_echo(
            "game-1",
            &session,
            &kind,
            &[second_old, second_new],
        ));
    }
}
