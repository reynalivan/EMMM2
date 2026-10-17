//! Executes runtime toggle batches: renames mod folders on disk. Filesystem
//! is the source of truth — the `mods` rows and `object_runtime_projection`
//! converge via the scoped disk reconcile the caller runs after the batch.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::modules::library::application::mods::core_ops::standardize_prefix;
#[cfg(test)]
use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;
use crate::platform::fs::file_utils::{FilesystemIdentityProof, FilesystemNamespaceProof};
use crate::platform::fs::rename::rename_no_replace;
use crate::platform::fs::retry::{blocking_error, RetryBudget};
use crate::shared::errors::{AppError, CollectionError};

#[derive(Debug)]
pub struct RuntimeToggleFailure {
    pub error: CollectionError,
    pub rollback_warnings: Vec<String>,
}

impl std::fmt::Display for RuntimeToggleFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for RuntimeToggleFailure {}

fn failure(error: CollectionError) -> RuntimeToggleFailure {
    RuntimeToggleFailure {
        error,
        rollback_warnings: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeToggleTarget {
    pub id: String,
    pub folder_path: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeToggleOperation {
    pub folder_path: String,
    pub target_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct RuntimeToggleBatchRequest {
    pub mods_path: PathBuf,
    pub operations: Vec<RuntimeToggleOperation>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct RuntimeToggleResult {
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub warnings: Vec<String>,
    pub path_rewrites: Vec<WorkspacePathRewrite>,
    /// Every absolute path the batch touched (old and new sides), for the
    /// caller's scoped reconcile.
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeRenamePlan {
    old_abs: PathBuf,
    requested_abs: PathBuf,
    new_abs: PathBuf,
    target_enabled: bool,
    expected_identity: String,
    expected_parent_identity: Option<String>,
    namespace_proof: Option<Arc<FilesystemNamespaceProof>>,
}

impl RuntimeRenamePlan {
    pub(crate) fn new(
        old_abs: PathBuf,
        requested_abs: PathBuf,
        new_abs: PathBuf,
        target_enabled: bool,
        expected_identity: String,
    ) -> Self {
        let expected_parent_identity = old_abs
            .parent()
            .and_then(crate::platform::fs::file_utils::filesystem_identity);
        Self {
            old_abs,
            requested_abs,
            new_abs,
            target_enabled,
            expected_identity,
            expected_parent_identity,
            namespace_proof: None,
        }
    }

    pub(crate) fn set_namespace_proof(&mut self, proof: Arc<FilesystemNamespaceProof>) {
        self.namespace_proof = Some(proof);
    }

    pub fn old_path(&self) -> &Path {
        &self.old_abs
    }
    pub fn new_path(&self) -> &Path {
        &self.new_abs
    }
    pub fn requested_path(&self) -> &Path {
        &self.requested_abs
    }
    pub fn target_enabled(&self) -> bool {
        self.target_enabled
    }
    pub fn expected_identity(&self) -> &str {
        &self.expected_identity
    }
    #[cfg(test)]
    pub fn apply(&self) -> std::io::Result<()> {
        self.rename_checked(false)
    }

    fn rename_checked(&self, reverse: bool) -> std::io::Result<()> {
        let (source, destination) = if reverse {
            (&self.new_abs, &self.old_abs)
        } else {
            (&self.old_abs, &self.new_abs)
        };
        let paths = [source.to_path_buf(), destination.to_path_buf()];
        let namespace_pins = self
            .namespace_proof
            .as_ref()
            .map(|proof| {
                proof
                    .pin_paths(&paths)
                    .map_err(|error| std::io::Error::other(error.to_string()))
            })
            .transpose()?;
        let parent_identity = source
            .parent()
            .and_then(crate::platform::fs::file_utils::filesystem_identity);
        if self.expected_parent_identity.is_none()
            || parent_identity != self.expected_parent_identity
        {
            return Err(std::io::Error::other(
                "Runtime rename parent changed after planning",
            ));
        }
        let actual_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(source);
        if actual_identity.as_deref() != Some(self.expected_identity.as_str()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Folder changed while preparing the rename: {}",
                    source.display()
                ),
            ));
        }
        if source == destination {
            return Ok(());
        }
        if destination.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Target folder already exists: {}", destination.display()),
            ));
        }
        if let Some(pins) = namespace_pins.as_ref() {
            let parent = source
                .parent()
                .ok_or_else(|| std::io::Error::other("Runtime rename has no parent"))?;
            let parent = crate::shared::path_key::physical_namespace_path(parent)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let parent_pin = pins
                .iter()
                .find(|pin| pin.path() == parent.as_path())
                .ok_or_else(|| std::io::Error::other("Runtime rename parent is not pinned"))?;
            let source_name = source
                .file_name()
                .ok_or_else(|| std::io::Error::other("Runtime rename has no source name"))?;
            let destination_name = destination
                .file_name()
                .ok_or_else(|| std::io::Error::other("Runtime rename has no target name"))?;
            let owned_source = crate::platform::fs::rename::PinnedDirectory::open_for_rename(
                &parent_pin.stable_path().join(source_name),
            )?;
            if owned_source.identity() != self.expected_identity {
                return Err(std::io::Error::other(
                    "Runtime rename source changed after namespace pinning",
                ));
            }
            owned_source.rename_into(parent_pin, destination_name)
        } else {
            rename_no_replace(source, destination)
        }
    }

    #[cfg(test)]
    pub fn rollback(&self) -> std::io::Result<()> {
        self.rename_checked(true)
    }

    pub(crate) async fn apply_async(&self, budget: &mut RetryBudget) -> std::io::Result<()> {
        self.rename_async(false, budget).await
    }

    async fn rename_async(&self, reverse: bool, budget: &mut RetryBudget) -> std::io::Result<()> {
        budget
            .run(|| {
                let plan = self.clone();
                async move {
                    tokio::task::spawn_blocking(move || plan.rename_checked(reverse))
                        .await
                        .map_err(blocking_error)?
                }
            })
            .await
    }

    #[cfg(test)]
    pub(crate) async fn rollback_async(&self, budget: &mut RetryBudget) -> std::io::Result<()> {
        self.rename_async(true, budget).await
    }
}

pub(crate) fn classify_rename_failure_with_identity(
    src: &Path,
    expected: &str,
    error: std::io::Error,
) -> CollectionError {
    match crate::platform::fs::retry::map_file_error_with_identity(
        src,
        expected,
        "rename mod folder",
        error,
    ) {
        AppError::FileLock { path, recovery_id } => CollectionError::FileLock { path, recovery_id },
        AppError::PathBusy { path } => CollectionError::PathBusy { path },
        other => CollectionError::Io(other.to_string()),
    }
}

#[cfg(test)]
pub async fn toggle_mods_mixed(
    request: RuntimeToggleBatchRequest,
) -> Result<RuntimeToggleResult, RuntimeToggleFailure> {
    if request.operations.is_empty() {
        return Ok(empty_result());
    }

    // Planning either yields a plan, skips a no-op, or aborts the whole batch —
    // it never produces warnings, so an empty plan set is just an empty result.
    let plans = plan_runtime_toggles(&request)?;
    if plans.is_empty() {
        return Ok(empty_result());
    }

    // Only rollback populates warnings, and rollback cannot run before this point.
    let mut warnings = Vec::new();

    let mut renamed = Vec::new();
    let mut retry_budget = RetryBudget::default();
    for plan in &plans {
        if plan.old_abs == plan.new_abs {
            renamed.push(plan.clone());
            continue;
        }

        match plan.apply_async(&mut retry_budget).await {
            Ok(()) => renamed.push(plan.clone()),
            Err(error) => {
                rollback_successes(&renamed, &mut warnings).await;
                // Classify before stringifying, so a folder held by the game
                // reports the holding process rather than "Access is denied".
                return Err(RuntimeToggleFailure {
                    error: classify_rename_failure_with_identity(
                        &plan.old_abs,
                        &plan.expected_identity,
                        error,
                    ),
                    rollback_warnings: warnings,
                });
            }
        }
    }

    let enabled_count = renamed.iter().filter(|plan| plan.target_enabled).count();
    let disabled_count = renamed.len().saturating_sub(enabled_count);

    Ok(RuntimeToggleResult {
        enabled_count,
        disabled_count,
        warnings,
        path_rewrites: renamed
            .iter()
            .filter(|plan| plan.requested_abs != plan.new_abs)
            .map(|plan| WorkspacePathRewrite {
                old_path: plan.requested_abs.to_string_lossy().to_string(),
                new_path: plan.new_abs.to_string_lossy().to_string(),
            })
            .collect(),
        changed_paths: renamed
            .iter()
            .flat_map(|plan| {
                [
                    plan.old_abs.to_string_lossy().to_string(),
                    plan.new_abs.to_string_lossy().to_string(),
                ]
            })
            .collect(),
    })
}

pub fn plan_runtime_toggles(
    request: &RuntimeToggleBatchRequest,
) -> Result<Vec<RuntimeRenamePlan>, RuntimeToggleFailure> {
    let root_proof = FilesystemIdentityProof::capture(&request.mods_path)
        .map_err(|error| failure(CollectionError::Validation(error.to_string())))?;
    let mut plans = Vec::new();
    for operation in &request.operations {
        match build_plan(&request.mods_path, operation) {
            Ok(Some(plan)) => plans.push(plan),
            Ok(None) => {}
            Err(error) => return Err(failure(CollectionError::Validation(error.to_string()))),
        }
    }
    validate_plans(&plans)
        .map_err(|error| failure(CollectionError::Validation(error.to_string())))?;
    if !plans.is_empty() {
        root_proof
            .validate(&request.mods_path)
            .map_err(|error| failure(CollectionError::Validation(error.to_string())))?;
        let paths = plans
            .iter()
            .flat_map(|plan| [plan.old_abs.clone(), plan.new_abs.clone()])
            .collect::<Vec<_>>();
        let namespace = Arc::new(
            FilesystemNamespaceProof::capture(&request.mods_path, &paths)
                .map_err(|error| failure(CollectionError::Validation(error.to_string())))?,
        );
        root_proof
            .validate(&request.mods_path)
            .map_err(|error| failure(CollectionError::Validation(error.to_string())))?;
        for plan in &mut plans {
            plan.set_namespace_proof(namespace.clone());
        }
    }
    Ok(plans)
}

#[cfg(test)]
fn empty_result() -> RuntimeToggleResult {
    RuntimeToggleResult {
        enabled_count: 0,
        disabled_count: 0,
        warnings: Vec::new(),
        path_rewrites: Vec::new(),
        changed_paths: Vec::new(),
    }
}

fn build_plan(
    mods_path: &Path,
    operation: &RuntimeToggleOperation,
) -> Result<Option<RuntimeRenamePlan>, AppError> {
    validate_relative_path(&operation.folder_path)?;

    let requested_abs = mods_path.join(&operation.folder_path);
    let old_abs =
        crate::modules::library::application::mods::core_ops::resolve_existing_runtime_variant(
            mods_path,
            &requested_abs,
            operation.target_enabled,
        )
        .unwrap_or_else(|| requested_abs.clone());
    if !old_abs.exists() {
        return Err(AppError::Internal(format!(
            "Mod folder does not exist: {}",
            old_abs.display()
        )));
    }

    let old_name = old_abs
        .file_name()
        .ok_or_else(|| {
            AppError::Internal(format!("Mod path has no file name: {}", old_abs.display()))
        })?
        .to_string_lossy()
        .to_string();
    let new_name = standardize_prefix(&old_name, operation.target_enabled);
    if new_name == old_name && requested_abs == old_abs {
        return Ok(None);
    }

    let new_abs = old_abs.with_file_name(&new_name);
    if new_abs.exists() && new_abs != old_abs {
        return Err(AppError::Internal(format!(
            "Target folder already exists: {}",
            new_abs.display()
        )));
    }

    if new_abs.strip_prefix(mods_path).is_err() {
        return Err(AppError::Security(format!(
            "Resolved path escaped mods root: {}",
            new_abs.display()
        )));
    }

    let expected_identity = crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&old_abs)
        .ok_or_else(|| {
            AppError::Io(format!(
                "Could not establish filesystem identity for {}",
                old_abs.display()
            ))
        })?;

    Ok(Some(RuntimeRenamePlan::new(
        old_abs,
        requested_abs,
        new_abs,
        operation.target_enabled,
        expected_identity,
    )))
}

#[cfg(test)]
async fn rollback_successes(plans: &[RuntimeRenamePlan], warnings: &mut Vec<String>) {
    let mut retry_budget = RetryBudget::default();
    for plan in plans.iter().rev() {
        if plan.old_abs == plan.new_abs {
            continue;
        }

        if !plan.new_abs.exists() {
            warnings.push(format!(
                "Rollback skipped for '{}': source missing",
                plan.new_abs.display()
            ));
            continue;
        }

        if plan.old_abs.exists() {
            warnings.push(format!(
                "Rollback skipped for '{}': target already exists",
                plan.old_abs.display()
            ));
            continue;
        }

        if let Err(error) = plan.rollback_async(&mut retry_budget).await {
            warnings.push(format!(
                "Rollback failed for '{}' to '{}': {error}",
                plan.new_abs.display(),
                plan.old_abs.display()
            ));
        }
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;

#[path = "engine_validation.rs"]
mod validation;
use validation::{validate_plans, validate_relative_path};
