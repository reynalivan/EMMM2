use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::platform::fs::atomic_file::atomic_write;
use crate::shared::errors::AppError;

const JOURNAL_FORMAT_VERSION: u32 = 1;
const ACTIVE_STATE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    Planned,
    Applying,
    Applied,
    DbCommitted,
    RollingBack,
    RolledBack,
    Completed,
    FailedNeedsRepair,
}

impl OperationStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::RolledBack | Self::Completed | Self::FailedNeedsRepair
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Planned,
    Applied,
    Skipped,
    RolledBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepSettlement {
    Applied,
    Skipped,
    RolledBack,
}

impl StepSettlement {
    fn status(self) -> StepStatus {
        match self {
            Self::Applied => StepStatus::Applied,
            Self::Skipped => StepStatus::Skipped,
            Self::RolledBack => StepStatus::RolledBack,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseProjectionStatus {
    NotStarted,
    Committed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStepKind {
    Rename,
    Quarantine,
    HardlinkReplace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedStep {
    pub sequence: u32,
    pub kind: MutationStepKind,
    pub old_path: Option<PathBuf>,
    pub new_path: Option<PathBuf>,
    pub stage_path: Option<PathBuf>,
    pub expected_identity: Option<String>,
}

impl PlannedStep {
    pub fn rename(sequence: u32, old_path: PathBuf, new_path: PathBuf) -> Self {
        Self {
            sequence,
            kind: MutationStepKind::Rename,
            old_path: Some(old_path),
            new_path: Some(new_path),
            stage_path: None,
            expected_identity: None,
        }
    }

    pub fn quarantine(sequence: u32, source: PathBuf, quarantine: PathBuf) -> Self {
        Self {
            sequence,
            kind: MutationStepKind::Quarantine,
            old_path: Some(source),
            new_path: Some(quarantine),
            stage_path: None,
            expected_identity: None,
        }
    }

    pub fn hardlink_replace(
        sequence: u32,
        keeper: PathBuf,
        target: PathBuf,
        backup: PathBuf,
        keeper_identity: String,
    ) -> Self {
        Self {
            sequence,
            kind: MutationStepKind::HardlinkReplace,
            old_path: Some(keeper),
            new_path: Some(target),
            stage_path: Some(backup),
            expected_identity: Some(keeper_identity),
        }
    }

    pub fn with_stage_path(mut self, stage_path: PathBuf) -> Self {
        self.stage_path = Some(stage_path);
        self
    }

    pub fn with_expected_identity(mut self, expected_identity: Option<String>) -> Self {
        self.expected_identity = expected_identity;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationPlan {
    pub kind: String,
    pub game_id: String,
    pub steps: Vec<PlannedStep>,
}

impl OperationPlan {
    pub fn new(
        kind: impl Into<String>,
        game_id: impl Into<String>,
        steps: Vec<PlannedStep>,
    ) -> Self {
        Self {
            kind: kind.into(),
            game_id: game_id.into(),
            steps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationStep {
    pub sequence: u32,
    pub kind: MutationStepKind,
    pub old_path: Option<PathBuf>,
    pub new_path: Option<PathBuf>,
    pub stage_path: Option<PathBuf>,
    #[serde(default)]
    pub expected_identity: Option<String>,
    pub status: StepStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub kind: String,
    pub game_id: String,
    pub created_at: String,
    pub status: OperationStatus,
    pub steps: Vec<OperationStep>,
    pub database_projection_status: DatabaseProjectionStatus,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JournalSnapshot {
    format_version: u32,
    revision: u64,
    operation_ids: Vec<String>,
    entries: Vec<Operation>,
    checksum: String,
}

#[derive(Serialize)]
struct JournalSnapshotPayload<'a> {
    format_version: u32,
    revision: u64,
    operation_ids: &'a [String],
    entries: &'a [Operation],
}

struct JournalState {
    revision: u64,
    entries: Vec<Operation>,
    active_revisions: HashMap<String, u64>,
}

struct LoadedJournal {
    revision: u64,
    entries: Vec<Operation>,
    needs_rewrite: bool,
}

enum DecodedJournal {
    Versioned {
        revision: u64,
        entries: Vec<Operation>,
        checksum: String,
    },
    Unversioned(Vec<Operation>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveOperationStateSnapshot {
    format_version: u32,
    revision: u64,
    operation_id: String,
    status: OperationStatus,
    database_projection_status: DatabaseProjectionStatus,
    step_statuses: String,
    last_error: Option<String>,
    checksum: String,
}

#[derive(Serialize)]
struct ActiveOperationStatePayload<'a> {
    format_version: u32,
    revision: u64,
    operation_id: &'a str,
    status: OperationStatus,
    database_projection_status: DatabaseProjectionStatus,
    step_statuses: &'a str,
    last_error: &'a Option<String>,
}

struct ActiveOperationState {
    revision: u64,
    operation_id: String,
    status: OperationStatus,
    database_projection_status: DatabaseProjectionStatus,
    step_statuses: Vec<StepStatus>,
    last_error: Option<String>,
    checksum: String,
}

#[derive(Clone, Copy)]
enum JournalArtifactKind {
    Recovery,
    Temp,
}

impl JournalArtifactKind {
    fn label(self) -> &'static str {
        match self {
            Self::Recovery => "recover",
            Self::Temp => "tmp",
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct JournalPersistenceMetrics {
    pub writes: u64,
    pub serialized_bytes: u64,
}

pub struct OperationJournal {
    path: Option<PathBuf>,
    active_state_root: Option<PathBuf>,
    max_history: usize,
    state: Mutex<JournalState>,
    #[cfg(test)]
    persistence_metrics: Mutex<JournalPersistenceMetrics>,
}

impl OperationJournal {
    pub fn new() -> Self {
        Self {
            path: None,
            active_state_root: None,
            max_history: 1,
            state: Mutex::new(JournalState {
                revision: 0,
                entries: Vec::new(),
                active_revisions: HashMap::new(),
            }),
            #[cfg(test)]
            persistence_metrics: Mutex::new(JournalPersistenceMetrics::default()),
        }
    }

    pub fn open(path: impl AsRef<Path>, max_history: usize) -> Result<Self, AppError> {
        if max_history == 0 {
            return Err(AppError::Validation(
                "Mutation journal history limit must be greater than zero".to_string(),
            ));
        }

        let path = path.as_ref().to_path_buf();
        let mut loaded = load_journal(&path)?;
        let initial_len = loaded.entries.len();
        Self::trim(&mut loaded.entries, max_history);
        if loaded.needs_rewrite || loaded.entries.len() != initial_len {
            loaded.revision = next_revision(loaded.revision)?;
            persist_snapshot(&path, &loaded.entries, loaded.revision)?;
        }
        let active_state_root = journal_active_state_root(&path)?;
        let active_revisions = load_active_states(&mut loaded.entries, &active_state_root)?;

        Ok(Self {
            path: Some(path),
            active_state_root: Some(active_state_root),
            max_history,
            state: Mutex::new(JournalState {
                revision: loaded.revision,
                entries: loaded.entries,
                active_revisions,
            }),
            #[cfg(test)]
            persistence_metrics: Mutex::new(JournalPersistenceMetrics::default()),
        })
    }

    pub fn plan_operation(&self, plan: OperationPlan) -> Result<String, AppError> {
        validate_plan(&plan)?;
        let id = Uuid::new_v4().to_string();
        let operation = Operation {
            id: id.clone(),
            kind: plan.kind,
            game_id: plan.game_id,
            created_at: chrono::Utc::now().to_rfc3339(),
            status: OperationStatus::Planned,
            steps: plan
                .steps
                .into_iter()
                .map(|step| OperationStep {
                    sequence: step.sequence,
                    kind: step.kind,
                    old_path: step.old_path,
                    new_path: step.new_path,
                    stage_path: step.stage_path,
                    expected_identity: step.expected_identity,
                    status: StepStatus::Planned,
                })
                .collect(),
            database_projection_status: DatabaseProjectionStatus::NotStarted,
            last_error: None,
        };

        let mut state = self.lock_state()?;
        let mut next = state.entries.clone();
        next.push(operation);
        Self::trim(&mut next, self.max_history);
        let revision = next_revision(state.revision)?;
        self.persist(&next, revision)?;
        state.entries = next;
        state.revision = revision;
        state.active_revisions.insert(id.clone(), 0);
        Ok(id)
    }

    pub fn mark_applying(&self, id: &str) -> Result<(), AppError> {
        self.mutate_operation(id, |operation| {
            require_status(operation, &[OperationStatus::Planned])?;
            operation.status = OperationStatus::Applying;
            Ok(())
        })
    }

    pub fn mark_step_applied(&self, id: &str, sequence: u32) -> Result<(), AppError> {
        self.mutate_operation(id, |operation| {
            require_status(
                operation,
                &[OperationStatus::Applying, OperationStatus::Applied],
            )?;
            let step = operation
                .steps
                .iter_mut()
                .find(|step| step.sequence == sequence)
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "Mutation journal step {sequence} for operation {id}"
                    ))
                })?;
            if step.status != StepStatus::Planned {
                return Err(AppError::Validation(format!(
                    "Mutation journal step {sequence} is not planned"
                )));
            }
            step.status = StepStatus::Applied;
            refresh_application_status(operation);
            Ok(())
        })
    }

    pub fn mark_step_skipped(&self, id: &str, sequence: u32) -> Result<(), AppError> {
        self.mutate_operation(id, |operation| {
            require_status(
                operation,
                &[OperationStatus::Applying, OperationStatus::Applied],
            )?;
            let step = operation
                .steps
                .iter_mut()
                .find(|step| step.sequence == sequence)
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "Mutation journal step {sequence} for operation {id}"
                    ))
                })?;
            if step.status != StepStatus::Planned {
                return Err(AppError::Validation(format!(
                    "Mutation journal step {sequence} is not planned"
                )));
            }
            step.status = StepStatus::Skipped;
            refresh_application_status(operation);
            Ok(())
        })
    }

    /// Validates the complete batch before applying it and persists one
    /// active-state snapshot, so callers never expose partial settlement.
    pub fn settle_steps(
        &self,
        id: &str,
        settlements: &[(u32, StepSettlement)],
    ) -> Result<(), AppError> {
        if settlements.is_empty() {
            return Ok(());
        }

        self.mutate_operation(id, |operation| {
            let operation_status = operation.status;
            require_status(
                operation,
                &[
                    OperationStatus::Applying,
                    OperationStatus::Applied,
                    OperationStatus::RollingBack,
                ],
            )?;

            let mut sequences = HashSet::with_capacity(settlements.len());
            let step_indices = operation
                .steps
                .iter()
                .enumerate()
                .map(|(index, step)| (step.sequence, index))
                .collect::<HashMap<_, _>>();
            let mut updates = Vec::with_capacity(settlements.len());
            for (sequence, settlement) in settlements {
                if !sequences.insert(*sequence) {
                    return Err(AppError::Validation(format!(
                        "Mutation journal step {sequence} is settled more than once"
                    )));
                }

                let step_index = *step_indices.get(sequence).ok_or_else(|| {
                        AppError::NotFound(format!(
                            "Mutation journal step {sequence} for operation {id}"
                        ))
                    })?;
                let step = &operation.steps[step_index];
                let valid = matches!((operation_status, step.status, *settlement),
                    (
                        OperationStatus::Applying | OperationStatus::Applied,
                        StepStatus::Planned,
                        StepSettlement::Applied | StepSettlement::Skipped,
                    ) |
                    (
                        OperationStatus::RollingBack,
                        StepStatus::Planned | StepStatus::Applied,
                        StepSettlement::RolledBack,
                    )
                );
                if !valid {
                    return Err(AppError::Validation(format!(
                        "Mutation journal step {sequence} cannot transition from {:?} to {:?} while operation {id} is {:?}",
                        step.status, settlement, operation_status
                    )));
                }
                updates.push((step_index, settlement.status()));
            }

            for (step_index, status) in updates {
                operation.steps[step_index].status = status;
            }
            refresh_application_status(operation);
            Ok(())
        })
    }

    pub fn mark_db_committed(&self, id: &str) -> Result<(), AppError> {
        self.mutate_operation(id, |operation| {
            require_status(
                operation,
                &[OperationStatus::Applying, OperationStatus::Applied],
            )?;
            if operation
                .steps
                .iter()
                .any(|step| step.status == StepStatus::Planned)
            {
                return Err(AppError::Validation(format!(
                    "Mutation operation {id} has unsettled filesystem steps"
                )));
            }
            operation.database_projection_status = DatabaseProjectionStatus::Committed;
            operation.status = OperationStatus::DbCommitted;
            Ok(())
        })
    }

    pub fn complete(&self, id: &str) -> Result<(), AppError> {
        self.mutate_operation(id, |operation| {
            require_status(operation, &[OperationStatus::DbCommitted])?;
            if operation.database_projection_status != DatabaseProjectionStatus::Committed {
                return Err(AppError::Validation(format!(
                    "Mutation operation {id} cannot complete before its database projection commits"
                )));
            }
            operation.status = OperationStatus::Completed;
            Ok(())
        })
    }

    pub fn begin_rollback(&self, id: &str) -> Result<(), AppError> {
        self.mutate_operation(id, |operation| {
            require_status(
                operation,
                &[
                    OperationStatus::Planned,
                    OperationStatus::Applying,
                    OperationStatus::Applied,
                    OperationStatus::RollingBack,
                ],
            )?;
            operation.status = OperationStatus::RollingBack;
            Ok(())
        })
    }

    pub fn mark_step_rolled_back(&self, id: &str, sequence: u32) -> Result<(), AppError> {
        self.mutate_operation(id, |operation| {
            require_status(
                operation,
                &[
                    OperationStatus::Applying,
                    OperationStatus::Applied,
                    OperationStatus::RollingBack,
                ],
            )?;
            let step = operation
                .steps
                .iter_mut()
                .find(|step| step.sequence == sequence)
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "Mutation journal step {sequence} for operation {id}"
                    ))
                })?;
            step.status = StepStatus::RolledBack;
            refresh_application_status(operation);
            Ok(())
        })
    }

    pub fn finish_rollback(&self, id: &str) -> Result<(), AppError> {
        self.mutate_operation(id, |operation| {
            require_status(operation, &[OperationStatus::RollingBack])?;
            if operation
                .steps
                .iter()
                .any(|step| !matches!(step.status, StepStatus::RolledBack | StepStatus::Skipped))
            {
                return Err(AppError::Validation(format!(
                    "Mutation operation {id} still has steps that were not rolled back"
                )));
            }
            operation.status = OperationStatus::RolledBack;
            Ok(())
        })
    }

    pub fn fail(&self, id: &str, error: impl Into<String>) -> Result<(), AppError> {
        let error = error.into();
        self.mutate_operation(id, |operation| {
            if operation.status.is_terminal() {
                return Err(AppError::Validation(format!(
                    "Mutation operation {id} is already terminal"
                )));
            }
            if operation.database_projection_status != DatabaseProjectionStatus::Committed {
                operation.database_projection_status = DatabaseProjectionStatus::Failed;
            }
            operation.last_error = Some(error);
            operation.status = OperationStatus::FailedNeedsRepair;
            Ok(())
        })
    }

    pub fn finalize_recovered_commit(&self, id: &str) -> Result<(), AppError> {
        self.mutate_operation(id, |operation| {
            if operation.database_projection_status != DatabaseProjectionStatus::Committed {
                return Err(AppError::Validation(format!(
                    "Mutation operation {id} has no committed database projection"
                )));
            }
            operation.status = OperationStatus::Completed;
            Ok(())
        })
    }

    pub fn entries(&self) -> Vec<Operation> {
        self.state
            .lock()
            .expect("mutation journal lock poisoned")
            .entries
            .clone()
    }

    fn mutate_operation<R>(
        &self,
        id: &str,
        mutate: impl FnOnce(&mut Operation) -> Result<R, AppError>,
    ) -> Result<R, AppError> {
        let mut state = self.lock_state()?;
        let mut next = state.entries.clone();
        let operation = next
            .iter_mut()
            .find(|operation| operation.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Mutation journal operation {id}")))?;
        let output = mutate(operation)?;
        let updated_operation = operation.clone();
        if updated_operation.status.is_terminal() {
            let revision = next_revision(state.revision)?;
            self.persist(&next, revision)?;
            self.remove_active_state(&updated_operation.id);
            state.entries = next;
            state.revision = revision;
            state.active_revisions.remove(id);
        } else {
            let active_revision = next_revision(*state.active_revisions.get(id).unwrap_or(&0))?;
            self.persist_active_state(&updated_operation, active_revision)?;
            state.entries = next;
            state
                .active_revisions
                .insert(id.to_string(), active_revision);
        }
        Ok(output)
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, JournalState>, AppError> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("Mutation journal lock poisoned".to_string()))
    }

    fn persist(&self, entries: &[Operation], revision: u64) -> Result<(), AppError> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        let bytes = serialize_snapshot(entries, revision)?;
        self.record_persistence_metrics(bytes.len())?;
        atomic_write(path, &bytes)
    }

    fn persist_active_state(&self, operation: &Operation, revision: u64) -> Result<(), AppError> {
        let Some(root) = self.active_state_root.as_ref() else {
            return Ok(());
        };
        let path = active_state_path(root, &operation.id)?;
        let bytes = serialize_active_state(operation, revision)?;
        self.record_persistence_metrics(bytes.len())?;
        atomic_write(&path, &bytes)
    }

    fn remove_active_state(&self, operation_id: &str) {
        let Some(root) = self.active_state_root.as_ref() else {
            return;
        };
        let Ok(path) = active_state_path(root, operation_id) else {
            return;
        };
        if path.exists() {
            if let Err(error) = std::fs::remove_file(&path) {
                log::warn!(
                    "Terminal mutation journal progress state remains at {}: {error}",
                    path.display()
                );
            }
        }
    }

    #[cfg(test)]
    fn record_persistence_metrics(&self, bytes: usize) -> Result<(), AppError> {
        let mut metrics = self.persistence_metrics.lock().map_err(|_| {
            AppError::Internal("Mutation journal persistence metrics lock poisoned".to_string())
        })?;
        metrics.writes += 1;
        metrics.serialized_bytes += bytes as u64;
        Ok(())
    }

    #[cfg(not(test))]
    fn record_persistence_metrics(&self, _bytes: usize) -> Result<(), AppError> {
        Ok(())
    }

    fn trim(entries: &mut Vec<Operation>, max_history: usize) {
        while entries.len() > max_history {
            let Some(index) = entries
                .iter()
                .position(|operation| operation.status.is_terminal())
            else {
                break;
            };
            entries.remove(index);
        }
    }

    #[cfg(test)]
    pub(crate) fn reset_persistence_metrics(&self) {
        *self
            .persistence_metrics
            .lock()
            .expect("mutation journal persistence metrics lock poisoned") =
            JournalPersistenceMetrics::default();
    }

    #[cfg(test)]
    pub(crate) fn persistence_metrics(&self) -> JournalPersistenceMetrics {
        *self
            .persistence_metrics
            .lock()
            .expect("mutation journal persistence metrics lock poisoned")
    }

    #[cfg(test)]
    pub(crate) fn seed_terminal_history_for_benchmark(
        &self,
        history_count: usize,
    ) -> Result<(), AppError> {
        let mut state = self.lock_state()?;
        let entries = (0..history_count)
            .map(|index| benchmark_history_operation(index as u32))
            .collect::<Vec<_>>();
        let revision = next_revision(state.revision)?;
        self.persist(&entries, revision)?;
        state.entries = entries;
        state.revision = revision;
        Ok(())
    }
}

impl Default for OperationJournal {
    fn default() -> Self {
        Self::new()
    }
}

fn next_revision(revision: u64) -> Result<u64, AppError> {
    revision
        .checked_add(1)
        .ok_or_else(|| AppError::Internal("Mutation journal revision overflow".to_string()))
}

fn persist_snapshot(path: &Path, entries: &[Operation], revision: u64) -> Result<(), AppError> {
    atomic_write(path, &serialize_snapshot(entries, revision)?)
}

fn serialize_snapshot(entries: &[Operation], revision: u64) -> Result<Vec<u8>, AppError> {
    let operation_ids = entries
        .iter()
        .map(|operation| operation.id.clone())
        .collect::<Vec<_>>();
    let checksum = snapshot_checksum(JOURNAL_FORMAT_VERSION, revision, &operation_ids, entries)?;
    Ok(serde_json::to_vec(&JournalSnapshot {
        format_version: JOURNAL_FORMAT_VERSION,
        revision,
        operation_ids,
        entries: entries.to_vec(),
        checksum,
    })?)
}

fn snapshot_checksum(
    format_version: u32,
    revision: u64,
    operation_ids: &[String],
    entries: &[Operation],
) -> Result<String, AppError> {
    let payload = JournalSnapshotPayload {
        format_version,
        revision,
        operation_ids,
        entries,
    };
    Ok(blake3::hash(&serde_json::to_vec(&payload)?)
        .to_hex()
        .to_string())
}

fn journal_active_state_root(journal_path: &Path) -> Result<PathBuf, AppError> {
    let parent = journal_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = journal_path
        .file_name()
        .ok_or_else(|| {
            AppError::Validation(format!(
                "Invalid mutation journal path: {}",
                journal_path.display()
            ))
        })?
        .to_string_lossy();
    Ok(parent.join(format!("{file_name}.active")))
}

fn active_state_path(root: &Path, operation_id: &str) -> Result<PathBuf, AppError> {
    Uuid::parse_str(operation_id).map_err(|_| {
        AppError::Validation(format!(
            "Mutation journal active state requires UUID operation id: {operation_id}"
        ))
    })?;
    Ok(root.join(format!("{operation_id}.json")))
}

fn load_active_states(
    entries: &mut [Operation],
    root: &Path,
) -> Result<HashMap<String, u64>, AppError> {
    let mut revisions = HashMap::new();
    for operation in entries
        .iter_mut()
        .filter(|operation| !operation.status.is_terminal())
    {
        let path = active_state_path(root, &operation.id)?;
        if let Some(active_state) = load_active_state(&path, operation)? {
            apply_active_state(operation, &active_state)?;
            revisions.insert(operation.id.clone(), active_state.revision);
        } else {
            // A snapshot written before compact progress existed remains recoverable from the
            // global planned state. Its next transition creates the sidecar.
            revisions.insert(operation.id.clone(), 0);
        }
    }
    Ok(revisions)
}

fn load_active_state(
    path: &Path,
    operation: &Operation,
) -> Result<Option<ActiveOperationState>, AppError> {
    let artifacts = replacement_artifacts(path)?;
    if path.exists() {
        // The canonical state is committed. A later temporary artifact is not proof that its
        // replacement completed, so it cannot override the canonical state.
        let state = read_active_state(path, operation)?;
        validate_noncanonical_active_state_artifacts(&artifacts, operation);
        return Ok(Some(state));
    }

    if artifacts.is_empty() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
    for (artifact_path, kind) in artifacts {
        let state = read_active_state(&artifact_path, operation).map_err(|_| {
            repair_required(
                path,
                &format!("{} active progress artifact is invalid", kind.label()),
            )
        })?;
        candidates.push((artifact_path, state));
    }

    let newest_revision = candidates
        .iter()
        .map(|(_, state)| state.revision)
        .max()
        .expect("active progress artifacts cannot be empty");
    let mut newest = candidates
        .into_iter()
        .filter(|(_, state)| state.revision == newest_revision);
    let (_, mut selected) = newest
        .next()
        .expect("newest active progress artifact must exist");
    if newest.any(|(_, other)| other.checksum != selected.checksum) {
        return Err(repair_required(
            path,
            "multiple active progress artifacts have the same revision but different contents",
        ));
    }

    selected.revision = next_revision(selected.revision)?;
    persist_active_state_snapshot(path, &selected)?;
    Ok(Some(selected))
}

fn read_active_state(path: &Path, operation: &Operation) -> Result<ActiveOperationState, AppError> {
    decode_active_state(&std::fs::read(path)?, operation)
}

fn validate_noncanonical_active_state_artifacts(
    artifacts: &[(PathBuf, JournalArtifactKind)],
    operation: &Operation,
) {
    for (artifact_path, kind) in artifacts {
        match read_active_state(artifact_path, operation) {
            Ok(state) => log::warn!(
                "Retaining stale mutation progress {} artifact at revision {}: {}",
                kind.label(),
                state.revision,
                artifact_path.display()
            ),
            Err(error) => log::warn!(
                "Retaining invalid mutation progress {} artifact {}: {error}",
                kind.label(),
                artifact_path.display()
            ),
        }
    }
}

fn serialize_active_state(operation: &Operation, revision: u64) -> Result<Vec<u8>, AppError> {
    if operation.status.is_terminal() {
        return Err(AppError::Validation(format!(
            "Terminal mutation operation {} cannot write active progress",
            operation.id
        )));
    }
    let step_statuses = encode_step_statuses(&operation.steps);
    serialize_active_state_fields(
        revision,
        &operation.id,
        operation.status,
        operation.database_projection_status,
        &step_statuses,
        &operation.last_error,
    )
}

fn persist_active_state_snapshot(
    path: &Path,
    state: &ActiveOperationState,
) -> Result<(), AppError> {
    let bytes = serialize_active_state_fields(
        state.revision,
        &state.operation_id,
        state.status,
        state.database_projection_status,
        &encode_step_statuses_from_statuses(&state.step_statuses),
        &state.last_error,
    )?;
    atomic_write(path, &bytes)
}

fn serialize_active_state_fields(
    revision: u64,
    operation_id: &str,
    status: OperationStatus,
    database_projection_status: DatabaseProjectionStatus,
    step_statuses: &str,
    last_error: &Option<String>,
) -> Result<Vec<u8>, AppError> {
    let checksum = active_state_checksum(
        ACTIVE_STATE_FORMAT_VERSION,
        revision,
        operation_id,
        status,
        database_projection_status,
        step_statuses,
        last_error,
    )?;
    Ok(serde_json::to_vec(&ActiveOperationStateSnapshot {
        format_version: ACTIVE_STATE_FORMAT_VERSION,
        revision,
        operation_id: operation_id.to_string(),
        status,
        database_projection_status,
        step_statuses: step_statuses.to_string(),
        last_error: last_error.clone(),
        checksum,
    })?)
}

fn active_state_checksum(
    format_version: u32,
    revision: u64,
    operation_id: &str,
    status: OperationStatus,
    database_projection_status: DatabaseProjectionStatus,
    step_statuses: &str,
    last_error: &Option<String>,
) -> Result<String, AppError> {
    let payload = ActiveOperationStatePayload {
        format_version,
        revision,
        operation_id,
        status,
        database_projection_status,
        step_statuses,
        last_error,
    };
    Ok(blake3::hash(&serde_json::to_vec(&payload)?)
        .to_hex()
        .to_string())
}

fn decode_active_state(
    bytes: &[u8],
    operation: &Operation,
) -> Result<ActiveOperationState, AppError> {
    let snapshot = serde_json::from_slice::<ActiveOperationStateSnapshot>(bytes)?;
    if snapshot.format_version != ACTIVE_STATE_FORMAT_VERSION {
        return Err(AppError::Validation(format!(
            "Unsupported mutation progress format version {}",
            snapshot.format_version
        )));
    }
    if snapshot.revision == 0 {
        return Err(AppError::Validation(
            "Mutation progress revision must be greater than zero".to_string(),
        ));
    }
    if snapshot.operation_id != operation.id || snapshot.status.is_terminal() {
        return Err(AppError::Validation(
            "Mutation progress does not match an active operation".to_string(),
        ));
    }
    let expected_checksum = active_state_checksum(
        snapshot.format_version,
        snapshot.revision,
        &snapshot.operation_id,
        snapshot.status,
        snapshot.database_projection_status,
        &snapshot.step_statuses,
        &snapshot.last_error,
    )?;
    if snapshot.checksum != expected_checksum {
        return Err(AppError::Validation(
            "Mutation progress checksum does not match its contents".to_string(),
        ));
    }
    let step_statuses = decode_step_statuses(&snapshot.step_statuses, operation.steps.len())?;
    Ok(ActiveOperationState {
        revision: snapshot.revision,
        operation_id: snapshot.operation_id,
        status: snapshot.status,
        database_projection_status: snapshot.database_projection_status,
        step_statuses,
        last_error: snapshot.last_error,
        checksum: snapshot.checksum,
    })
}

fn apply_active_state(
    operation: &mut Operation,
    active_state: &ActiveOperationState,
) -> Result<(), AppError> {
    if active_state.operation_id != operation.id
        || active_state.step_statuses.len() != operation.steps.len()
    {
        return Err(AppError::Validation(
            "Mutation progress cannot be applied to this operation".to_string(),
        ));
    }
    for (step, status) in operation.steps.iter_mut().zip(&active_state.step_statuses) {
        step.status = *status;
    }
    operation.status = active_state.status;
    operation.database_projection_status = active_state.database_projection_status;
    operation.last_error = active_state.last_error.clone();
    Ok(())
}

fn encode_step_statuses(steps: &[OperationStep]) -> String {
    encode_statuses(steps.len(), steps.iter().map(|step| step.status))
}

fn encode_step_statuses_from_statuses(statuses: &[StepStatus]) -> String {
    encode_statuses(statuses.len(), statuses.iter().copied())
}

fn encode_statuses(status_count: usize, statuses: impl Iterator<Item = StepStatus>) -> String {
    let mut packed = vec![0_u8; status_count.div_ceil(4)];
    for (index, status) in statuses.enumerate() {
        packed[index / 4] |= step_status_code(status) << ((index % 4) * 2);
    }
    let mut encoded = String::with_capacity(packed.len() * 2);
    for byte in packed {
        write!(&mut encoded, "{byte:02x}").expect("writing to a string cannot fail");
    }
    encoded
}

fn decode_step_statuses(encoded: &str, count: usize) -> Result<Vec<StepStatus>, AppError> {
    let byte_count = count.div_ceil(4);
    if encoded.len() != byte_count * 2 || !encoded.is_ascii() {
        return Err(AppError::Validation(
            "Mutation progress step status encoding is invalid".to_string(),
        ));
    }
    let mut packed = Vec::with_capacity(byte_count);
    for chunk in encoded.as_bytes().chunks_exact(2) {
        let hex = std::str::from_utf8(chunk).map_err(|_| {
            AppError::Validation("Mutation progress step status encoding is invalid".to_string())
        })?;
        packed.push(u8::from_str_radix(hex, 16).map_err(|_| {
            AppError::Validation("Mutation progress step status encoding is invalid".to_string())
        })?);
    }
    if !count.is_multiple_of(4)
        && packed
            .last()
            .is_some_and(|byte| *byte >> ((count % 4) * 2) != 0)
    {
        return Err(AppError::Validation(
            "Mutation progress step status encoding has unused bits".to_string(),
        ));
    }
    (0..count)
        .map(|index| step_status_from_code((packed[index / 4] >> ((index % 4) * 2)) & 0b11))
        .collect()
}

fn step_status_code(status: StepStatus) -> u8 {
    match status {
        StepStatus::Planned => 0,
        StepStatus::Applied => 1,
        StepStatus::Skipped => 2,
        StepStatus::RolledBack => 3,
    }
}

fn step_status_from_code(code: u8) -> Result<StepStatus, AppError> {
    match code {
        0 => Ok(StepStatus::Planned),
        1 => Ok(StepStatus::Applied),
        2 => Ok(StepStatus::Skipped),
        3 => Ok(StepStatus::RolledBack),
        _ => Err(AppError::Validation(
            "Mutation progress step status encoding is invalid".to_string(),
        )),
    }
}

fn load_journal(path: &Path) -> Result<LoadedJournal, AppError> {
    let artifacts = replacement_artifacts(path)?;
    if path.exists() {
        // A surviving canonical file is the committed snapshot. A newer `.tmp` may be an
        // interrupted write that never committed and must not override it.
        return match read_journal(path)? {
            DecodedJournal::Versioned {
                revision, entries, ..
            } => {
                validate_noncanonical_artifacts(&artifacts);
                Ok(LoadedJournal {
                    revision,
                    entries,
                    needs_rewrite: false,
                })
            }
            DecodedJournal::Unversioned(entries) => {
                if artifacts.is_empty() {
                    Ok(LoadedJournal {
                        revision: 0,
                        entries,
                        needs_rewrite: true,
                    })
                } else {
                    Err(repair_required(
                        path,
                        "unversioned journal has recovery artifacts",
                    ))
                }
            }
        };
    }

    // The canonical gap occurs after the old snapshot became `.recover`; here revision
    // distinguishes that older recovery snapshot from a fully written replacement.
    load_missing_canonical_journal(path, artifacts)
}

fn load_missing_canonical_journal(
    path: &Path,
    artifacts: Vec<(PathBuf, JournalArtifactKind)>,
) -> Result<LoadedJournal, AppError> {
    if artifacts.is_empty() {
        return Ok(LoadedJournal {
            revision: 0,
            entries: Vec::new(),
            needs_rewrite: false,
        });
    }

    let mut versioned = Vec::new();
    let mut unversioned = Vec::new();
    for (artifact_path, kind) in artifacts {
        match read_journal(&artifact_path)
            .map_err(|_| repair_required(path, &format!("{} artifact is invalid", kind.label())))?
        {
            DecodedJournal::Versioned {
                revision,
                entries,
                checksum,
            } => versioned.push((artifact_path, revision, entries, checksum)),
            DecodedJournal::Unversioned(entries) => unversioned.push((artifact_path, entries)),
        }
    }

    if !versioned.is_empty() {
        if !unversioned.is_empty() {
            return Err(repair_required(
                path,
                "versioned and unversioned recovery artifacts disagree",
            ));
        }
        let newest_revision = versioned
            .iter()
            .map(|(_, revision, _, _)| *revision)
            .max()
            .expect("versioned artifacts cannot be empty");
        let mut newest = versioned
            .into_iter()
            .filter(|(_, revision, _, _)| *revision == newest_revision);
        let (_, revision, entries, checksum) =
            newest.next().expect("newest versioned artifact must exist");
        if newest.any(|(_, _, _, other_checksum)| other_checksum != checksum) {
            return Err(repair_required(
                path,
                "multiple recovery artifacts have the same revision but different contents",
            ));
        }
        return Ok(LoadedJournal {
            revision,
            entries,
            needs_rewrite: true,
        });
    }

    if unversioned.len() == 1 {
        let (_, entries) = unversioned
            .pop()
            .expect("single unversioned artifact must exist");
        return Ok(LoadedJournal {
            revision: 0,
            entries,
            needs_rewrite: true,
        });
    }

    Err(repair_required(
        path,
        "multiple unversioned recovery artifacts are ambiguous",
    ))
}

fn read_journal(path: &Path) -> Result<DecodedJournal, AppError> {
    decode_journal(&std::fs::read(path)?)
}

fn replacement_artifacts(path: &Path) -> Result<Vec<(PathBuf, JournalArtifactKind)>, AppError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        return Ok(Vec::new());
    }
    let name = path
        .file_name()
        .ok_or_else(|| {
            AppError::Validation(format!("Invalid mutation journal path: {}", path.display()))
        })?
        .to_string_lossy();
    let recovery_prefix = format!("{name}.recover.");
    let temp_prefix = format!("{name}.tmp.");

    let mut artifacts = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.starts_with(&recovery_prefix) {
            artifacts.push((entry.path(), JournalArtifactKind::Recovery));
        } else if file_name.starts_with(&temp_prefix) {
            artifacts.push((entry.path(), JournalArtifactKind::Temp));
        }
    }
    Ok(artifacts)
}

fn validate_noncanonical_artifacts(artifacts: &[(PathBuf, JournalArtifactKind)]) {
    for (artifact_path, kind) in artifacts {
        match read_journal(artifact_path) {
            Ok(DecodedJournal::Versioned { revision, .. }) => log::warn!(
                "Retaining stale mutation journal {} artifact at revision {}: {}",
                kind.label(),
                revision,
                artifact_path.display()
            ),
            Ok(DecodedJournal::Unversioned(_)) => log::warn!(
                "Retaining stale unversioned mutation journal {} artifact: {}",
                kind.label(),
                artifact_path.display()
            ),
            Err(error) => log::warn!(
                "Retaining invalid mutation journal {} artifact {}: {error}",
                kind.label(),
                artifact_path.display()
            ),
        }
    }
}

fn repair_required(path: &Path, detail: &str) -> AppError {
    AppError::Validation(format!(
        "Mutation journal recovery requires repair for {}: {detail}",
        path.display()
    ))
}

fn validate_plan(plan: &OperationPlan) -> Result<(), AppError> {
    if plan.kind.trim().is_empty() || plan.game_id.trim().is_empty() {
        return Err(AppError::Validation(
            "Mutation operation kind and game id are required".to_string(),
        ));
    }
    if plan.steps.is_empty() {
        return Err(AppError::Validation(
            "Mutation operation must plan at least one filesystem step".to_string(),
        ));
    }

    let mut sequences = plan
        .steps
        .iter()
        .map(|step| step.sequence)
        .collect::<Vec<_>>();
    sequences.sort_unstable();
    if sequences
        .iter()
        .enumerate()
        .any(|(expected, actual)| *actual as usize != expected)
    {
        return Err(AppError::Validation(
            "Mutation operation step sequences must be contiguous and start at zero".to_string(),
        ));
    }
    if plan.steps.iter().any(|step| {
        step.old_path.is_none()
            || step.new_path.is_none()
            || (step.kind == MutationStepKind::HardlinkReplace
                && (step.stage_path.is_none() || step.expected_identity.is_none()))
    }) {
        return Err(AppError::Validation(
            "Every mutation step requires the paths and identity required by its kind".to_string(),
        ));
    }
    Ok(())
}

fn require_status(operation: &Operation, allowed: &[OperationStatus]) -> Result<(), AppError> {
    if allowed.contains(&operation.status) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Mutation operation {} is in invalid status {:?}",
            operation.id, operation.status
        )))
    }
}

fn refresh_application_status(operation: &mut Operation) {
    if operation
        .steps
        .iter()
        .any(|step| step.status == StepStatus::RolledBack)
    {
        operation.status = OperationStatus::RollingBack;
    } else if operation
        .steps
        .iter()
        .all(|step| step.status != StepStatus::Planned)
    {
        operation.status = OperationStatus::Applied;
    }
}

#[derive(Deserialize)]
struct LegacyJournalEntry {
    id: String,
    plan: String,
    status: LegacyOperationStatus,
}

#[derive(Deserialize)]
enum LegacyOperationStatus {
    Pending,
    Completed,
    Failed,
}

fn decode_journal(bytes: &[u8]) -> Result<DecodedJournal, AppError> {
    if let Ok(snapshot) = serde_json::from_slice::<JournalSnapshot>(bytes) {
        return validate_snapshot(snapshot);
    }
    if let Ok(entries) = serde_json::from_slice::<Vec<Operation>>(bytes) {
        return Ok(DecodedJournal::Unversioned(entries));
    }

    let legacy = serde_json::from_slice::<Vec<LegacyJournalEntry>>(bytes)?;
    Ok(DecodedJournal::Unversioned(
        legacy
            .into_iter()
            .map(|entry| {
                let completed = matches!(entry.status, LegacyOperationStatus::Completed);
                Operation {
                    id: entry.id,
                    kind: format!("legacy: {}", entry.plan),
                    game_id: "legacy-unknown".to_string(),
                    created_at: "1970-01-01T00:00:00Z".to_string(),
                    status: if completed {
                        OperationStatus::Completed
                    } else {
                        OperationStatus::FailedNeedsRepair
                    },
                    steps: Vec::new(),
                    database_projection_status: if completed {
                        DatabaseProjectionStatus::Committed
                    } else {
                        DatabaseProjectionStatus::Failed
                    },
                    last_error: (!completed).then(|| {
                        "Legacy journal entry has no durable filesystem step plan".to_string()
                    }),
                }
            })
            .collect(),
    ))
}

fn validate_snapshot(snapshot: JournalSnapshot) -> Result<DecodedJournal, AppError> {
    if snapshot.format_version != JOURNAL_FORMAT_VERSION {
        return Err(AppError::Validation(format!(
            "Unsupported mutation journal format version {}",
            snapshot.format_version
        )));
    }
    if snapshot.revision == 0 {
        return Err(AppError::Validation(
            "Mutation journal snapshot revision must be greater than zero".to_string(),
        ));
    }
    let expected_ids = snapshot
        .entries
        .iter()
        .map(|operation| operation.id.clone())
        .collect::<Vec<_>>();
    if snapshot.operation_ids != expected_ids
        || expected_ids.iter().any(|id| id.trim().is_empty())
        || expected_ids
            .iter()
            .enumerate()
            .any(|(index, id)| expected_ids[..index].contains(id))
    {
        return Err(AppError::Validation(
            "Mutation journal snapshot operation identifiers are invalid".to_string(),
        ));
    }
    let expected_checksum = snapshot_checksum(
        snapshot.format_version,
        snapshot.revision,
        &snapshot.operation_ids,
        &snapshot.entries,
    )?;
    if snapshot.checksum != expected_checksum {
        return Err(AppError::Validation(
            "Mutation journal snapshot checksum does not match its contents".to_string(),
        ));
    }

    Ok(DecodedJournal::Versioned {
        revision: snapshot.revision,
        entries: snapshot.entries,
        checksum: snapshot.checksum,
    })
}

#[cfg(test)]
fn benchmark_history_operation(index: u32) -> Operation {
    Operation {
        id: format!("benchmark-history-{index}"),
        kind: "benchmark-history".to_string(),
        game_id: "benchmark-game".to_string(),
        created_at: "1970-01-01T00:00:00Z".to_string(),
        status: OperationStatus::Completed,
        steps: vec![OperationStep {
            sequence: 0,
            kind: MutationStepKind::Rename,
            old_path: Some(PathBuf::from(format!("C:/Benchmark/Old/{index}"))),
            new_path: Some(PathBuf::from(format!("C:/Benchmark/New/{index}"))),
            stage_path: None,
            expected_identity: None,
            status: StepStatus::Applied,
        }],
        database_projection_status: DatabaseProjectionStatus::Committed,
        last_error: None,
    }
}
