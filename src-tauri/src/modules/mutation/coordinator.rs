use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use crate::modules::mutation::journal::{OperationJournal, OperationPlan};
use crate::modules::mutation::task_registry::TaskRegistry;
use crate::platform::fs::operation_lock::{OpGuard, OperationLock};
use crate::shared::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationExemption {
    CatalogProjection,
    CollectionMetadata,
    DuplicateIgnoreRules,
    LibraryMetadata,
    PreviewFile,
    Reconciliation,
    Thumbnail,
    WorkspaceConfiguration,
}

pub struct MutationExemptionGuard {
    _guard: OpGuard,
    _reason: MutationExemption,
}

impl MutationExemptionGuard {
    pub fn op_guard(&self) -> &OpGuard {
        &self._guard
    }
}

pub struct MutationCoordinator {
    lock: OperationLock,
    journal: OnceLock<Arc<OperationJournal>>,
    task_registry: Arc<TaskRegistry>,
    latest_intents: Arc<Mutex<IntentRegistry>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntentTarget {
    ModPath(String),
    ObjectId(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct IntentKey {
    game_id: String,
    source_epoch: Option<String>,
    target: IntentTarget,
}

#[derive(Default)]
struct IntentRegistry {
    revisions: HashMap<IntentKey, u64>,
    aliases: HashMap<IntentKey, IntentTarget>,
}

impl IntentKey {
    fn new(game_id: &str, source_epoch: Option<&str>, target: IntentTarget) -> Self {
        let target = match target {
            IntentTarget::ModPath(path) => IntentTarget::ModPath(location_key(Path::new(&path))),
            IntentTarget::ObjectId(id) => IntentTarget::ObjectId(id),
        };
        Self {
            game_id: game_id.to_string(),
            source_epoch: source_epoch.map(str::to_owned),
            target,
        }
    }
}

fn location_key(path: &Path) -> String {
    crate::shared::path_key::exact_location_key_for_path(path)
}

impl IntentRegistry {
    fn resolve(
        &mut self,
        game_id: &str,
        source_epoch: Option<&str>,
        target: IntentTarget,
    ) -> IntentKey {
        let path_identity = match &target {
            IntentTarget::ModPath(path) => {
                crate::platform::fs::file_utils::filesystem_identity(Path::new(path))
            }
            IntentTarget::ObjectId(_) => None,
        };
        let location = IntentKey::new(game_id, source_epoch, target);
        let target = path_identity
            .map(|id| IntentTarget::ModPath(format!("identity:{id}")))
            .or_else(|| self.aliases.get(&location).cloned())
            .unwrap_or_else(|| location.target.clone());
        if target != location.target {
            self.aliases.insert(location.clone(), target.clone());
        }
        IntentKey { target, ..location }
    }
}

pub struct IntentAdmission {
    revision: Option<u64>,
    keys: Vec<IntentKey>,
    latest: Arc<Mutex<IntentRegistry>>,
    source_epoch: Option<String>,
    path_targets: HashMap<String, IntentKey>,
}

impl IntentAdmission {
    pub(crate) fn is_current_object(&self, game_id: &str, object_id: &str) -> bool {
        let Some(revision) = self.revision else {
            return true;
        };
        let key = IntentKey::new(
            game_id,
            self.source_epoch.as_deref(),
            IntentTarget::ObjectId(object_id.to_string()),
        );
        crate::shared::sync::lock(&self.latest)
            .revisions
            .get(&key)
            .is_none_or(|latest| *latest <= revision)
    }
    pub(crate) fn bind_request_alias(&mut self, requested: &Path, resolved: &Path) {
        if let Some(key) = self.path_targets.get(&location_key(resolved)).cloned() {
            self.path_targets.insert(location_key(requested), key);
        }
    }
    pub(crate) fn validate_resolved_path(
        &self,
        requested: &Path,
        resolved: &Path,
    ) -> Result<(), AppError> {
        let expected = self
            .path_targets
            .get(&location_key(requested))
            .and_then(|key| match &key.target {
                IntentTarget::ModPath(identity) => identity.strip_prefix("identity:"),
                IntentTarget::ObjectId(_) => None,
            });
        let actual = crate::platform::fs::file_utils::filesystem_identity(resolved);
        if expected.is_none() || actual.as_deref() != expected {
            return Err(AppError::Io("Mod folder identity changed after the switch was requested; refresh the folder before retrying".to_string()));
        }
        Ok(())
    }
    pub fn is_current(&self) -> bool {
        let Some(revision) = self.revision else {
            return true;
        };
        let latest = crate::shared::sync::lock(&self.latest);
        self.keys.iter().all(|key| {
            latest
                .revisions
                .get(key)
                .is_some_and(|latest| *latest <= revision)
        })
    }

    pub fn is_current_path(&self, game_id: &str, path: &Path) -> bool {
        let Some(revision) = self.revision else {
            return true;
        };
        let mut latest = crate::shared::sync::lock(&self.latest);
        let key = self
            .path_targets
            .get(&location_key(path))
            .cloned()
            .unwrap_or_else(|| {
                latest.resolve(
                    game_id,
                    self.source_epoch.as_deref(),
                    IntentTarget::ModPath(path.to_string_lossy().into_owned()),
                )
            });
        latest
            .revisions
            .get(&key)
            .is_none_or(|latest| *latest <= revision)
    }
}

impl MutationCoordinator {
    pub fn unconfigured() -> Self {
        Self {
            lock: OperationLock::new(),
            journal: OnceLock::new(),
            task_registry: Arc::new(TaskRegistry::new()),
            latest_intents: Arc::new(Mutex::new(IntentRegistry::default())),
        }
    }

    pub fn with_lock(lock: OperationLock, journal: Arc<OperationJournal>) -> Self {
        let coordinator = Self {
            lock,
            journal: OnceLock::new(),
            task_registry: Arc::new(TaskRegistry::new()),
            latest_intents: Arc::new(Mutex::new(IntentRegistry::default())),
        };
        coordinator
            .configure(journal)
            .expect("new mutation coordinator must be configurable");
        coordinator
    }

    pub fn configure(&self, journal: Arc<OperationJournal>) -> Result<(), AppError> {
        self.journal.set(journal).map_err(|_| {
            AppError::Internal("Mutation coordinator is already configured".to_string())
        })
    }

    pub async fn acquire_exempt(
        &self,
        reason: MutationExemption,
    ) -> Result<MutationExemptionGuard, AppError> {
        Self::acquire_exemption_on(&self.lock, reason).await
    }

    pub async fn acquire_exemption_on(
        lock: &OperationLock,
        reason: MutationExemption,
    ) -> Result<MutationExemptionGuard, AppError> {
        Ok(MutationExemptionGuard {
            _guard: lock.acquire().await?,
            _reason: reason,
        })
    }

    /// Serialization envelope used only while a durable operation is planned
    /// under an already-owned collection recovery task.
    pub(crate) async fn acquire_nested_operation_lock(&self) -> Result<OpGuard, AppError> {
        self.lock.acquire().await
    }

    pub(crate) async fn acquire_nested_operation_on(
        lock: &OperationLock,
    ) -> Result<OpGuard, AppError> {
        lock.acquire().await
    }

    pub async fn acquire_operation(&self, plan: OperationPlan) -> Result<MutationGuard, AppError> {
        let guard = self.lock.acquire().await?;
        self.begin_operation(plan, Some(guard))
    }

    /// Start a durable plan while an outer recovery task already owns the
    /// coordinator's operation lock. Callers must retain that lock until this
    /// guard reaches a terminal state.
    pub fn begin_operation_under_lock(
        &self,
        plan: OperationPlan,
    ) -> Result<MutationGuard, AppError> {
        self.begin_operation(plan, None)
    }

    fn begin_operation(
        &self,
        plan: OperationPlan,
        guard: Option<OpGuard>,
    ) -> Result<MutationGuard, AppError> {
        let journal = self.journal()?;
        let operation_id = journal.plan_operation(plan)?;
        if !self.task_registry.register(operation_id.clone())? {
            journal.fail(
                &operation_id,
                format!("Mutation operation {operation_id} is already active"),
            )?;
            return Err(AppError::Internal(format!(
                "Mutation operation {operation_id} is already active"
            )));
        }
        journal.mark_applying(&operation_id)?;

        Ok(MutationGuard {
            _guard: guard,
            operation_id: Some(operation_id),
            journal: Some(journal.clone()),
            task_registry: self.task_registry.clone(),
            registered: true,
        })
    }

    pub fn inner_lock(&self) -> &OperationLock {
        &self.lock
    }

    pub fn task_registry(&self) -> &TaskRegistry {
        &self.task_registry
    }

    pub fn admit_intents(
        &self,
        game_id: &str,
        revision: Option<u64>,
        targets: impl IntoIterator<Item = IntentTarget>,
    ) -> IntentAdmission {
        self.admit_intents_in_epoch(game_id, None, revision, targets)
    }

    pub fn admit_intents_in_epoch(
        &self,
        game_id: &str,
        source_epoch: Option<&str>,
        revision: Option<u64>,
        targets: impl IntoIterator<Item = IntentTarget>,
    ) -> IntentAdmission {
        let mut latest = crate::shared::sync::lock(&self.latest_intents);
        let mut path_targets = HashMap::new();
        let keys = targets
            .into_iter()
            .map(|target| {
                let location = match &target {
                    IntentTarget::ModPath(path) => Some(location_key(Path::new(path))),
                    IntentTarget::ObjectId(_) => None,
                };
                let key = latest.resolve(game_id, source_epoch, target);
                if let Some(location) = location {
                    path_targets.insert(location, key.clone());
                }
                key
            })
            .collect::<Vec<_>>();
        if let Some(revision) = revision {
            // Keep the high-water mark after a command returns: IPC can deliver
            // an older request after the newer request has already completed.
            for key in &keys {
                latest
                    .revisions
                    .entry(key.clone())
                    .and_modify(|current| *current = (*current).max(revision))
                    .or_insert(revision);
            }
        }
        IntentAdmission {
            revision,
            keys,
            latest: self.latest_intents.clone(),
            source_epoch: source_epoch.map(str::to_owned),
            path_targets,
        }
    }

    pub fn pending_disk_commits(
        &self,
    ) -> Result<Vec<crate::modules::mutation::journal::Operation>, AppError> {
        Ok(self.journal()?.pending_disk_commits())
    }

    pub fn has_pending_disk_commit_for_game(&self, game_id: &str) -> Result<bool, AppError> {
        Ok(self.journal()?.has_pending_disk_commit_for_game(game_id))
    }

    pub fn latest_toggle_disk_revision(&self, game_id: &str) -> Result<u64, AppError> {
        self.latest_toggle_disk_revision_in_epoch(game_id, None)
    }

    pub(crate) fn latest_toggle_disk_revision_in_epoch(
        &self,
        game_id: &str,
        source_epoch: Option<&str>,
    ) -> Result<u64, AppError> {
        Ok(self
            .journal()?
            .entries()
            .into_iter()
            .filter(|operation| {
                operation.game_id == game_id
                    && source_epoch
                        .is_none_or(|epoch| operation.source_epoch.as_deref() == Some(epoch))
                    && matches!(operation.kind.as_str(), "workspace-switch" | "bulk-toggle")
            })
            .filter_map(|operation| operation.disk_revision)
            .max()
            .unwrap_or(0))
    }

    pub fn current_journal_revision(&self) -> Result<u64, AppError> {
        self.journal()?.current_revision()
    }

    pub(crate) fn earliest_toggle_projection_repair(
        &self,
        game_id: &str,
        source_epoch: &str,
    ) -> Result<Option<(u64, String)>, AppError> {
        Ok(self.repair_toggle_disk_commits_in_epoch(game_id, source_epoch)?.into_iter()
            .filter_map(|operation| operation.disk_revision.map(|revision| (revision, operation.last_error.unwrap_or_else(|| "Disk projection needs repair; resolve the affected folder conflict before capturing a collection".to_string()))))
            .min_by_key(|(revision, _)| *revision))
    }

    pub(crate) fn repair_toggle_disk_commits_in_epoch(
        &self,
        game_id: &str,
        source_epoch: &str,
    ) -> Result<Vec<crate::modules::mutation::journal::Operation>, AppError> {
        Ok(self
            .journal()?
            .entries()
            .into_iter()
            .filter(|operation| {
                operation.game_id == game_id
                    && operation.source_epoch.as_deref() == Some(source_epoch)
                    && matches!(operation.kind.as_str(), "workspace-switch" | "bulk-toggle")
                    && operation.database_projection_status
                        == crate::modules::mutation::journal::DatabaseProjectionStatus::NeedsRepair
            })
            .collect())
    }

    pub(crate) fn toggle_projection_lineage_in_epoch(
        &self,
        game_id: &str,
        source_epoch: &str,
    ) -> Result<Vec<crate::modules::mutation::journal::Operation>, AppError> {
        let mut operations = self
            .journal()?
            .entries()
            .into_iter()
            .filter(|operation| {
                operation.game_id == game_id
                    && operation.source_epoch.as_deref() == Some(source_epoch)
                    && operation.disk_revision.is_some()
                    && matches!(operation.kind.as_str(), "workspace-switch" | "bulk-toggle")
            })
            .collect::<Vec<_>>();
        operations.sort_by_key(|operation| operation.disk_revision);
        Ok(operations)
    }

    pub(crate) fn complete_repaired_disk_projection(
        &self,
        game_id: &str,
        source_epoch: &str,
        operation_ids: &[String],
    ) -> Result<(), AppError> {
        let repairs = self.repair_toggle_disk_commits_in_epoch(game_id, source_epoch)?;
        for id in operation_ids {
            if !repairs.iter().any(|operation| &operation.id == id) {
                return Err(AppError::Validation(
                    "Repair operation does not belong to the current root epoch".to_string(),
                ));
            }
        }
        for id in operation_ids {
            self.journal()?.mark_repaired_projection_committed(id)?;
            self.journal()?.complete(id)?;
        }
        Ok(())
    }

    pub fn pending_toggle_disk_commit_ids(&self, game_id: &str) -> Result<Vec<String>, AppError> {
        Ok(self
            .pending_disk_commits()?
            .into_iter()
            .filter(|operation| {
                operation.game_id == game_id
                    && matches!(operation.kind.as_str(), "workspace-switch" | "bulk-toggle")
            })
            .map(|operation| operation.id)
            .collect())
    }

    pub fn complete_disk_projection(&self, operation_ids: &[String]) -> Result<(), AppError> {
        self.journal()?.complete_disk_projection(operation_ids)
    }

    pub fn note_disk_projection_failure(
        &self,
        operation_ids: &[String],
        error: &str,
    ) -> Result<(), AppError> {
        let journal = self.journal()?;
        for operation_id in operation_ids {
            journal.mark_projection_failed(operation_id, error)?;
        }
        Ok(())
    }

    pub(crate) fn isolate_projection_for_repair(
        &self,
        operation_id: &str,
        error: &str,
    ) -> Result<(), AppError> {
        self.journal()?
            .isolate_disk_commit_for_repair(operation_id, error.to_string())
    }

    fn journal(&self) -> Result<&Arc<OperationJournal>, AppError> {
        self.journal
            .get()
            .ok_or_else(|| AppError::Internal("Mutation coordinator is not configured".to_string()))
    }
}

pub struct MutationGuard {
    _guard: Option<OpGuard>,
    operation_id: Option<String>,
    journal: Option<Arc<OperationJournal>>,
    task_registry: Arc<TaskRegistry>,
    registered: bool,
}

impl MutationGuard {
    pub fn operation_id(&self) -> Option<&str> {
        self.operation_id.as_deref()
    }

    pub fn op_guard(&self) -> &OpGuard {
        self._guard
            .as_ref()
            .expect("journal-only mutation guard has no operation lock")
    }

    pub fn mark_step_applied(&self, sequence: u32) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.mark_step_applied(operation_id, sequence)
    }

    pub fn mark_step_skipped(&self, sequence: u32) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.mark_step_skipped(operation_id, sequence)
    }

    pub fn settle_steps(
        &self,
        settlements: &[(u32, crate::modules::mutation::journal::StepSettlement)],
    ) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.settle_steps(operation_id, settlements)
    }

    pub fn mark_disk_committed(&self) -> Result<u64, AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.mark_disk_committed(operation_id)
    }

    pub fn mark_db_committed(&self) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.mark_db_committed(operation_id)
    }

    pub fn begin_rollback(&self) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.begin_rollback(operation_id)
    }

    pub fn abort_unapplied(mut self) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.abort_unapplied(operation_id)?;
        self.unregister();
        Ok(())
    }

    pub fn mark_step_rolled_back(&self, sequence: u32) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.mark_step_rolled_back(operation_id, sequence)
    }

    pub fn finish_rollback(mut self) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.finish_rollback(operation_id)?;
        self.unregister();
        Ok(())
    }

    pub fn commit(mut self) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.complete(operation_id)?;
        self.unregister();
        Ok(())
    }

    pub fn fail(mut self, error: impl Into<String>) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.fail(operation_id, error)?;
        self.unregister();
        Ok(())
    }

    fn durable_parts(&self) -> Result<(&str, &OperationJournal), AppError> {
        match (self.operation_id.as_deref(), self.journal.as_deref()) {
            (Some(operation_id), Some(journal)) => Ok((operation_id, journal)),
            _ => Err(AppError::Internal(
                "This mutation guard has no durable operation plan".to_string(),
            )),
        }
    }

    fn unregister(&mut self) {
        let Some(operation_id) = self.operation_id.as_deref() else {
            return;
        };
        if self.registered {
            if let Err(error) = self.task_registry.unregister(operation_id) {
                log::error!("Failed to unregister mutation operation {operation_id}: {error}");
            }
            self.registered = false;
        }
    }
}

impl Drop for MutationGuard {
    fn drop(&mut self) {
        self.unregister();
    }
}

#[cfg(test)]
mod architecture_tests {
    use std::path::{Path, PathBuf};

    use super::{IntentTarget, MutationCoordinator};

    #[test]
    fn physical_folders_with_prefix_equivalent_paths_do_not_supersede_each_other() {
        let temp = tempfile::tempdir().unwrap();
        let enabled = temp.path().join("Alice/Skin");
        let disabled = temp.path().join("DISABLED Alice/Skin");
        std::fs::create_dir_all(&enabled).unwrap();
        std::fs::create_dir_all(&disabled).unwrap();
        let coordinator = MutationCoordinator::unconfigured();
        let first = coordinator.admit_intents(
            "game",
            Some(1),
            [IntentTarget::ModPath(
                enabled.to_string_lossy().into_owned(),
            )],
        );
        coordinator.admit_intents(
            "game",
            Some(2),
            [IntentTarget::ModPath(
                disabled.to_string_lossy().into_owned(),
            )],
        );
        assert!(
            first.is_current(),
            "distinct disk identities must not share admission"
        );
    }

    #[test]
    fn replaced_folder_does_not_inherit_previous_intent_identity() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("Alice");
        std::fs::create_dir(&path).unwrap();
        let coordinator = MutationCoordinator::unconfigured();
        let first = coordinator.admit_intents(
            "game",
            Some(1),
            [IntentTarget::ModPath(path.to_string_lossy().into_owned())],
        );
        std::fs::rename(&path, temp.path().join("Original")).unwrap();
        std::fs::create_dir(&path).unwrap();
        coordinator.admit_intents(
            "game",
            Some(2),
            [IntentTarget::ModPath(path.to_string_lossy().into_owned())],
        );
        assert!(
            first.is_current(),
            "a replacement folder is a distinct target"
        );
        assert!(
            first.validate_resolved_path(&path, &path).is_err(),
            "a queued switch cannot rebind to the replacement"
        );
    }

    #[test]
    fn later_folder_intent_supersedes_both_storage_spellings() {
        let temp = tempfile::tempdir().unwrap();
        let old_path = temp.path().join("DISABLED Alice");
        let new_path = temp.path().join("Alice");
        std::fs::create_dir(&old_path).unwrap();
        let coordinator = MutationCoordinator::unconfigured();
        let old = coordinator.admit_intents(
            "game",
            Some(10),
            [IntentTarget::ModPath(
                old_path.to_string_lossy().into_owned(),
            )],
        );
        std::fs::rename(&old_path, &new_path).unwrap();
        let latest = coordinator.admit_intents(
            "game",
            Some(11),
            [IntentTarget::ModPath(
                new_path.to_string_lossy().into_owned(),
            )],
        );
        assert!(!old.is_current());
        assert!(latest.is_current());
        assert!(!old.is_current_path("game", &new_path));
        assert!(latest.is_current_path("game", &old_path));
        assert!(
            old.validate_resolved_path(&old_path, &new_path).is_ok(),
            "proven rename alias keeps its identity"
        );
    }

    #[test]
    fn thousand_rapid_intents_keep_only_the_latest_revision_for_one_folder() {
        let temp = tempfile::tempdir().unwrap();
        let enabled = temp.path().join("Alice");
        let disabled = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&disabled).unwrap();
        let coordinator = MutationCoordinator::unconfigured();
        let first = coordinator.admit_intents(
            "game",
            Some(1),
            [IntentTarget::ModPath(
                disabled.to_string_lossy().into_owned(),
            )],
        );
        for revision in 2..=1_000 {
            let path = if revision % 2 == 0 {
                std::fs::rename(&disabled, &enabled).unwrap();
                &enabled
            } else {
                std::fs::rename(&enabled, &disabled).unwrap();
                &disabled
            };
            coordinator.admit_intents(
                "game",
                Some(revision),
                [IntentTarget::ModPath(path.to_string_lossy().into_owned())],
            );
        }
        let latest = coordinator.admit_intents(
            "game",
            Some(1_000),
            [IntentTarget::ModPath(
                enabled.to_string_lossy().into_owned(),
            )],
        );
        assert!(!first.is_current());
        assert!(latest.is_current());
        assert_eq!(
            crate::shared::sync::lock(&coordinator.latest_intents)
                .revisions
                .len(),
            1
        );
    }

    #[test]
    fn older_bulk_arriving_after_newer_single_stays_superseded() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("Alice");
        let disabled = temp.path().join("DISABLED Alice");
        std::fs::create_dir(&path).unwrap();
        let coordinator = MutationCoordinator::unconfigured();
        let latest = coordinator.admit_intents(
            "game",
            Some(30),
            [IntentTarget::ModPath(path.to_string_lossy().into_owned())],
        );
        drop(latest);
        std::fs::rename(&path, &disabled).unwrap();
        let bulk = coordinator.admit_intents(
            "game",
            Some(29),
            [
                IntentTarget::ModPath(disabled.to_string_lossy().into_owned()),
                IntentTarget::ModPath("E:/Mods/Bob".into()),
            ],
        );
        assert!(!bulk.is_current_path("game", &disabled));
        assert!(bulk.is_current_path("game", Path::new("E:/Mods/Bob")));
    }

    #[test]
    fn equal_names_under_different_parents_and_games_remain_distinct() {
        let coordinator = MutationCoordinator::unconfigured();
        let first = coordinator.admit_intents(
            "game-a",
            Some(1),
            [IntentTarget::ModPath("E:/Mods/Alice/Skin".into())],
        );
        coordinator.admit_intents(
            "game-a",
            Some(2),
            [IntentTarget::ModPath("E:/Mods/Bob/Skin".into())],
        );
        coordinator.admit_intents(
            "game-b",
            Some(3),
            [IntentTarget::ModPath("E:/Mods/Alice/Skin".into())],
        );
        assert!(first.is_current());
    }

    #[test]
    fn object_id_intents_share_one_order_across_single_and_batch() {
        let coordinator = MutationCoordinator::unconfigured();
        let batch = coordinator.admit_intents(
            "game",
            Some(4),
            [
                IntentTarget::ObjectId("alice".into()),
                IntentTarget::ObjectId("bob".into()),
            ],
        );
        coordinator.admit_intents("game", Some(5), [IntentTarget::ObjectId("bob".into())]);
        assert!(!batch.is_current());
    }

    #[test]
    fn unicode_path_components_do_not_panic_during_key_normalization() {
        let coordinator = MutationCoordinator::unconfigured();
        let admission = coordinator.admit_intents(
            "game",
            Some(1),
            [IntentTarget::ModPath("E:/Mods/東京/衣装".into())],
        );
        assert!(admission.is_current());
    }

    #[test]
    fn extended_length_windows_spelling_shares_the_same_intent_key() {
        let coordinator = MutationCoordinator::unconfigured();
        let older = coordinator.admit_intents(
            "game",
            Some(1),
            [IntentTarget::ModPath("E:\\Mods\\DISABLED Blue".into())],
        );
        coordinator.admit_intents(
            "game",
            Some(2),
            [IntentTarget::ModPath(
                "\\\\?\\E:\\Mods\\DISABLED Blue".into(),
            )],
        );
        assert!(!older.is_current());
    }

    fn rust_files(root: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(root).expect("read source directory") {
            let path = entry.expect("read source entry").path();
            if path.is_dir() {
                rust_files(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    #[test]
    fn production_code_has_no_generic_coordinator_acquire_or_root_pipeline() {
        let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let coordinator_path = source_root.join("modules/mutation/coordinator.rs");
        let mut files = Vec::new();
        rust_files(&source_root, &mut files);
        let mut violations = Vec::new();

        for path in files {
            if path == coordinator_path
                || path.file_name().is_some_and(|name| name == "tests.rs")
                || path.components().any(|part| part.as_os_str() == "tests")
            {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read Rust source");
            let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
            for banned in [
                "op_lock.acquire().await",
                "operation_lock.acquire().await",
                "coordinator.acquire().await",
                "crate::pipeline",
            ] {
                if production.contains(banned) {
                    violations.push(format!("{}: {banned}", path.display()));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "durable mutation architecture violations:\n{}",
            violations.join("\n")
        );
    }
}
