use std::sync::{Arc, OnceLock};

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
}

impl MutationCoordinator {
    pub fn unconfigured() -> Self {
        Self {
            lock: OperationLock::new(),
            journal: OnceLock::new(),
            task_registry: Arc::new(TaskRegistry::new()),
        }
    }

    pub fn with_lock(lock: OperationLock, journal: Arc<OperationJournal>) -> Self {
        let coordinator = Self {
            lock,
            journal: OnceLock::new(),
            task_registry: Arc::new(TaskRegistry::new()),
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

    pub fn mark_db_committed(&self) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.mark_db_committed(operation_id)
    }

    pub fn begin_rollback(&self) -> Result<(), AppError> {
        let (operation_id, journal) = self.durable_parts()?;
        journal.begin_rollback(operation_id)
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
