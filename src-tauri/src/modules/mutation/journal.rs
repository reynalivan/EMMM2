use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::platform::fs::atomic_file::atomic_write;
use crate::shared::errors::AppError;

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

pub struct OperationJournal {
    path: Option<PathBuf>,
    max_history: usize,
    entries: Mutex<Vec<Operation>>,
}

impl OperationJournal {
    pub fn new() -> Self {
        Self {
            path: None,
            max_history: 1,
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn open(path: impl AsRef<Path>, max_history: usize) -> Result<Self, AppError> {
        if max_history == 0 {
            return Err(AppError::Validation(
                "Mutation journal history limit must be greater than zero".to_string(),
            ));
        }

        let path = path.as_ref().to_path_buf();
        let (mut entries, migrated) = if path.exists() {
            decode_journal(&std::fs::read(&path)?)?
        } else {
            (Vec::new(), false)
        };
        let initial_len = entries.len();
        Self::trim(&mut entries, max_history);
        if migrated || entries.len() != initial_len {
            atomic_write(&path, &serde_json::to_vec(&entries)?)?;
        }

        Ok(Self {
            path: Some(path),
            max_history,
            entries: Mutex::new(entries),
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

        let mut entries = self.lock_entries()?;
        let mut next = entries.clone();
        next.push(operation);
        Self::trim(&mut next, self.max_history);
        self.persist(&next)?;
        *entries = next;
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
        self.entries
            .lock()
            .expect("mutation journal lock poisoned")
            .clone()
    }

    fn mutate_operation<R>(
        &self,
        id: &str,
        mutate: impl FnOnce(&mut Operation) -> Result<R, AppError>,
    ) -> Result<R, AppError> {
        let mut entries = self.lock_entries()?;
        let mut next = entries.clone();
        let operation = next
            .iter_mut()
            .find(|operation| operation.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Mutation journal operation {id}")))?;
        let output = mutate(operation)?;
        self.persist(&next)?;
        *entries = next;
        Ok(output)
    }

    fn lock_entries(&self) -> Result<std::sync::MutexGuard<'_, Vec<Operation>>, AppError> {
        self.entries
            .lock()
            .map_err(|_| AppError::Internal("Mutation journal lock poisoned".to_string()))
    }

    fn persist(&self, entries: &[Operation]) -> Result<(), AppError> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        atomic_write(path, &serde_json::to_vec(entries)?)
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
}

impl Default for OperationJournal {
    fn default() -> Self {
        Self::new()
    }
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

fn decode_journal(bytes: &[u8]) -> Result<(Vec<Operation>, bool), AppError> {
    if let Ok(entries) = serde_json::from_slice::<Vec<Operation>>(bytes) {
        return Ok((entries, false));
    }

    let legacy = serde_json::from_slice::<Vec<LegacyJournalEntry>>(bytes)?;
    Ok((
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
        true,
    ))
}
