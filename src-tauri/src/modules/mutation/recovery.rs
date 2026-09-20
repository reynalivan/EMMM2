use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::modules::mutation::journal::{
    DatabaseProjectionStatus, MutationStepKind, Operation, OperationJournal, OperationStatus,
    StepStatus,
};
use crate::shared::errors::AppError;

pub struct RecoveryRoots {
    game_roots: HashMap<String, PathBuf>,
    staging_root: PathBuf,
}

impl RecoveryRoots {
    pub fn new(game_roots: HashMap<String, PathBuf>, staging_root: PathBuf) -> Self {
        Self {
            game_roots,
            staging_root,
        }
    }
}

pub struct RecoveryRunner {
    journal: Arc<OperationJournal>,
    roots: RecoveryRoots,
}

impl RecoveryRunner {
    pub fn new(journal: Arc<OperationJournal>, roots: RecoveryRoots) -> Self {
        Self { journal, roots }
    }

    pub async fn run_recovery(&self) -> Result<Vec<String>, AppError> {
        let pending = self
            .journal
            .entries()
            .into_iter()
            .filter(|operation| !operation.status.is_terminal())
            .collect::<Vec<_>>();
        let mut recovered = Vec::with_capacity(pending.len());
        for operation in pending {
            self.recover_operation(&operation)?;
            recovered.push(operation.id);
        }
        Ok(recovered)
    }

    fn recover_operation(&self, operation: &Operation) -> Result<(), AppError> {
        let Some(game_root) = self.roots.game_roots.get(&operation.game_id) else {
            return self.mark_repair(
                operation,
                format!(
                    "Configured root for game {} is unavailable",
                    operation.game_id
                ),
            );
        };

        let states = match operation
            .steps
            .iter()
            .map(|step| {
                if step.status == StepStatus::Skipped {
                    return Ok(DiskStepState::Skipped);
                }
                let old_path = step.old_path.as_deref().ok_or_else(|| {
                    AppError::Validation("Mutation recovery requires old_path".to_string())
                })?;
                let new_path = step.new_path.as_deref().ok_or_else(|| {
                    AppError::Validation("Mutation recovery requires new_path".to_string())
                })?;
                validate_recovery_path(old_path, game_root)?;
                validate_recovery_path(new_path, game_root)?;
                match step.kind {
                    MutationStepKind::Rename | MutationStepKind::Quarantine => {
                        if let Some(stage_path) = step.stage_path.as_deref() {
                            validate_recovery_path(stage_path, &self.roots.staging_root)?;
                        }
                        classify_rename(
                            old_path,
                            new_path,
                            step.stage_path.as_deref(),
                            step.expected_identity.as_deref(),
                        )
                    }
                    MutationStepKind::HardlinkReplace => {
                        let backup = step.stage_path.as_deref().ok_or_else(|| {
                            AppError::Validation(
                                "Hardlink recovery requires backup path".to_string(),
                            )
                        })?;
                        validate_recovery_path(backup, game_root)?;
                        classify_hardlink(
                            old_path,
                            new_path,
                            backup,
                            step.expected_identity.as_deref().ok_or_else(|| {
                                AppError::Validation(
                                    "Hardlink recovery requires keeper identity".to_string(),
                                )
                            })?,
                            operation.database_projection_status
                                == DatabaseProjectionStatus::Committed,
                        )
                    }
                }
            })
            .collect::<Result<Vec<_>, AppError>>()
        {
            Ok(states) => states,
            Err(error) => return self.mark_repair(operation, error.to_string()),
        };

        if states.contains(&DiskStepState::Ambiguous) {
            return self.mark_repair(
                operation,
                "Filesystem state is ambiguous; no recovery mutation was attempted".to_string(),
            );
        }

        if operation.database_projection_status == DatabaseProjectionStatus::Committed {
            let disk_matches_projection =
                operation.steps.iter().zip(&states).all(|(step, state)| {
                    matches!(
                        (step.status, state),
                        (StepStatus::Applied, DiskStepState::AtTarget)
                            | (StepStatus::RolledBack, DiskStepState::AtSource)
                            | (StepStatus::Skipped, DiskStepState::Skipped)
                    )
                });
            if disk_matches_projection {
                for step in operation
                    .steps
                    .iter()
                    .filter(|step| step.status == StepStatus::Applied)
                {
                    finalize_committed_step(step)?;
                }
                return self.journal.finalize_recovered_commit(&operation.id);
            }
            return self.mark_repair(
                operation,
                "Database projection is committed but filesystem is not fully at target"
                    .to_string(),
            );
        }

        self.journal.begin_rollback(&operation.id)?;
        for (step, state) in operation.steps.iter().zip(states).rev() {
            let rollback_result = match (step.kind, state) {
                (_, DiskStepState::Skipped) => continue,
                (MutationStepKind::HardlinkReplace, DiskStepState::AtSource) => Ok(()),
                (MutationStepKind::HardlinkReplace, DiskStepState::AtStage) => std::fs::rename(
                    step.stage_path.as_deref().expect("validated backup path"),
                    step.new_path.as_deref().expect("validated target path"),
                )
                .map_err(AppError::from),
                (MutationStepKind::HardlinkReplace, DiskStepState::AtTarget) => rollback_hardlink(
                    step.new_path.as_deref().expect("validated target path"),
                    step.stage_path.as_deref().expect("validated backup path"),
                ),
                (_, DiskStepState::AtSource) => Ok(()),
                (_, DiskStepState::AtTarget) => std::fs::rename(
                    step.new_path.as_deref().expect("validated new_path"),
                    step.old_path.as_deref().expect("validated old_path"),
                )
                .map_err(AppError::from),
                (_, DiskStepState::AtStage) => std::fs::rename(
                    step.stage_path.as_deref().expect("classified stage_path"),
                    step.old_path.as_deref().expect("validated old_path"),
                )
                .map_err(AppError::from),
                (_, DiskStepState::Ambiguous) => {
                    unreachable!("ambiguous state handled before rollback")
                }
            };
            if let Err(error) = rollback_result {
                return self.mark_repair(
                    operation,
                    format!(
                        "Rollback failed at step {} after earlier compensation: {error}",
                        step.sequence
                    ),
                );
            }
            self.journal
                .mark_step_rolled_back(&operation.id, step.sequence)?;
        }
        self.journal.finish_rollback(&operation.id)
    }

    fn mark_repair(&self, operation: &Operation, error: String) -> Result<(), AppError> {
        if operation.status == OperationStatus::FailedNeedsRepair {
            return Ok(());
        }
        self.journal.fail(&operation.id, error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiskStepState {
    AtSource,
    AtTarget,
    AtStage,
    Skipped,
    Ambiguous,
}

fn classify_rename(
    old_path: &Path,
    new_path: &Path,
    stage_path: Option<&Path>,
    expected_identity: Option<&str>,
) -> Result<DiskStepState, AppError> {
    let old_exists = old_path.try_exists()?;
    let new_exists = new_path.try_exists()?;
    let stage_exists = match stage_path {
        Some(path) => path.try_exists()?,
        None => false,
    };
    let old_matches = old_exists
        && expected_identity
            .is_none_or(|expected| filesystem_identity(old_path).as_deref() == Some(expected));
    let new_matches = new_exists
        && expected_identity
            .is_none_or(|expected| filesystem_identity(new_path).as_deref() == Some(expected));
    Ok(match (old_exists, new_exists, stage_exists) {
        (true, false, false) if old_matches => DiskStepState::AtSource,
        (false, true, false) if new_matches => DiskStepState::AtTarget,
        (false, false, true) => DiskStepState::AtStage,
        _ => DiskStepState::Ambiguous,
    })
}

fn classify_hardlink(
    keeper: &Path,
    target: &Path,
    backup: &Path,
    expected_identity: &str,
    projection_committed: bool,
) -> Result<DiskStepState, AppError> {
    if !keeper.try_exists()? {
        return Ok(DiskStepState::Ambiguous);
    }
    let keeper_matches = filesystem_identity(keeper).as_deref() == Some(expected_identity);
    if !keeper_matches {
        return Ok(DiskStepState::Ambiguous);
    }
    let target_exists = target.try_exists()?;
    let backup_exists = backup.try_exists()?;
    let target_is_keeper =
        target_exists && filesystem_identity(target).as_deref() == Some(expected_identity);
    Ok(match (target_exists, backup_exists, target_is_keeper) {
        (true, false, false) => DiskStepState::AtSource,
        (false, true, false) => DiskStepState::AtStage,
        (true, true, true) => DiskStepState::AtTarget,
        (true, false, true) if projection_committed => DiskStepState::AtTarget,
        _ => DiskStepState::Ambiguous,
    })
}

fn filesystem_identity(path: &Path) -> Option<String> {
    crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(
        path,
    )
}

fn rollback_hardlink(target: &Path, backup: &Path) -> Result<(), AppError> {
    std::fs::remove_file(target)?;
    std::fs::rename(backup, target)?;
    Ok(())
}

fn finalize_committed_step(
    step: &crate::modules::mutation::journal::OperationStep,
) -> Result<(), AppError> {
    match step.kind {
        MutationStepKind::HardlinkReplace => {
            if let Some(backup) = step.stage_path.as_deref().filter(|path| path.exists()) {
                std::fs::remove_file(backup)?;
            }
        }
        MutationStepKind::Quarantine => {
            if let Some(quarantine) = step.new_path.as_deref().filter(|path| path.exists()) {
                crate::platform::fs::recycle_bin::move_path_to_recycle_bin(quarantine)?;
            }
        }
        MutationStepKind::Rename => {}
    }
    Ok(())
}

fn validate_recovery_path(path: &Path, allowed_root: &Path) -> Result<(), AppError> {
    if !path.is_absolute() {
        return Err(AppError::Validation(format!(
            "Recovery path must be absolute: {}",
            path.display()
        )));
    }
    let canonical_root = std::fs::canonicalize(allowed_root).map_err(|error| {
        AppError::Validation(format!(
            "Recovery root {} is unavailable: {error}",
            allowed_root.display()
        ))
    })?;
    let existing = nearest_existing_ancestor(path).ok_or_else(|| {
        AppError::Validation(format!(
            "Recovery path has no existing ancestor: {}",
            path.display()
        ))
    })?;
    let canonical_existing = std::fs::canonicalize(existing)?;
    if !canonical_existing.starts_with(&canonical_root)
        || (path.try_exists()? && canonical_existing == canonical_root)
    {
        return Err(AppError::Security(format!(
            "Recovery path escapes its configured root: {}",
            path.display()
        )));
    }
    Ok(())
}

fn nearest_existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors().find(|candidate| candidate.exists())
}
