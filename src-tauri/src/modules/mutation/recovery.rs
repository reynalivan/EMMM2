use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::modules::mutation::journal::{
    DatabaseProjectionStatus, MutationStepKind, Operation, OperationJournal, OperationStatus,
    StepStatus, COLLECTION_APPLY_OPERATION_KIND, FOLDER_CREATE_OPERATION_KIND,
    FOLDER_CREATE_STAGE_PREFIX, LEGACY_COLLECTION_APPLY_OPERATION_KIND,
};
use crate::platform::fs::rename::rename_no_replace;
use crate::shared::errors::AppError;

pub struct RecoveryRoots {
    game_roots: HashMap<String, PathBuf>,
    staging_root: PathBuf,
    ready_to_move_roots: HashMap<String, PathBuf>,
}

impl RecoveryRoots {
    pub fn new(game_roots: HashMap<String, PathBuf>, staging_root: PathBuf) -> Self {
        Self {
            game_roots,
            staging_root,
            ready_to_move_roots: HashMap::new(),
        }
    }

    pub fn with_ready_to_move_roots(mut self, roots: HashMap<String, PathBuf>) -> Self {
        self.ready_to_move_roots = roots;
        self
    }
}

pub struct RecoveryRunner {
    journal: Arc<OperationJournal>,
    roots: RecoveryRoots,
    projection_pool: Option<sqlx::SqlitePool>,
}

struct ExternalImportSourcePins {
    directories: Vec<crate::platform::fs::rename::PinnedDirectory>,
}

impl ExternalImportSourcePins {
    fn destination_parent(&self) -> &crate::platform::fs::rename::PinnedDirectory {
        self.directories
            .last()
            .expect("external source ancestry always includes its root")
    }
}

impl RecoveryRunner {
    pub fn new(journal: Arc<OperationJournal>, roots: RecoveryRoots) -> Self {
        Self {
            journal,
            roots,
            projection_pool: None,
        }
    }

    pub fn with_projection_pool(mut self, pool: sqlx::SqlitePool) -> Self {
        self.projection_pool = Some(pool);
        self
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
            self.recover_operation(&operation).await?;
            recovered.push(operation.id);
        }
        self.cleanup_terminal_receipts().await;
        Ok(recovered)
    }

    async fn cleanup_terminal_receipts(&self) {
        let Some(pool) = &self.projection_pool else {
            return;
        };
        for operation in self.journal.entries().into_iter().filter(|operation| {
            operation.kind == COLLECTION_APPLY_OPERATION_KIND
                && operation.status == OperationStatus::Completed
        }) {
            match crate::modules::mutation::adapters::sqlite::projection_receipts::get(
                pool,
                &operation.id,
            )
            .await
            {
                Ok(Some(receipt))
                    if receipt.operation_id == operation.id
                        && receipt.game_id == operation.game_id
                        && operation.source_epoch.as_deref()
                            == Some(receipt.source_epoch.as_str()) =>
                {
                    if let Err(error) = crate::modules::mutation::adapters::sqlite::projection_receipts::delete(pool, &operation.id).await {
                        log::warn!("Collection projection receipt cleanup failed: {error}");
                    }
                }
                Ok(None) => {}
                Ok(Some(_)) => log::warn!(
                    "Collection projection receipt for {} has an unknown binding; retaining evidence",
                    operation.id
                ),
                Err(error) => log::warn!(
                    "Collection projection receipt lookup for {} failed: {error}",
                    operation.id
                ),
            }
        }
    }

    async fn recover_operation(&self, operation: &Operation) -> Result<(), AppError> {
        if operation.database_projection_status == DatabaseProjectionStatus::NeedsRepair {
            return Ok(());
        }
        if matches!(
            operation.kind.as_str(),
            COLLECTION_APPLY_OPERATION_KIND | LEGACY_COLLECTION_APPLY_OPERATION_KIND
        ) && operation.source_epoch.is_none()
        {
            return self.mark_repair(
                operation,
                "Collection recovery requires its original Mods root identity; preserve legacy evidence for manual recovery".to_string(),
            );
        }
        let folder_create = operation.kind == FOLDER_CREATE_OPERATION_KIND;
        if folder_create && (operation.source_epoch.is_none() || operation.steps.len() != 1) {
            return self.mark_repair(
                operation,
                "Folder creation recovery requires its original root identity and one owned step"
                    .to_string(),
            );
        }
        if let Some(expected_epoch) = &operation.source_epoch {
            let actual = self
                .roots
                .game_roots
                .get(&operation.game_id)
                .and_then(|root| crate::platform::fs::file_utils::filesystem_identity(root));
            if actual.as_ref() != Some(expected_epoch) {
                return self.mark_repair(
                    operation,
                    "Mods root identity changed; preserving the journal for manual recovery"
                        .to_string(),
                );
            }
        }
        let receipt = if matches!(
            operation.kind.as_str(),
            COLLECTION_APPLY_OPERATION_KIND | LEGACY_COLLECTION_APPLY_OPERATION_KIND
        ) {
            match &self.projection_pool {
                Some(pool) => {
                    crate::modules::mutation::adapters::sqlite::projection_receipts::get(
                        pool,
                        &operation.id,
                    )
                    .await?
                }
                None if operation.database_projection_status
                    == DatabaseProjectionStatus::Committed =>
                {
                    None
                }
                None => return self.mark_repair(
                    operation,
                    "Collection recovery has no projection database; preserving journal evidence"
                        .to_string(),
                ),
            }
        } else {
            None
        };
        let receipt_committed = match receipt.as_ref() {
            Some(receipt)
                if operation.kind == COLLECTION_APPLY_OPERATION_KIND
                    && receipt.operation_id == operation.id
                    && receipt.game_id == operation.game_id
                    && operation.source_epoch.as_deref() == Some(receipt.source_epoch.as_str()) =>
            {
                true
            }
            Some(_) => {
                return self.mark_repair(
                    operation,
                    "Collection projection receipt does not match its journal binding".to_string(),
                )
            }
            None if operation.kind == LEGACY_COLLECTION_APPLY_OPERATION_KIND
                && operation.database_projection_status != DatabaseProjectionStatus::Committed =>
            {
                return self.mark_repair(
                    operation,
                    "Legacy collection journal has no transaction-bound completion proof"
                        .to_string(),
                )
            }
            None => false,
        };
        if receipt_committed && operation.status == OperationStatus::RollingBack {
            return self.mark_repair(
                operation,
                "Collection receipt conflicts with a rolling-back journal".to_string(),
            );
        }
        if operation.disk_revision.is_some() {
            return match operation.status {
                OperationStatus::DiskCommitted => Ok(()),
                OperationStatus::DbCommitted => {
                    self.journal.finalize_recovered_commit(&operation.id)
                }
                _ => self.mark_repair(
                    operation,
                    "Disk-committed operation has an invalid journal status".to_string(),
                ),
            };
        }

        let Some(game_root) = self.roots.game_roots.get(&operation.game_id) else {
            return self.mark_repair(
                operation,
                format!(
                    "Configured root for game {} is unavailable",
                    operation.game_id
                ),
            );
        };
        let _creation_root = if folder_create {
            let root = match crate::platform::fs::rename::PinnedDirectory::open(game_root) {
                Ok(root) => root,
                Err(error) => return self.mark_repair(operation, error.to_string()),
            };
            if Some(root.identity()) != operation.source_epoch.as_deref() {
                return self.mark_repair(
                    operation,
                    "Folder creation root was replaced before recovery".to_string(),
                );
            }
            Some(root)
        } else {
            None
        };

        let source_pins = match operation
            .steps
            .iter()
            .map(|step| self.pin_external_import_source(operation, step, game_root))
            .collect::<Result<Vec<_>, AppError>>()
        {
            Ok(pins) => pins,
            Err(error) => return self.mark_repair(operation, error.to_string()),
        };

        let states = match operation
            .steps
            .iter()
            .zip(&source_pins)
            .map(|(step, external_source)| {
                if step.status == StepStatus::Skipped {
                    return Ok(DiskStepState::Skipped);
                }
                let old_path = step.old_path.as_deref().ok_or_else(|| {
                    AppError::Validation("Mutation recovery requires old_path".to_string())
                })?;
                let new_path = step.new_path.as_deref().ok_or_else(|| {
                    AppError::Validation("Mutation recovery requires new_path".to_string())
                })?;
                if external_source.is_none() {
                    validate_recovery_path(old_path, game_root)?;
                }
                validate_recovery_path(new_path, game_root)?;
                if folder_create {
                    validate_folder_create_step(step)?;
                    if path_is_absent(old_path)? && path_is_absent(new_path)? {
                        return Ok(DiskStepState::AtSource);
                    }
                }
                match step.kind {
                    MutationStepKind::Rename | MutationStepKind::Quarantine => {
                        if let Some(stage_path) = step.stage_path.as_deref() {
                            if step.kind == MutationStepKind::Rename {
                                if external_source.is_some() {
                                    validate_external_source_path(
                                        stage_path,
                                        external_source
                                            .as_ref()
                                            .expect("checked external source")
                                            .directories[0]
                                            .path(),
                                    )?;
                                } else {
                                    validate_recovery_path(stage_path, game_root)?;
                                }
                                if stage_path.parent() != old_path.parent() {
                                    return Err(AppError::Security(format!(
                                        "Staged rename is not beside its source: {}",
                                        stage_path.display()
                                    )));
                                }
                            } else {
                                validate_recovery_path(stage_path, &self.roots.staging_root)?;
                            }
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

        if receipt_committed
            || operation.database_projection_status == DatabaseProjectionStatus::Committed
        {
            let disk_matches_projection =
                operation.steps.iter().zip(&states).all(|(step, state)| {
                    if receipt_committed && step.status == StepStatus::RolledBack {
                        return false;
                    }
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
                if receipt_committed
                    && operation.database_projection_status != DatabaseProjectionStatus::Committed
                {
                    self.journal.mark_db_committed(&operation.id)?;
                }
                self.journal.finalize_recovered_commit(&operation.id)?;
                return Ok(());
            }
            return self.mark_repair(
                operation,
                "Database projection is committed but filesystem is not fully at target"
                    .to_string(),
            );
        }

        self.journal.begin_rollback(&operation.id)?;
        for ((step, state), external_source) in
            operation.steps.iter().zip(states).zip(&source_pins).rev()
        {
            let rollback_result = if folder_create && state != DiskStepState::Skipped {
                rollback_folder_create(step, state)
            } else {
                match (step.kind, state) {
                    (_, DiskStepState::Skipped) => continue,
                    (MutationStepKind::HardlinkReplace, DiskStepState::AtSource) => Ok(()),
                    (MutationStepKind::HardlinkReplace, DiskStepState::AtStage) => std::fs::rename(
                        step.stage_path.as_deref().expect("validated backup path"),
                        step.new_path.as_deref().expect("validated target path"),
                    )
                    .map_err(AppError::from),
                    (MutationStepKind::HardlinkReplace, DiskStepState::AtTarget) => {
                        rollback_hardlink(
                            step.new_path.as_deref().expect("validated target path"),
                            step.stage_path.as_deref().expect("validated backup path"),
                        )
                    }
                    (_, DiskStepState::AtSource) => Ok(()),
                    (_, DiskStepState::AtTarget) => match external_source {
                        Some(pins) => rollback_rename_into_pinned_parent(
                            step.new_path.as_deref().expect("validated new_path"),
                            step.old_path.as_deref().expect("validated old_path"),
                            step.expected_identity.as_deref(),
                            pins.destination_parent(),
                        ),
                        None => rollback_rename(
                            step.new_path.as_deref().expect("validated new_path"),
                            step.old_path.as_deref().expect("validated old_path"),
                            step.expected_identity.as_deref(),
                        ),
                    },
                    (_, DiskStepState::AtStage) => match external_source {
                        Some(pins) => rollback_rename_into_pinned_parent(
                            step.stage_path.as_deref().expect("classified stage_path"),
                            step.old_path.as_deref().expect("validated old_path"),
                            step.expected_identity.as_deref(),
                            pins.destination_parent(),
                        ),
                        None => rollback_rename(
                            step.stage_path.as_deref().expect("classified stage_path"),
                            step.old_path.as_deref().expect("validated old_path"),
                            step.expected_identity.as_deref(),
                        ),
                    },
                    (_, DiskStepState::Ambiguous) => {
                        unreachable!("ambiguous state handled before rollback")
                    }
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
        if operation.disk_revision.is_some() {
            self.journal
                .isolate_disk_commit_for_repair(&operation.id, error)
        } else {
            self.journal.fail(&operation.id, error)
        }
    }

    fn pin_external_import_source(
        &self,
        operation: &Operation,
        step: &crate::modules::mutation::journal::OperationStep,
        game_root: &Path,
    ) -> Result<Option<ExternalImportSourcePins>, AppError> {
        if operation.kind != "import-commit" || step.status == StepStatus::Skipped {
            return Ok(None);
        }
        let old_path = step.old_path.as_deref().ok_or_else(|| {
            AppError::Validation("Mutation recovery requires old_path".to_string())
        })?;
        let old_path = crate::shared::path_key::physical_namespace_path(old_path)?;
        let game_root =
            crate::shared::path_key::physical_namespace_path(&game_root.canonicalize()?)?;
        if old_path != game_root && old_path.starts_with(&game_root) {
            return Ok(None);
        }
        let expected_root_identity = step.source_root_identity.as_deref().ok_or_else(|| {
            AppError::Security(
                "External import recovery requires its original trusted source-root identity"
                    .to_string(),
            )
        })?;
        let roots = std::iter::once(&self.roots.staging_root)
            .chain(self.roots.ready_to_move_roots.get(&operation.game_id));
        let mut root_identity_changed = false;
        for configured_root in roots {
            let Ok(canonical_root) = configured_root.canonicalize() else {
                continue;
            };
            let root = crate::shared::path_key::physical_namespace_path(&canonical_root)?;
            if old_path == root || !old_path.starts_with(&root) {
                continue;
            }
            let root_pin = crate::platform::fs::rename::PinnedDirectory::open(&root)?;
            if root_pin.identity() != expected_root_identity {
                root_identity_changed = true;
                continue;
            }
            let parent = old_path.parent().ok_or_else(|| {
                AppError::Validation("Import recovery source has no parent".to_string())
            })?;
            let relative = parent.strip_prefix(&root).map_err(|_| {
                AppError::Security("Import recovery source escaped its trusted root".to_string())
            })?;
            let mut directories = vec![root_pin];
            let mut current = root;
            for component in relative.components() {
                let std::path::Component::Normal(name) = component else {
                    return Err(AppError::Security(
                        "Import recovery source contains a non-normal component".to_string(),
                    ));
                };
                current.push(name);
                let canonical =
                    crate::shared::path_key::physical_namespace_path(&current.canonicalize()?)?;
                if canonical != current {
                    return Err(AppError::Security(format!(
                        "Import recovery source ancestor is rebound: {}",
                        current.display()
                    )));
                }
                let pin = crate::platform::fs::rename::PinnedDirectory::open(&current)?;
                let canonical_after_open =
                    crate::shared::path_key::physical_namespace_path(&current.canonicalize()?)?;
                if canonical_after_open != current
                    || crate::platform::fs::file_utils::filesystem_identity(&current).as_deref()
                        != Some(pin.identity())
                {
                    return Err(AppError::Security(format!(
                        "Import recovery source ancestor changed while being pinned: {}",
                        current.display()
                    )));
                }
                directories.push(pin);
            }
            return Ok(Some(ExternalImportSourcePins { directories }));
        }
        Err(AppError::Security(if root_identity_changed {
            "Import source root was replaced before recovery".to_string()
        } else {
            "Import recovery source is outside configured staging and ReadyToMove roots".to_string()
        }))
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

fn path_is_absent(path: &Path) -> Result<bool, AppError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}

fn validate_folder_create_step(
    step: &crate::modules::mutation::journal::OperationStep,
) -> Result<(), AppError> {
    let old = step.old_path.as_deref().expect("validated old_path");
    let new = step.new_path.as_deref().expect("validated new_path");
    let stage_id = old
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(FOLDER_CREATE_STAGE_PREFIX));
    if step.kind != MutationStepKind::Rename
        || step.stage_path.is_some()
        || step
            .expected_identity
            .as_deref()
            .is_none_or(|value| value.is_empty())
        || old.parent() != new.parent()
        || stage_id.is_none_or(|id| uuid::Uuid::parse_str(id).is_err())
    {
        return Err(AppError::Security(
            "Folder creation journal has no valid owned sibling staging path; retained for review"
                .to_string(),
        ));
    }
    Ok(())
}

fn rollback_folder_create(
    step: &crate::modules::mutation::journal::OperationStep,
    state: DiskStepState,
) -> Result<(), AppError> {
    let stage = step.old_path.as_deref().expect("validated staging path");
    let expected = step
        .expected_identity
        .as_deref()
        .expect("validated creation identity");
    let path = match state {
        DiskStepState::AtSource if path_is_absent(stage)? => return Ok(()),
        DiskStepState::AtSource => stage,
        DiskStepState::AtTarget => step.new_path.as_deref().expect("validated target path"),
        _ => {
            return Err(AppError::Validation(
                "Invalid folder creation recovery state".to_string(),
            ))
        }
    };
    let parent = crate::platform::fs::rename::PinnedDirectory::open(
        path.parent().expect("validated creation parent"),
    )?;
    // New folder-create steps bind this field to the original staging parent.
    // Older journals did not persist it and retain their child/root checks.
    if step
        .source_root_identity
        .as_deref()
        .is_some_and(|expected| expected != parent.identity())
    {
        return Err(AppError::Security(
            "Folder creation recovery parent was replaced".to_string(),
        ));
    }
    let owned = crate::platform::fs::rename::PinnedDirectory::open_for_mutation(path)?;
    if owned.identity() != expected {
        return Err(AppError::Security(
            "Folder creation recovery directory was replaced".to_string(),
        ));
    }
    owned.remove_empty(&parent).map_err(AppError::from)
}

fn classify_rename(
    old_path: &Path,
    new_path: &Path,
    stage_path: Option<&Path>,
    expected_identity: Option<&str>,
) -> Result<DiskStepState, AppError> {
    if stage_path.is_some() && expected_identity.is_none() {
        return Ok(DiskStepState::Ambiguous);
    }
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
    let stage_matches = stage_exists
        && expected_identity.is_some_and(|expected| {
            stage_path.and_then(filesystem_identity).as_deref() == Some(expected)
        });
    Ok(match (old_exists, new_exists, stage_exists) {
        (true, false, false) if old_matches => DiskStepState::AtSource,
        (false, true, false) if new_matches => DiskStepState::AtTarget,
        (false, false, true) if stage_matches => DiskStepState::AtStage,
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

fn rollback_rename(
    source: &Path,
    destination: &Path,
    expected_identity: Option<&str>,
) -> Result<(), AppError> {
    if let Some(expected_identity) = expected_identity {
        if filesystem_identity(source).as_deref() != Some(expected_identity) {
            return Err(AppError::Io(format!(
                "Recovery source changed before rollback: {}",
                source.display()
            )));
        }
    }
    match std::fs::symlink_metadata(destination) {
        Ok(_) => {
            return Err(AppError::Io(format!(
                "Recovery destination is occupied: {}",
                destination.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(AppError::Io(format!(
                "Could not inspect recovery destination {}: {error}",
                destination.display()
            )))
        }
    }
    rename_no_replace(source, destination).map_err(AppError::from)
}

fn rollback_rename_into_pinned_parent(
    source: &Path,
    destination: &Path,
    expected_identity: Option<&str>,
    destination_parent: &crate::platform::fs::rename::PinnedDirectory,
) -> Result<(), AppError> {
    let name = destination
        .file_name()
        .ok_or_else(|| AppError::Validation("Recovery destination has no file name".to_string()))?;
    let requested_parent = destination
        .parent()
        .ok_or_else(|| AppError::Validation("Recovery destination has no parent".to_string()))?;
    let requested_parent = crate::shared::path_key::physical_namespace_path(requested_parent)?;
    let pinned_parent =
        crate::shared::path_key::physical_namespace_path(destination_parent.path())?;
    if requested_parent != pinned_parent {
        return Err(AppError::Security(
            "Recovery destination parent changed after validation".to_string(),
        ));
    }
    let owned = crate::platform::fs::rename::PinnedDirectory::open_for_rename(source)?;
    if expected_identity.is_some_and(|expected| owned.identity() != expected) {
        return Err(AppError::Io(format!(
            "Recovery source changed before rollback: {}",
            source.display()
        )));
    }
    let stable_destination = destination_parent.stable_path().join(name);
    match std::fs::symlink_metadata(&stable_destination) {
        Ok(_) => {
            return Err(AppError::Io(format!(
                "Recovery destination is occupied: {}",
                destination.display()
            )))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    owned
        .rename_into(destination_parent, name)
        .map_err(AppError::from)
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

fn validate_external_source_path(path: &Path, allowed_root: &Path) -> Result<(), AppError> {
    let path = crate::shared::path_key::physical_namespace_path(path)?;
    let allowed_root = crate::shared::path_key::physical_namespace_path(allowed_root)?;
    if path == allowed_root || !path.starts_with(&allowed_root) {
        return Err(AppError::Security(format!(
            "Recovery path escapes its configured source root: {}",
            path.display()
        )));
    }
    validate_recovery_path(&path, &allowed_root)
}

fn nearest_existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors().find(|candidate| candidate.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::mutation::journal::{OperationPlan, PlannedStep};

    #[tokio::test]
    async fn replaced_root_never_rolls_back_an_acknowledged_disk_commit() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Mods");
        let old = root.join("A");
        let disabled = root.join("DISABLED A");
        std::fs::create_dir_all(&old).unwrap();
        let epoch = crate::platform::fs::file_utils::filesystem_identity(&root).unwrap();
        let identity = crate::platform::fs::file_utils::filesystem_identity(&old).unwrap();
        let journal =
            Arc::new(OperationJournal::open(temp.path().join("journal.json"), 16).unwrap());
        let operation_id = journal
            .plan_operation(
                OperationPlan::new(
                    "workspace-switch",
                    "game",
                    vec![PlannedStep::rename(0, old.clone(), disabled.clone())
                        .with_expected_identity(Some(identity))],
                )
                .with_source_epoch(epoch),
            )
            .unwrap();
        journal.mark_applying(&operation_id).unwrap();
        std::fs::rename(&old, &disabled).unwrap();
        journal
            .settle_steps(
                &operation_id,
                &[(
                    0,
                    crate::modules::mutation::journal::StepSettlement::Applied,
                )],
            )
            .unwrap();
        journal.mark_disk_committed(&operation_id).unwrap();
        let preserved = temp.path().join("Preserved");
        std::fs::rename(&root, &preserved).unwrap();
        std::fs::create_dir_all(&disabled).unwrap();
        let replacement = crate::platform::fs::file_utils::filesystem_identity(&disabled);
        let runner = RecoveryRunner::new(
            journal.clone(),
            RecoveryRoots::new(
                HashMap::from([("game".into(), root)]),
                temp.path().join("staging"),
            ),
        );
        runner.run_recovery().await.unwrap();
        assert_eq!(
            crate::platform::fs::file_utils::filesystem_identity(&disabled),
            replacement
        );
        assert!(preserved.join("DISABLED A").exists());
        assert!(!old.exists());
        assert_eq!(journal.entries()[0].status, OperationStatus::DiskCommitted);
        assert_eq!(
            journal.entries()[0].database_projection_status,
            DatabaseProjectionStatus::NeedsRepair
        );
        assert!(journal.pending_disk_commits().is_empty());
        assert!(!journal.has_pending_disk_commit_for_game("game"));
        drop(runner);
        drop(journal);
        let loaded = OperationJournal::open(temp.path().join("journal.json"), 16).unwrap();
        assert_eq!(loaded.entries()[0].status, OperationStatus::DiskCommitted);
        assert_eq!(
            loaded.entries()[0].database_projection_status,
            DatabaseProjectionStatus::NeedsRepair
        );
    }

    fn conflict_recovery(
        root: &Path,
        journal: Arc<OperationJournal>,
    ) -> (RecoveryRunner, PathBuf, PathBuf, PathBuf) {
        let game_root = root.join("Mods");
        let staging_root = root.join("staging");
        let old_path = game_root.join("Alice").join("Blue");
        let stage_path = game_root.join("Alice").join(".emmm-conflict-stage-0-test");
        let target_path = game_root.join("Alice").join("Blue One");
        std::fs::create_dir_all(&old_path).unwrap();
        std::fs::create_dir_all(&staging_root).unwrap();
        let expected_identity = filesystem_identity(&old_path).unwrap();
        journal
            .plan_operation(OperationPlan::new(
                "folder-conflict-rename",
                "game",
                vec![
                    PlannedStep::rename(0, old_path.clone(), target_path.clone())
                        .with_stage_path(stage_path.clone())
                        .with_expected_identity(Some(expected_identity)),
                ],
            ))
            .unwrap();
        let runner = RecoveryRunner::new(
            journal,
            RecoveryRoots::new(HashMap::from([("game".into(), game_root)]), staging_root),
        );
        (runner, old_path, stage_path, target_path)
    }

    #[tokio::test]
    async fn conflict_stage_in_game_root_restores_expected_folder() {
        let temp = tempfile::tempdir().unwrap();
        let journal =
            Arc::new(OperationJournal::open(temp.path().join("journal.json"), 16).unwrap());
        let (runner, old_path, stage_path, target_path) =
            conflict_recovery(temp.path(), journal.clone());
        let expected_identity = filesystem_identity(&old_path).unwrap();
        std::fs::rename(&old_path, &stage_path).unwrap();

        runner.run_recovery().await.unwrap();

        assert_eq!(filesystem_identity(&old_path), Some(expected_identity));
        assert!(!stage_path.exists());
        assert!(!target_path.exists());
        assert_eq!(journal.entries()[0].status, OperationStatus::RolledBack);
    }

    #[tokio::test]
    async fn replacement_conflict_stage_is_left_for_manual_repair() {
        let temp = tempfile::tempdir().unwrap();
        let journal =
            Arc::new(OperationJournal::open(temp.path().join("journal.json"), 16).unwrap());
        let (runner, old_path, stage_path, _) = conflict_recovery(temp.path(), journal.clone());
        let parked = temp.path().join("parked");
        std::fs::rename(&old_path, &stage_path).unwrap();
        std::fs::rename(&stage_path, &parked).unwrap();
        std::fs::create_dir(&stage_path).unwrap();
        let replacement_identity = filesystem_identity(&stage_path).unwrap();

        runner.run_recovery().await.unwrap();

        assert!(!old_path.exists());
        assert_eq!(filesystem_identity(&stage_path), Some(replacement_identity));
        assert!(parked.exists());
        let operation = &journal.entries()[0];
        assert_eq!(operation.status, OperationStatus::FailedNeedsRepair);
        assert!(operation
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("ambiguous"));
    }
}
