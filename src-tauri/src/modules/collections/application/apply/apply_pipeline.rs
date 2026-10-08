//! Durable collection apply orchestration.

use std::collections::HashSet;
use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::modules::collections::application::collection::ApplyCollectionRequest;
use crate::modules::collections::domain::collection::{
    ApplyResult, Collection, CollectionMod, CollectionObject,
};
use crate::modules::settings::application::config::AppSettings;
use crate::modules::workspace::application::scanner::watcher::WatcherSuppressor;
use crate::modules::workspace::domain::task::TaskStatus;
use crate::modules::workspace::domain::workspace::WorkspacePathRewrite;
use crate::shared::errors::CollectionError;

/// Context passed through all pipeline steps during a collection apply.
pub struct ApplyContext {
    pub pool: SqlitePool,
    pub game_id: String,
    pub collection_id: String,
    pub mods_path: PathBuf,
    pub suppressor: std::sync::Arc<WatcherSuppressor>,
    pub ignore_missing: bool,
    pub settings: AppSettings,
    pub finalize_active_collection: bool,
    pub final_active_collection_id: Option<String>,
    pub mutation_started: bool,
    pub runtime_target_state:
        Option<crate::modules::collections::domain::collection::ProjectedCollectionState>,
    pub requested_target_state:
        Option<crate::modules::collections::domain::collection::ProjectedCollectionState>,
    pub defer_task_completion: bool,
    pub rollback_collection_id: Option<String>,
    pub rollback_active_collection_id: Option<String>,

    pub collection: Option<Collection>,
    pub target_mods: Vec<CollectionMod>,
    pub target_objects: Vec<CollectionObject>,
    pub currently_enabled_path_keys: HashSet<String>,
    /// Collection-owned paths that Safe Mode may physically mutate. This keeps
    /// external/unmanaged mods outside the Safe Mode scope.
    pub safe_mode_scope_path_keys: HashSet<String>,
    pub to_enable: Vec<String>,
    pub to_disable: Vec<String>,
    pub warnings: Vec<String>,
    pub final_state_name: Option<String>,
    pub skipped_missing_paths: Vec<String>,
    pub runtime_path_rewrites: Vec<WorkspacePathRewrite>,
    /// Runtime filter derived from per-game Safe Mode. The collection snapshot
    /// remains the requested state and is never rewritten by this filter.
    pub safe_mode: bool,
    /// F5 transitions only mutate members owned by the selected managed
    /// collection, including when turning Safe Mode off. Ordinary preset
    /// application retains its existing whole-runtime reconciliation behavior.
    pub restrict_current_state_to_target_scope: bool,
    pub(crate) prepared_renames: Option<super::steps::batch_rename::PreparedCollectionRenames>,
    pub(crate) mutation_guard: Option<crate::modules::mutation::coordinator::MutationGuard>,

    pub mods_enabled: usize,
    pub mods_disabled: usize,
}

impl ApplyContext {
    /// The collection row loaded once by the validation step.
    pub fn collection(&self) -> Result<&Collection, CollectionError> {
        self.collection
            .as_ref()
            .ok_or_else(|| CollectionError::NotFound {
                id: self.collection_id.clone(),
            })
    }

    /// Seed a context from the caller's request. Takes the borrowed request
    /// type directly — an owned intermediate struct would just re-declare the
    /// same eight fields a third time.
    pub fn new(request: ApplyCollectionRequest<'_>) -> Self {
        Self {
            pool: request.pool.clone(),
            game_id: request.game_id.to_string(),
            collection_id: request.collection_id.to_string(),
            mods_path: request.mods_path,
            suppressor: request.suppressor,
            ignore_missing: request.ignore_missing,
            safe_mode: request
                .settings
                .safety
                .runtime_safe_mode_for(request.game_id),
            settings: request.settings,
            finalize_active_collection: request.capture_last_changes,
            final_active_collection_id: request
                .capture_last_changes
                .then(|| request.collection_id.to_string()),
            mutation_started: false,
            runtime_target_state: None,
            requested_target_state: None,
            defer_task_completion: false,
            rollback_collection_id: None,
            rollback_active_collection_id: None,
            collection: None,
            target_mods: Vec::new(),
            target_objects: Vec::new(),
            currently_enabled_path_keys: HashSet::new(),
            safe_mode_scope_path_keys: HashSet::new(),
            restrict_current_state_to_target_scope: false,
            to_enable: Vec::new(),
            to_disable: Vec::new(),
            warnings: Vec::new(),
            final_state_name: None,
            skipped_missing_paths: Vec::new(),
            runtime_path_rewrites: Vec::new(),
            prepared_renames: None,
            mutation_guard: None,
            mods_enabled: 0,
            mods_disabled: 0,
        }
    }
}

/// Disk Reconcile must not perform these physical collection renames during a
/// passive startup or watcher refresh.
pub async fn execute(
    ctx: &mut ApplyContext,
    task_id: &str,
    expected_status: TaskStatus,
    settle_normal_failure: bool,
    mutation_coordinator: Option<&crate::modules::mutation::coordinator::MutationCoordinator>,
) -> Result<ApplyResult, CollectionError> {
    crate::modules::library::application::apply_progress::start(&ctx.game_id);

    match execute_inner(ctx, mutation_coordinator).await {
        Ok(result) => {
            if let Err(error) = finalize_apply(ctx, task_id, expected_status).await {
                if settle_normal_failure {
                    settle_normal_apply_failure(ctx, task_id, true).await;
                }
                finish_failed_apply(ctx);
                return Err(error);
            }
            if let Some(guard) = ctx.mutation_guard.take() {
                guard.mark_db_committed().map_err(mutation_error)?;
                guard.commit().map_err(mutation_error)?;
            }
            crate::modules::library::application::apply_progress::finish(
                &ctx.game_id,
                result.final_state_name.clone(),
                result.warnings.clone(),
                true,
            );
            Ok(result)
        }
        Err(error) => {
            if settle_normal_failure {
                settle_normal_apply_failure(ctx, task_id, false).await;
            }
            finish_failed_apply(ctx);
            Err(error)
        }
    }
}

async fn finalize_apply(
    ctx: &ApplyContext,
    task_id: &str,
    expected_status: TaskStatus,
) -> Result<(), CollectionError> {
    let outcome: Result<(), sqlx::Error> = async {
        let mut tx = ctx.pool.begin().await?;
        if ctx.finalize_active_collection {
            crate::modules::collections::adapters::sqlite::runtime::set_active_tx(
                &mut tx,
                &ctx.game_id,
                ctx.final_active_collection_id.as_deref(),
            )
            .await?;
        }
        finalize_task_snapshot(ctx, &mut tx, task_id, expected_status).await?;
        tx.commit().await
    }
    .await;

    outcome.map_err(|error| {
        log::error!("apply_pipeline: failed to finalize task '{task_id}': {error}");
        CollectionError::Db(format!(
            "Applied collection but failed to finalize recovery task '{task_id}': {error}"
        ))
    })
}

async fn finalize_task_snapshot(
    ctx: &ApplyContext,
    conn: &mut sqlx::SqliteConnection,
    task_id: &str,
    expected: TaskStatus,
) -> Result<(), sqlx::Error> {
    if ctx.defer_task_completion {
        return Ok(());
    }
    let completed = crate::modules::workspace::adapters::sqlite::task::compare_and_set_status_tx(
        conn,
        task_id,
        expected,
        TaskStatus::Completed,
    )
    .await?;
    if !completed {
        return Err(sqlx::Error::RowNotFound);
    }
    if let Some(snapshot) = ctx
        .requested_target_state
        .as_ref()
        .filter(|_| ctx.safe_mode)
    {
        crate::modules::collections::adapters::sqlite::safe_mode::set_snapshot_tx(
            conn,
            &ctx.game_id,
            Some(snapshot),
        )
        .await
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    }
    Ok(())
}

async fn settle_normal_apply_failure(ctx: &ApplyContext, task_id: &str, finalization_failed: bool) {
    let status = if finalization_failed || ctx.mutation_started {
        TaskStatus::Pending
    } else {
        TaskStatus::Failed
    };
    if let Err(error) = crate::modules::workspace::adapters::sqlite::task::compare_and_set_status(
        &ctx.pool,
        task_id,
        TaskStatus::Running,
        status,
    )
    .await
    {
        log::error!("apply_pipeline: failed to mark task '{task_id}' as failed: {error}");
    }
}

fn finish_failed_apply(ctx: &ApplyContext) {
    crate::modules::library::application::apply_progress::finish(
        &ctx.game_id,
        ctx.final_state_name.clone(),
        ctx.warnings.clone(),
        false,
    );
}

async fn execute_inner(
    ctx: &mut ApplyContext,
    mutation_coordinator: Option<&crate::modules::mutation::coordinator::MutationCoordinator>,
) -> Result<ApplyResult, CollectionError> {
    crate::modules::library::application::apply_progress::update(
        &ctx.game_id,
        "preparing",
        0,
        0,
        None,
    );
    if !ctx.mods_path.exists() || !ctx.mods_path.is_dir() {
        return Err(CollectionError::RuntimeState(
            crate::shared::errors::RuntimeStateError::NoModsPath {
                game_id: ctx.game_id.clone(),
            },
        ));
    }

    if ctx.runtime_target_state.is_none() {
        super::steps::validate_collection::validate(ctx).await?;
    }

    crate::modules::library::application::apply_progress::update(
        &ctx.game_id,
        "diffing",
        0,
        0,
        None,
    );
    super::steps::resolve_target::resolve(ctx).await?;

    super::steps::validate_paths::validate(ctx).await?;
    crate::modules::library::application::apply_progress::set_warnings(
        &ctx.game_id,
        ctx.warnings.clone(),
    );

    super::steps::resolve_current_state::resolve(ctx).await?;
    super::steps::resolve_current_state::compute_diff(ctx);
    let steps = super::steps::batch_rename::prepare(ctx).await?;
    ctx.mutation_started = !steps.is_empty();
    if let Some(coordinator) = mutation_coordinator.filter(|_| !steps.is_empty()) {
        ctx.mutation_guard = Some(
            coordinator
                .begin_operation_under_lock(crate::modules::mutation::journal::OperationPlan::new(
                    "collection-apply",
                    &ctx.game_id,
                    steps,
                ))
                .map_err(mutation_error)?,
        );
    }

    crate::modules::library::application::apply_progress::update(
        &ctx.game_id,
        "renaming",
        0,
        ctx.to_enable.len() + ctx.to_disable.len(),
        None,
    );
    super::steps::batch_rename::rename(ctx).await?;

    crate::modules::library::application::apply_progress::update(
        &ctx.game_id,
        "verifying",
        ctx.mods_enabled + ctx.mods_disabled,
        ctx.to_enable.len() + ctx.to_disable.len(),
        None,
    );

    ctx.final_state_name = ctx
        .collection
        .as_ref()
        .map(|collection| collection.name.clone());

    let apply_result = ApplyResult {
        mods_enabled: ctx.mods_enabled,
        mods_disabled: ctx.mods_disabled,
        warnings: ctx.warnings.clone(),
        final_state_name: ctx.final_state_name.clone(),
        partial_apply: !ctx.skipped_missing_paths.is_empty(),
        skipped_missing_paths: ctx.skipped_missing_paths.clone(),
        runtime_path_rewrites: ctx.runtime_path_rewrites.clone(),
        sync_warning: None,
    };
    Ok(apply_result)
}

fn mutation_error(error: crate::shared::errors::AppError) -> CollectionError {
    CollectionError::Io(error.to_string())
}

#[cfg(test)]
#[path = "apply_pipeline_tests.rs"]
mod tests;
