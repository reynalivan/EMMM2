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

use crate::shared::path_key::exact_location_key_for_path as rename_location_key;
use crate::shared::sync::lock;
use notify::event::{ModifyKind, RenameMode};
use notify::EventKind;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TRUSTED_ECHO_TTL: Duration = Duration::from_secs(10);
const MAX_EXPECTED_RENAME_EDGES: usize = 8192;

type UnprovenRenameObserver = Arc<dyn Fn(&[PathBuf], bool) + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatcherSession {
    generation: u64,
    canonical_root: String,
    canonical_runtime_config: Option<String>,
    root_path: PathBuf,
    root_identity: Option<String>,
}

impl WatcherSession {
    pub(super) fn new_with_runtime_config(
        generation: u64,
        root: &Path,
        runtime_config_path: Option<&Path>,
    ) -> Self {
        Self {
            generation,
            canonical_root: rename_location_key(root),
            canonical_runtime_config: runtime_config_path.map(rename_location_key),
            root_path: root.to_path_buf(),
            root_identity: crate::platform::fs::file_utils::filesystem_identity(root),
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn covers_root(&self, root: &Path) -> bool {
        self.canonical_root == rename_location_key(root)
    }

    pub(crate) fn covers(&self, root: &Path, runtime_config_path: Option<&Path>) -> bool {
        self.covers_root(root)
            && self.canonical_runtime_config == runtime_config_path.map(rename_location_key)
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

struct PendingRenameEcho {
    evidence_id: u64,
    game_id: String,
    owner: WatcherSession,
    old_key: String,
    new_key: String,
    expected_identity: String,
    old_path: PathBuf,
    new_path: PathBuf,
    committed: bool,
    saw_old_path: bool,
    saw_new_path: bool,
    expires_at: Instant,
    on_unproven: Option<UnprovenRenameObserver>,
}

impl PendingRenameEcho {
    fn belongs_to(&self, evidence: &TrustedWatcherMutationEvidence) -> bool {
        self.evidence_id == evidence.id
            && self.game_id == evidence.game_id
            && self.owner == evidence.owner
    }

    fn completed(&self) -> bool {
        self.saw_old_path && self.saw_new_path
    }
}

#[derive(Default)]
struct RenameEchoLedger {
    edges: Vec<PendingRenameEcho>,
    expiry_worker_running: bool,
}

enum RenameObservation {
    Both { old_key: String, new_key: String },
    Old(String),
    New(String),
    Either(String),
}

impl RenameObservation {
    fn from_event(kind: &EventKind, paths: &[PathBuf]) -> Option<Self> {
        match kind {
            EventKind::Modify(ModifyKind::Name(RenameMode::Both | RenameMode::Any))
                if paths.len() == 2 =>
            {
                Some(Self::Both {
                    old_key: rename_location_key(&paths[0]),
                    new_key: rename_location_key(&paths[1]),
                })
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) if paths.len() == 1 => {
                Some(Self::Old(rename_location_key(&paths[0])))
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) if paths.len() == 1 => {
                Some(Self::New(rename_location_key(&paths[0])))
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)) if paths.len() == 1 => {
                Some(Self::Either(rename_location_key(&paths[0])))
            }
            _ => None,
        }
    }

    fn matches(&self, edge: &PendingRenameEcho) -> bool {
        if !self.matches_locations(edge) {
            return false;
        }
        match self {
            Self::Both { .. } => !edge.saw_old_path && !edge.saw_new_path,
            Self::Old(_) => !edge.saw_old_path,
            Self::New(_) => !edge.saw_new_path,
            Self::Either(key) => {
                (!edge.saw_old_path && edge.old_key == *key)
                    || (!edge.saw_new_path && edge.new_key == *key)
            }
        }
    }

    fn matches_locations(&self, edge: &PendingRenameEcho) -> bool {
        match self {
            Self::Both { old_key, new_key } => edge.old_key == *old_key && edge.new_key == *new_key,
            Self::Old(key) => edge.old_key == *key,
            Self::New(key) => edge.new_key == *key,
            Self::Either(key) => edge.old_key == *key || edge.new_key == *key,
        }
    }

    fn record(self, edge: &mut PendingRenameEcho) {
        match self {
            Self::Both { .. } => {
                edge.saw_old_path = true;
                edge.saw_new_path = true;
            }
            Self::Old(_) => edge.saw_old_path = true,
            Self::New(_) => edge.saw_new_path = true,
            Self::Either(key) => {
                edge.saw_old_path |= edge.old_key == key;
                edge.saw_new_path |= edge.new_key == key;
            }
        }
    }
}

#[derive(PartialEq, Eq)]
enum LineageProof {
    Verified,
    Pending,
    Unproven,
}

fn lineage_proof(edges: &[PendingRenameEcho], index: usize) -> LineageProof {
    let first = &edges[index];
    if first.owner.root_identity.is_none()
        || crate::platform::fs::file_utils::filesystem_identity(&first.owner.root_path)
            != first.owner.root_identity
    {
        return LineageProof::Unproven;
    }
    let mut participants = vec![(first.expected_identity.as_str(), first.old_path.clone())];
    let mut planned = false;
    let mut checked_locations = HashSet::new();
    for edge in edges
        .iter()
        .skip(index)
        .filter(|edge| edge.game_id == first.game_id && edge.owner == first.owner)
    {
        let participant = participants
            .iter()
            .position(|(identity, _)| *identity == edge.expected_identity);
        if let Some(participant) = participant {
            if rename_location_key(&participants[participant].1) != edge.old_key {
                return LineageProof::Unproven;
            }
        } else if participants
            .iter()
            .any(|(_, endpoint)| endpoint.starts_with(&edge.old_path))
        {
            participants.push((edge.expected_identity.as_str(), edge.old_path.clone()));
        } else {
            continue;
        }
        planned |= !edge.committed;
        for (key, path) in [
            (&edge.old_key, &edge.old_path),
            (&edge.new_key, &edge.new_path),
        ] {
            if !checked_locations.insert((edge.expected_identity.as_str(), key)) {
                continue;
            }
            match path.try_exists() {
                Ok(false) => {}
                Ok(true)
                    if crate::platform::fs::file_utils::filesystem_identity(path).as_deref()
                        == Some(edge.expected_identity.as_str()) => {}
                _ => return LineageProof::Unproven,
            }
        }
        // A proven parent move changes the location of its tracked descendants,
        // without changing their filesystem identities.
        for (_, endpoint) in &mut participants {
            if let Ok(suffix) = endpoint.strip_prefix(&edge.old_path) {
                *endpoint = edge.new_path.join(suffix);
            }
        }
    }
    if planned {
        return LineageProof::Pending;
    }
    if participants.iter().all(|(identity, endpoint)| {
        crate::platform::fs::file_utils::filesystem_identity(endpoint).as_deref() == Some(*identity)
    }) {
        LineageProof::Verified
    } else {
        LineageProof::Unproven
    }
}

fn observed_paths_match_identity(paths: &[PathBuf], expected_identity: &str) -> bool {
    paths.iter().all(|path| match path.try_exists() {
        Ok(false) => true,
        Ok(true) => {
            crate::platform::fs::file_utils::filesystem_identity(path).as_deref()
                == Some(expected_identity)
        }
        Err(_) => false,
    })
}

// The debouncer folds successive renames into original -> final, including
// A -> A. Pending chains retain the same deferred proof as individual echoes.
fn record_folded_rename(
    edges: &mut [PendingRenameEcho],
    game_id: &str,
    owner: &WatcherSession,
    observation: &RenameObservation,
    paths: &[PathBuf],
    observer: &UnprovenRenameObserver,
) -> bool {
    let RenameObservation::Both { old_key, new_key } = observation else {
        return false;
    };
    for start in 0..edges.len() {
        let first = &edges[start];
        if first.game_id != game_id
            || first.owner != *owner
            || first.old_key != *old_key
            || !observed_paths_match_identity(paths, &first.expected_identity)
        {
            continue;
        }
        let proof = lineage_proof(edges, start);
        if proof == LineageProof::Unproven {
            continue;
        }
        let mut endpoint = old_key.as_str();
        let mut chain = Vec::new();
        let mut covered = 0;
        for (index, edge) in edges.iter().enumerate().skip(start) {
            if edge.expected_identity != first.expected_identity {
                continue;
            }
            if edge.game_id != game_id || edge.owner != *owner || edge.old_key != endpoint {
                break;
            }
            chain.push(index);
            endpoint = &edge.new_key;
            if endpoint == new_key {
                covered = chain.len();
            }
        }
        if covered < 2 {
            continue;
        }
        for index in chain.into_iter().take(covered) {
            edges[index].saw_old_path = true;
            edges[index].saw_new_path = true;
            edges[index].on_unproven =
                (proof == LineageProof::Pending).then(|| Arc::clone(observer));
        }
        return true;
    }
    false
}

fn release_unproven(
    edges: Vec<PendingRenameEcho>,
    repair: &Mutex<RepairLedger>,
    coverage_lost: bool,
) {
    for edge in edges {
        let unresolved = !edge.committed || !edge.completed() || edge.on_unproven.is_some();
        if unresolved {
            let mut ledger = lock(repair);
            if ledger.owner.as_ref() == Some(&edge.owner) {
                ledger.dropped_generation = ledger.dropped_generation.saturating_add(1);
            }
            drop(ledger);
            if let Some(observer) = edge.on_unproven {
                observer(&[edge.old_path, edge.new_path], coverage_lost);
            }
        }
    }
}

fn expire_edges(ledger: &Mutex<RenameEchoLedger>, repair: &Mutex<RepairLedger>) {
    let now = Instant::now();
    let mut ledger = lock(ledger);
    let mut expired = Vec::new();
    let mut index = 0;
    while index < ledger.edges.len() {
        if ledger.edges[index].expires_at <= now {
            expired.push(ledger.edges.remove(index));
        } else {
            index += 1;
        }
    }
    drop(ledger);
    release_unproven(expired, repair, true);
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
    repair: Arc<Mutex<RepairLedger>>,
    /// Canonical suppressed roots and the number of live guards using each.
    /// Duplicate registrations are folded so bulk mutations do not make every
    /// watcher event scan the same path repeatedly.
    scoped: Mutex<HashMap<String, usize>>,
    expected_rename_echoes: Arc<Mutex<RenameEchoLedger>>,
}

impl WatcherSuppressor {
    pub fn new(suppressed: bool) -> Self {
        Self {
            guard_depth: AtomicUsize::new(usize::from(suppressed)),
            next_evidence_id: AtomicU64::new(0),
            repair: Arc::new(Mutex::new(RepairLedger::default())),
            scoped: Mutex::new(HashMap::new()),
            expected_rename_echoes: Arc::new(Mutex::new(RenameEchoLedger::default())),
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
        self.invalidate_rename_echoes(session);
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
        expire_edges(&self.expected_rename_echoes, &self.repair);
        let expires_at = Instant::now() + TRUSTED_ECHO_TTL;
        let mut ledger = lock(&self.expected_rename_echoes);
        let mut overflow = Vec::new();
        for rename in renames {
            let edge = PendingRenameEcho {
                evidence_id: id,
                game_id: game_id.to_string(),
                owner: owner.clone(),
                old_key: rename_location_key(&rename.old_path),
                new_key: rename_location_key(&rename.new_path),
                expected_identity: rename.expected_identity,
                old_path: rename.old_path,
                new_path: rename.new_path,
                committed: false,
                saw_old_path: false,
                saw_new_path: false,
                expires_at,
                on_unproven: None,
            };
            if ledger.edges.len() < MAX_EXPECTED_RENAME_EDGES {
                ledger.edges.push(edge);
            } else {
                overflow.push(edge);
            }
        }
        if !overflow.is_empty() {
            let mut index = 0;
            while index < ledger.edges.len() {
                if ledger.edges[index].owner == *owner {
                    overflow.push(ledger.edges.remove(index));
                } else {
                    index += 1;
                }
            }
        }
        let spawn_expiry = !ledger.expiry_worker_running && !ledger.edges.is_empty();
        if spawn_expiry {
            ledger.expiry_worker_running = true;
        }
        drop(ledger);
        release_unproven(overflow, &self.repair, true);
        if spawn_expiry {
            let ledger = Arc::clone(&self.expected_rename_echoes);
            let repair = Arc::clone(&self.repair);
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    loop {
                        let deadline = {
                            let mut pending = lock(&ledger);
                            let deadline = pending.edges.iter().map(|edge| edge.expires_at).min();
                            if deadline.is_none() {
                                pending.expiry_worker_running = false;
                            }
                            deadline
                        };
                        let Some(deadline) = deadline else {
                            break;
                        };
                        tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await;
                        expire_edges(&ledger, &repair);
                    }
                });
            } else {
                lock(&ledger).expiry_worker_running = false;
            }
        }
        TrustedWatcherMutationEvidence {
            id,
            game_id: game_id.to_string(),
            owner: owner.clone(),
        }
    }

    pub(crate) fn discard_expected_rename_echoes(&self, evidence: &TrustedWatcherMutationEvidence) {
        self.remove_rename_echoes(|edge| edge.belongs_to(evidence), false);
    }

    pub(crate) fn retain_expected_rename_echoes<'a>(
        &self,
        evidence: &TrustedWatcherMutationEvidence,
        renames: impl IntoIterator<Item = (&'a Path, &'a Path)>,
    ) {
        let retained = renames
            .into_iter()
            .map(|(old_path, new_path)| {
                (rename_location_key(old_path), rename_location_key(new_path))
            })
            .collect::<HashSet<_>>();
        self.remove_rename_echoes(
            |edge| {
                edge.belongs_to(evidence)
                    && !retained.contains(&(edge.old_key.clone(), edge.new_key.clone()))
            },
            false,
        );
    }

    fn remove_rename_echoes(
        &self,
        remove: impl Fn(&PendingRenameEcho) -> bool,
        coverage_lost: bool,
    ) {
        let mut ledger = lock(&self.expected_rename_echoes);
        let mut removed = Vec::new();
        let mut index = 0;
        while index < ledger.edges.len() {
            if remove(&ledger.edges[index]) {
                removed.push(ledger.edges.remove(index));
            } else {
                index += 1;
            }
        }
        drop(ledger);
        release_unproven(removed, &self.repair, coverage_lost);
        self.settle_deferred_echoes();
    }

    /// Called only after storage verification and durable DiskCommitted.
    pub(crate) fn commit_expected_rename_echoes(
        &self,
        evidence: &TrustedWatcherMutationEvidence,
    ) -> bool {
        expire_edges(&self.expected_rename_echoes, &self.repair);
        let mut ledger = lock(&self.expected_rename_echoes);
        let mut found = false;
        for edge in &mut ledger.edges {
            if edge.belongs_to(evidence) {
                edge.committed = true;
                found = true;
            }
        }
        let verified = found
            && ledger
                .edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| edge.belongs_to(evidence))
                .all(|(index, _)| lineage_proof(&ledger.edges, index) == LineageProof::Verified);
        if !verified {
            drop(ledger);
            self.discard_expected_rename_echoes(evidence);
            return false;
        }
        drop(ledger);
        self.settle_deferred_echoes();
        true
    }

    fn settle_deferred_echoes(&self) {
        let mut ledger = lock(&self.expected_rename_echoes);
        let mut failed_callbacks = Vec::new();
        for index in 0..ledger.edges.len() {
            if ledger.edges[index].on_unproven.is_none() {
                continue;
            }
            match lineage_proof(&ledger.edges, index) {
                LineageProof::Verified => {
                    ledger.edges[index].on_unproven = None;
                }
                LineageProof::Pending => {}
                LineageProof::Unproven => {
                    let edge = &mut ledger.edges[index];
                    if let Some(observer) = edge.on_unproven.take() {
                        failed_callbacks
                            .push((observer, vec![edge.old_path.clone(), edge.new_path.clone()]));
                    }
                }
            }
        }
        drop(ledger);
        for (observer, paths) in failed_callbacks {
            observer(&paths, false);
        }
    }

    pub(crate) fn expected_echo_watermark(&self) -> u64 {
        self.next_evidence_id.load(Ordering::Acquire)
    }

    /// The caller has verified a disk snapshot and continuity through this watermark.
    pub(crate) fn mark_rename_echoes_reconciled_through(
        &self,
        owner: &WatcherSession,
        watermark: u64,
    ) {
        let mut ledger = lock(&self.expected_rename_echoes);
        let mut unresolved_identities = ledger
            .edges
            .iter()
            .filter(|edge| {
                edge.owner == *owner
                    && (!edge.committed || !edge.completed() || edge.on_unproven.is_some())
            })
            .map(|edge| (edge.game_id.clone(), edge.expected_identity.clone()))
            .collect::<HashSet<_>>();
        // Retain ancestor evidence while an outstanding child echo still needs
        // its path rewrite, even when the parent's own echo was consumed.
        let mut identities_by_location: HashMap<(String, String), HashSet<String>> = HashMap::new();
        let mut locations_by_identity: HashMap<(String, String), HashSet<PathBuf>> = HashMap::new();
        for edge in ledger.edges.iter().filter(|edge| edge.owner == *owner) {
            for (key, path) in [
                (&edge.old_key, &edge.old_path),
                (&edge.new_key, &edge.new_path),
            ] {
                identities_by_location
                    .entry((edge.game_id.clone(), key.clone()))
                    .or_default()
                    .insert(edge.expected_identity.clone());
                locations_by_identity
                    .entry((edge.game_id.clone(), edge.expected_identity.clone()))
                    .or_default()
                    .insert(path.clone());
            }
        }
        let mut pending = unresolved_identities.iter().cloned().collect::<Vec<_>>();
        while let Some((game_id, identity)) = pending.pop() {
            let Some(paths) = locations_by_identity.get(&(game_id.clone(), identity)) else {
                continue;
            };
            for path in paths {
                for ancestor in path.ancestors().skip(1) {
                    let Some(identities) = identities_by_location
                        .get(&(game_id.clone(), rename_location_key(ancestor)))
                    else {
                        continue;
                    };
                    for identity in identities {
                        let participant = (game_id.clone(), identity.clone());
                        if unresolved_identities.insert(participant.clone()) {
                            pending.push(participant);
                        }
                    }
                }
            }
        }
        ledger.edges.retain(|edge| {
            edge.owner != *owner
                || edge.evidence_id > watermark
                || unresolved_identities
                    .contains(&(edge.game_id.clone(), edge.expected_identity.clone()))
        });
    }

    /// Holds early authority observations until their operation supplies disk proof.
    pub(crate) fn observe_expected_rename_echo(
        &self,
        game_id: &str,
        owner: &WatcherSession,
        kind: &EventKind,
        paths: &[PathBuf],
        on_unproven: UnprovenRenameObserver,
    ) -> bool {
        self.consume_rename_echo(game_id, owner, kind, paths, Some(on_unproven))
            || self.committed_rename_echo_matches(Some(game_id), owner, kind, paths)
    }

    /// Only durable identity-proven echoes may bypass delivery; pending edges
    /// still reach reconciliation in case their operation fails or rolls back.
    pub(crate) fn is_committed_rename_echo(
        &self,
        owner: &WatcherSession,
        kind: &EventKind,
        paths: &[PathBuf],
    ) -> bool {
        self.committed_rename_echo_matches(None, owner, kind, paths)
    }

    fn committed_rename_echo_matches(
        &self,
        game_id: Option<&str>,
        owner: &WatcherSession,
        kind: &EventKind,
        paths: &[PathBuf],
    ) -> bool {
        expire_edges(&self.expected_rename_echoes, &self.repair);
        let Some(observation) = RenameObservation::from_event(kind, paths) else {
            return false;
        };
        let ledger = lock(&self.expected_rename_echoes);
        ledger.edges.iter().enumerate().any(|(index, edge)| {
            edge.owner == *owner
                && game_id.is_none_or(|game_id| edge.game_id == game_id)
                && observation.matches_locations(edge)
                && observed_paths_match_identity(paths, &edge.expected_identity)
                && lineage_proof(&ledger.edges, index) == LineageProof::Verified
        })
    }

    #[cfg(test)]
    pub(crate) fn consume_expected_rename_echo(
        &self,
        game_id: &str,
        owner: &WatcherSession,
        kind: &EventKind,
        paths: &[PathBuf],
    ) -> bool {
        self.consume_rename_echo(game_id, owner, kind, paths, None)
    }

    fn consume_rename_echo(
        &self,
        game_id: &str,
        owner: &WatcherSession,
        kind: &EventKind,
        paths: &[PathBuf],
        on_unproven: Option<UnprovenRenameObserver>,
    ) -> bool {
        expire_edges(&self.expected_rename_echoes, &self.repair);
        let Some(observation) = RenameObservation::from_event(kind, paths) else {
            return false;
        };
        let mut ledger = lock(&self.expected_rename_echoes);
        if on_unproven.as_ref().is_some_and(|observer| {
            record_folded_rename(
                &mut ledger.edges,
                game_id,
                owner,
                &observation,
                paths,
                observer,
            )
        }) {
            return true;
        }
        let Some(index) = ledger.edges.iter().position(|edge| {
            edge.game_id == game_id
                && edge.owner == *owner
                && (observation.matches(edge)
                    // A stitched observation can supply the missing half of an
                    // earlier split event; it still needs the full native proof.
                    || (on_unproven.is_some()
                        && matches!(observation, RenameObservation::Both { .. })
                        && !edge.completed()
                        && observation.matches_locations(edge)))
        }) else {
            return false;
        };
        let expected_identity = &ledger.edges[index].expected_identity;
        if !observed_paths_match_identity(paths, expected_identity) {
            return false;
        }
        match lineage_proof(&ledger.edges, index) {
            LineageProof::Unproven => return false,
            LineageProof::Pending => {
                let Some(observer) = on_unproven else {
                    return false;
                };
                ledger.edges[index].on_unproven = Some(observer);
            }
            LineageProof::Verified => {}
        }
        observation.record(&mut ledger.edges[index]);
        true
    }

    pub(crate) fn invalidate_rename_echoes(&self, owner: &WatcherSession) {
        self.remove_rename_echoes(|edge| edge.owner == *owner, true);
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

    fn source_identity(path: &Path) -> String {
        crate::platform::fs::file_utils::filesystem_identity(path).expect("filesystem identity")
    }

    fn paired_rename() -> EventKind {
        EventKind::Modify(ModifyKind::Name(RenameMode::Both))
    }

    fn record_rename(
        suppressor: &WatcherSuppressor,
        session: &WatcherSession,
        old_path: &Path,
        new_path: &Path,
    ) -> TrustedWatcherMutationEvidence {
        suppressor.expect_rename_echoes(
            "game",
            session,
            [ExpectedRenameEcho {
                old_path: old_path.to_path_buf(),
                new_path: new_path.to_path_buf(),
                expected_identity: source_identity(old_path),
            }],
        )
    }

    fn dirty_observer() -> (UnprovenRenameObserver, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&count);
        (
            Arc::new(move |_, _| {
                observed.fetch_add(1, Ordering::Relaxed);
            }),
            count,
        )
    }

    #[test]
    fn exact_rename_locations_do_not_alias_enabled_and_disabled_names() {
        assert_ne!(
            rename_location_key(Path::new("C:/Mods/Alice")),
            rename_location_key(Path::new("C:/Mods/DISABLED Alice"))
        );
        assert_eq!(
            rename_location_key(Path::new("C:\\Mods\\Alice")),
            rename_location_key(Path::new("c:/mods/alice"))
        );
    }

    #[test]
    fn planned_echo_is_deferred_and_only_committed_after_disk_proof() {
        let temp = tempfile::tempdir().expect("tempdir");
        let old_path = temp.path().join("Alice");
        let new_path = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&old_path).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let evidence = record_rename(&suppressor, &session, &old_path, &new_path);
        let paths = vec![old_path.clone(), new_path.clone()];
        assert!(!suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &paths
        ));
        let (observer, dirty) = dirty_observer();
        assert!(suppressor.observe_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &paths,
            observer
        ));
        assert_eq!(dirty.load(Ordering::Relaxed), 0);
        std::fs::rename(&old_path, &new_path).expect("rename");
        assert!(suppressor.commit_expected_rename_echoes(&evidence));
        assert_eq!(dirty.load(Ordering::Relaxed), 0);
        assert!(!suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &paths
        ));
    }

    #[test]
    fn abort_releases_early_echo_as_dirty_without_leaving_suppression() {
        let temp = tempfile::tempdir().expect("tempdir");
        let old_path = temp.path().join("Alice");
        let new_path = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&old_path).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        suppressor.begin_session(&session);
        let evidence = record_rename(&suppressor, &session, &old_path, &new_path);
        let paths = vec![old_path, new_path];
        let (observer, dirty) = dirty_observer();
        assert!(suppressor.observe_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &paths,
            observer
        ));
        suppressor.discard_expected_rename_echoes(&evidence);
        assert_eq!(dirty.load(Ordering::Relaxed), 1);
        assert!(suppressor.has_unrepaired_drops());
        assert!(!suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &paths
        ));
    }

    #[test]
    fn unexecuted_plan_cannot_be_promoted_to_commit() {
        let temp = tempfile::tempdir().expect("tempdir");
        let old_path = temp.path().join("Alice");
        let new_path = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&old_path).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let evidence = record_rename(&suppressor, &session, &old_path, &new_path);
        assert!(!suppressor.commit_expected_rename_echoes(&evidence));
    }

    #[test]
    fn reordered_split_reverse_echoes_are_consumed_once() {
        let temp = tempfile::tempdir().expect("tempdir");
        let enabled = temp.path().join("Alice");
        let disabled = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&enabled).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let first = record_rename(&suppressor, &session, &enabled, &disabled);
        std::fs::rename(&enabled, &disabled).expect("disable");
        assert!(suppressor.commit_expected_rename_echoes(&first));
        let second = record_rename(&suppressor, &session, &disabled, &enabled);
        std::fs::rename(&disabled, &enabled).expect("enable");
        assert!(suppressor.commit_expected_rename_echoes(&second));
        for (mode, path) in [
            (RenameMode::To, &enabled),
            (RenameMode::From, &enabled),
            (RenameMode::To, &disabled),
            (RenameMode::From, &disabled),
        ] {
            assert!(suppressor.consume_expected_rename_echo(
                "game",
                &session,
                &EventKind::Modify(ModifyKind::Name(mode)),
                std::slice::from_ref(path)
            ));
        }
        assert!(!suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &[enabled, disabled]
        ));
    }

    #[test]
    fn intermediate_path_replacement_is_never_swallowed_as_internal_echo() {
        let temp = tempfile::tempdir().expect("tempdir");
        let enabled = temp.path().join("Alice");
        let disabled = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&enabled).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let first = record_rename(&suppressor, &session, &enabled, &disabled);
        std::fs::rename(&enabled, &disabled).expect("disable");
        assert!(suppressor.commit_expected_rename_echoes(&first));
        let second = record_rename(&suppressor, &session, &disabled, &enabled);
        std::fs::rename(&disabled, &enabled).expect("enable");
        assert!(suppressor.commit_expected_rename_echoes(&second));
        std::fs::create_dir(&disabled).expect("external different identity");
        assert!(!suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &[enabled, disabled]
        ));
    }

    #[test]
    fn replaced_root_cannot_consume_old_session_evidence() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("Mods");
        let old_path = root.join("Alice");
        let new_path = root.join("DISABLED Alice");
        std::fs::create_dir_all(&old_path).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, &root, None);
        let evidence = record_rename(&suppressor, &session, &old_path, &new_path);
        std::fs::rename(&old_path, &new_path).expect("rename");
        assert!(suppressor.commit_expected_rename_echoes(&evidence));
        std::fs::rename(&root, temp.path().join("Previous Mods")).expect("replace root");
        std::fs::create_dir(&root).expect("new root");
        assert!(!suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &[old_path, new_path]
        ));
    }

    #[test]
    fn expiry_and_capacity_loss_request_repair_instead_of_silent_drop() {
        let temp = tempfile::tempdir().expect("tempdir");
        let old_path = temp.path().join("Alice");
        let new_path = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&old_path).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        suppressor.begin_session(&session);
        record_rename(&suppressor, &session, &old_path, &new_path);
        let (observer, dirty) = dirty_observer();
        assert!(suppressor.observe_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &[old_path.clone(), new_path.clone()],
            observer
        ));
        lock(&suppressor.expected_rename_echoes).edges[0].expires_at = Instant::now();
        expire_edges(&suppressor.expected_rename_echoes, &suppressor.repair);
        assert_eq!(dirty.load(Ordering::Relaxed), 1);
        assert!(suppressor.has_unrepaired_drops());
        suppressor.expect_rename_echoes(
            "game",
            &session,
            (0..=MAX_EXPECTED_RENAME_EDGES).map(|_| ExpectedRenameEcho {
                old_path: old_path.clone(),
                new_path: new_path.clone(),
                expected_identity: "identity".to_string(),
            }),
        );
        assert!(lock(&suppressor.expected_rename_echoes).edges.is_empty());
    }

    #[test]
    fn stitched_echo_after_observed_half_finishes_evidence_before_expiry() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("Alice");
        let target = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&source).unwrap();
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        suppressor.begin_session(&session);
        let evidence = record_rename(&suppressor, &session, &source, &target);
        let (observer, dirty) = dirty_observer();
        assert!(suppressor.observe_expected_rename_echo(
            "game",
            &session,
            &EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            std::slice::from_ref(&source),
            observer.clone()
        ));
        std::fs::rename(&source, &target).unwrap();
        assert!(suppressor.commit_expected_rename_echoes(&evidence));
        assert!(suppressor.observe_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &[source, target],
            observer
        ));
        lock(&suppressor.expected_rename_echoes).edges[0].expires_at = Instant::now();
        expire_edges(&suppressor.expected_rename_echoes, &suppressor.repair);
        assert_eq!(dirty.load(Ordering::Relaxed), 0);
        assert!(
            !suppressor.has_unrepaired_drops(),
            "both rename halves were observed with committed native proof"
        );
    }

    #[test]
    fn child_and_parent_in_one_receipt_verify_folded_endpoints() {
        let temp = tempfile::tempdir().expect("tempdir");
        let parent = temp.path().join("A");
        let moved_parent = temp.path().join("DISABLED A");
        let old_child = parent.join("DISABLED X");
        let new_child = parent.join("X");
        std::fs::create_dir_all(&old_child).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let evidence = suppressor.expect_rename_echoes(
            "game",
            &session,
            [
                ExpectedRenameEcho {
                    old_path: old_child.clone(),
                    new_path: new_child.clone(),
                    expected_identity: source_identity(&old_child),
                },
                ExpectedRenameEcho {
                    old_path: parent.clone(),
                    new_path: moved_parent.clone(),
                    expected_identity: source_identity(&parent),
                },
            ],
        );
        std::fs::rename(&old_child, &new_child).expect("child rename");
        std::fs::rename(&parent, &moved_parent).expect("parent rename");
        assert!(suppressor.commit_expected_rename_echoes(&evidence));
        assert!(suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &[old_child, new_child]
        ));
    }

    #[test]
    fn child_echo_does_not_use_foreign_game_or_session_parent_evidence() {
        for foreign_session in [false, true] {
            let temp = tempfile::tempdir().expect("tempdir");
            let parent = temp.path().join("A");
            let moved_parent = temp.path().join("DISABLED A");
            let old_child = parent.join("DISABLED X");
            let new_child = parent.join("X");
            std::fs::create_dir_all(&old_child).expect("source");
            let suppressor = WatcherSuppressor::new(false);
            let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
            let child = record_rename(&suppressor, &session, &old_child, &new_child);
            std::fs::rename(&old_child, &new_child).expect("child rename");
            assert!(suppressor.commit_expected_rename_echoes(&child));
            let other = WatcherSession::new_with_runtime_config(2, temp.path(), None);
            let ancestor = suppressor.expect_rename_echoes(
                if foreign_session {
                    "game"
                } else {
                    "other-game"
                },
                if foreign_session { &other } else { &session },
                [ExpectedRenameEcho {
                    old_path: parent.clone(),
                    new_path: moved_parent.clone(),
                    expected_identity: source_identity(&parent),
                }],
            );
            std::fs::rename(&parent, &moved_parent).expect("foreign parent rename");
            assert!(suppressor.commit_expected_rename_echoes(&ancestor));
            assert!(!suppressor.consume_expected_rename_echo(
                "game",
                &session,
                &paired_rename(),
                &[old_child, new_child]
            ));
        }
    }

    #[test]
    fn child_echo_rejects_replaced_ancestor_even_if_child_identity_is_preserved() {
        let temp = tempfile::tempdir().expect("tempdir");
        let parent = temp.path().join("A");
        let moved_parent = temp.path().join("DISABLED A");
        let old_child = parent.join("DISABLED X");
        let new_child = parent.join("X");
        std::fs::create_dir_all(&old_child).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let child = record_rename(&suppressor, &session, &old_child, &new_child);
        std::fs::rename(&old_child, &new_child).expect("child rename");
        assert!(suppressor.commit_expected_rename_echoes(&child));
        let ancestor = record_rename(&suppressor, &session, &parent, &moved_parent);
        std::fs::rename(&parent, &moved_parent).expect("parent rename");
        assert!(suppressor.commit_expected_rename_echoes(&ancestor));
        let previous_parent = temp.path().join("Previous A");
        std::fs::rename(&moved_parent, &previous_parent).expect("external replace");
        std::fs::create_dir(&moved_parent).expect("replacement ancestor");
        std::fs::rename(previous_parent.join("X"), moved_parent.join("X"))
            .expect("preserve child identity");
        assert!(!suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &[old_child, new_child]
        ));
    }

    #[test]
    fn child_echo_follows_committed_ancestor_rename() {
        let temp = tempfile::tempdir().expect("tempdir");
        let parent = temp.path().join("A");
        let moved_parent = temp.path().join("DISABLED A");
        let old_child = parent.join("DISABLED X");
        let new_child = parent.join("X");
        std::fs::create_dir_all(&old_child).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let child = record_rename(&suppressor, &session, &old_child, &new_child);
        std::fs::rename(&old_child, &new_child).expect("child rename");
        assert!(suppressor.commit_expected_rename_echoes(&child));
        let ancestor = record_rename(&suppressor, &session, &parent, &moved_parent);
        std::fs::rename(&parent, &moved_parent).expect("parent rename");
        assert!(suppressor.commit_expected_rename_echoes(&ancestor));
        assert!(suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &[old_child, new_child]
        ));
    }

    #[test]
    fn child_echo_folds_reverse_ancestor_and_successor_child_renames() {
        let temp = tempfile::tempdir().expect("tempdir");
        let parent = temp.path().join("A");
        let moved_parent = temp.path().join("DISABLED A");
        let old_child = parent.join("DISABLED X");
        let new_child = parent.join("X");
        std::fs::create_dir_all(&old_child).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let mut paths = Vec::new();
        for (old_path, new_path) in [
            (old_child.clone(), new_child.clone()),
            (parent.clone(), moved_parent.clone()),
            (moved_parent.join("X"), moved_parent.join("DISABLED X")),
            (moved_parent.clone(), parent.clone()),
        ] {
            let evidence = record_rename(&suppressor, &session, &old_path, &new_path);
            std::fs::rename(&old_path, &new_path).expect("rename");
            assert!(suppressor.commit_expected_rename_echoes(&evidence));
            paths.push([old_path, new_path]);
        }
        for echo in paths {
            assert!(suppressor.consume_expected_rename_echo(
                "game",
                &session,
                &paired_rename(),
                &echo
            ));
        }
    }

    #[test]
    fn child_echo_waits_for_planned_ancestor_and_rejects_parent_reoccupation() {
        let temp = tempfile::tempdir().expect("tempdir");
        let parent = temp.path().join("A");
        let moved_parent = temp.path().join("DISABLED A");
        let old_child = parent.join("DISABLED X");
        let new_child = parent.join("X");
        std::fs::create_dir_all(&old_child).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let child = record_rename(&suppressor, &session, &old_child, &new_child);
        std::fs::rename(&old_child, &new_child).expect("child rename");
        assert!(suppressor.commit_expected_rename_echoes(&child));
        let ancestor = record_rename(&suppressor, &session, &parent, &moved_parent);
        std::fs::rename(&parent, &moved_parent).expect("parent rename");
        let (observer, dirty) = dirty_observer();
        assert!(suppressor.observe_expected_rename_echo(
            "game",
            &session,
            &EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            std::slice::from_ref(&old_child),
            observer
        ));
        assert_eq!(dirty.load(Ordering::Relaxed), 0);
        assert!(suppressor.commit_expected_rename_echoes(&ancestor));
        assert_eq!(dirty.load(Ordering::Relaxed), 0);
        // Consumed parent evidence remains needed by an outstanding child half.
        assert!(suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired_rename(),
            &[parent.clone(), moved_parent]
        ));
        suppressor
            .mark_rename_echoes_reconciled_through(&session, suppressor.expected_echo_watermark());
        assert!(lock(&suppressor.expected_rename_echoes)
            .edges
            .iter()
            .any(|edge| edge.belongs_to(&ancestor)));
        std::fs::create_dir(&parent).expect("external replacement parent");
        assert!(!suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            &[new_child]
        ));
    }

    #[test]
    fn folded_reverse_rename_echo_completes_the_maximal_committed_chain() {
        for count in [2, 3, 4] {
            let temp = tempfile::tempdir().expect("tempdir");
            let enabled = temp.path().join("Alice");
            let disabled = temp.path().join("DISABLED Alice");
            std::fs::create_dir(&enabled).expect("source");
            let suppressor = WatcherSuppressor::new(false);
            let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
            suppressor.begin_session(&session);
            for index in 0..count {
                let (old_path, new_path) = if index % 2 == 0 {
                    (&enabled, &disabled)
                } else {
                    (&disabled, &enabled)
                };
                let evidence = record_rename(&suppressor, &session, old_path, new_path);
                std::fs::rename(old_path, new_path).expect("rename");
                assert!(suppressor.commit_expected_rename_echoes(&evidence));
            }
            let endpoint = if count % 2 == 0 { &enabled } else { &disabled };
            let (observer, dirty) = dirty_observer();
            assert!(
                suppressor.observe_expected_rename_echo(
                    "game",
                    &session,
                    &paired_rename(),
                    &[enabled.clone(), endpoint.clone()],
                    observer,
                ),
                "folded {count}-edge observation"
            );
            {
                let mut ledger = lock(&suppressor.expected_rename_echoes);
                assert!(
                    ledger.edges.iter().all(PendingRenameEcho::completed),
                    "all {count} edges must be covered"
                );
                for edge in &mut ledger.edges {
                    edge.expires_at = Instant::now();
                }
            }
            expire_edges(&suppressor.expected_rename_echoes, &suppressor.repair);
            assert!(!suppressor.has_unrepaired_drops());
            assert_eq!(dirty.load(Ordering::Relaxed), 0);
        }
    }

    #[test]
    fn folded_pending_reverse_echo_waits_for_commit_and_releases_abort() {
        for outcome in ["commit", "abort", "expire"] {
            let temp = tempfile::tempdir().expect("tempdir");
            let enabled = temp.path().join("Alice");
            let disabled = temp.path().join("DISABLED Alice");
            std::fs::create_dir(&enabled).expect("source");
            let suppressor = WatcherSuppressor::new(false);
            let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
            suppressor.begin_session(&session);
            let first = record_rename(&suppressor, &session, &enabled, &disabled);
            std::fs::rename(&enabled, &disabled).expect("disable");
            assert!(suppressor.commit_expected_rename_echoes(&first));
            let second = record_rename(&suppressor, &session, &disabled, &enabled);
            std::fs::rename(&disabled, &enabled).expect("enable before durable commit");
            let paths = [enabled.clone(), enabled.clone()];
            let (observer, dirty) = dirty_observer();
            assert!(suppressor.observe_expected_rename_echo(
                "game",
                &session,
                &paired_rename(),
                &paths,
                observer
            ));
            assert!(!suppressor.is_committed_rename_echo(&session, &paired_rename(), &paths));
            assert_eq!(dirty.load(Ordering::Relaxed), 0);
            if outcome == "commit" {
                assert!(suppressor.commit_expected_rename_echoes(&second));
            } else if outcome == "abort" {
                std::fs::rename(&enabled, &disabled).expect("rollback");
                suppressor.discard_expected_rename_echoes(&second);
            }
            for edge in &mut lock(&suppressor.expected_rename_echoes).edges {
                edge.expires_at = Instant::now();
            }
            expire_edges(&suppressor.expected_rename_echoes, &suppressor.repair);
            assert_eq!(suppressor.has_unrepaired_drops(), outcome != "commit");
            assert_eq!(dirty.load(Ordering::Relaxed) == 0, outcome == "commit");
        }
    }

    #[test]
    fn folded_rename_rejects_replaced_discontinuous_and_foreign_chains() {
        for failure in ["replacement", "discontinuous", "foreign"] {
            let temp = tempfile::tempdir().expect("tempdir");
            let enabled = temp.path().join("Alice");
            let disabled = temp.path().join("DISABLED Alice");
            std::fs::create_dir(&enabled).expect("source");
            let suppressor = WatcherSuppressor::new(false);
            let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
            suppressor.begin_session(&session);
            for (old_path, new_path) in [(&enabled, &disabled), (&disabled, &enabled)] {
                let evidence = record_rename(&suppressor, &session, old_path, new_path);
                std::fs::rename(old_path, new_path).expect("rename");
                assert!(suppressor.commit_expected_rename_echoes(&evidence));
            }
            match failure {
                "replacement" => {
                    std::fs::rename(&enabled, temp.path().join("OriginalAlice"))
                        .expect("move original");
                    std::fs::create_dir(&enabled).expect("replacement");
                }
                "discontinuous" => {
                    lock(&suppressor.expected_rename_echoes).edges[1].old_key =
                        rename_location_key(&temp.path().join("Untracked"))
                }
                "foreign" => {
                    lock(&suppressor.expected_rename_echoes).edges[1].game_id = "other".into()
                }
                _ => unreachable!(),
            }
            let (observer, _) = dirty_observer();
            assert!(
                !suppressor.observe_expected_rename_echo(
                    "game",
                    &session,
                    &paired_rename(),
                    &[enabled.clone(), enabled],
                    observer,
                ),
                "must reject {failure}"
            );
            assert!(lock(&suppressor.expected_rename_echoes)
                .edges
                .iter()
                .all(|edge| !edge.completed()));
        }
    }

    #[test]
    fn rapid_reverse_lineage_consumes_all_delayed_echoes_without_repair() {
        let temp = tempfile::tempdir().expect("tempdir");
        let enabled = temp.path().join("Alice");
        let disabled = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&enabled).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        suppressor.begin_session(&session);
        let mut renames = Vec::new();
        for index in 0..1000 {
            let (old_path, new_path) = if index % 2 == 0 {
                (&enabled, &disabled)
            } else {
                (&disabled, &enabled)
            };
            let evidence = record_rename(&suppressor, &session, old_path, new_path);
            std::fs::rename(old_path, new_path).expect("rename");
            assert!(suppressor.commit_expected_rename_echoes(&evidence));
            renames.push([old_path.clone(), new_path.clone()]);
        }
        for paths in renames {
            assert!(suppressor.consume_expected_rename_echo(
                "game",
                &session,
                &paired_rename(),
                &paths
            ));
        }
        assert!(enabled.is_dir());
        assert!(!disabled.exists());
        assert!(!suppressor.has_unrepaired_drops());
        suppressor
            .mark_rename_echoes_reconciled_through(&session, suppressor.expected_echo_watermark());
        assert!(lock(&suppressor.expected_rename_echoes).edges.is_empty());
    }

    #[test]
    fn verified_watermark_preserves_newer_and_planned_evidence() {
        let temp = tempfile::tempdir().expect("tempdir");
        let enabled = temp.path().join("Alice");
        let disabled = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&enabled).expect("source");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let first = record_rename(&suppressor, &session, &enabled, &disabled);
        std::fs::rename(&enabled, &disabled).expect("disable");
        assert!(suppressor.commit_expected_rename_echoes(&first));
        let watermark = suppressor.expected_echo_watermark();
        let second = record_rename(&suppressor, &session, &disabled, &enabled);
        suppressor.mark_rename_echoes_reconciled_through(&session, watermark);
        let ledger = lock(&suppressor.expected_rename_echoes);
        assert_eq!(ledger.edges.len(), 2);
        assert!(ledger.edges[1].belongs_to(&second));
    }

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
    fn delayed_reverse_rename_echo_uses_latest_identity_endpoint() {
        let temp = tempfile::tempdir().expect("tempdir");
        let enabled = temp.path().join("Alice");
        let disabled = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&enabled).expect("source");
        let identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&enabled)
            .expect("identity");
        let suppressor = WatcherSuppressor::new(false);
        let session = WatcherSession::new_with_runtime_config(1, temp.path(), None);
        let first = suppressor.expect_rename_echoes(
            "game",
            &session,
            [ExpectedRenameEcho {
                old_path: enabled.clone(),
                new_path: disabled.clone(),
                expected_identity: identity.clone(),
            }],
        );
        std::fs::rename(&enabled, &disabled).expect("disable");
        assert!(suppressor.commit_expected_rename_echoes(&first));
        let second = suppressor.expect_rename_echoes(
            "game",
            &session,
            [ExpectedRenameEcho {
                old_path: disabled.clone(),
                new_path: enabled.clone(),
                expected_identity: identity,
            }],
        );
        std::fs::rename(&disabled, &enabled).expect("enable");
        assert!(suppressor.commit_expected_rename_echoes(&second));

        let paired = EventKind::Modify(ModifyKind::Name(RenameMode::Both));
        assert!(suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired,
            &[enabled.clone(), disabled.clone()]
        ));
        assert!(suppressor.consume_expected_rename_echo(
            "game",
            &session,
            &paired,
            &[disabled, enabled]
        ));
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
        assert!(suppressor.commit_expected_rename_echoes(&evidence));
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
