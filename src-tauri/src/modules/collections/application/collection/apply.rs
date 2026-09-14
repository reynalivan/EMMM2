//! Entry point that hands an apply request to the apply pipeline.

use crate::modules::collections::domain::collection::ApplyResult;
use crate::modules::workspace::domain::task::{TaskStatus, TASK_TYPE_APPLY_COLLECTION};
use crate::shared::errors::{AppError, CollectionError};
use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Conservative paths that collection apply may mutate: target members can be
/// enabled, while currently enabled mods can be disabled.
pub(crate) async fn collection_preflight_scope_paths(
    pool: &SqlitePool,
    game_id: &str,
    collection_id: &str,
    mods_path: &Path,
) -> Result<Vec<String>, AppError> {
    let mods = crate::modules::collections::adapters::sqlite::get_mods(pool, collection_id).await?;
    let objects =
        crate::modules::collections::adapters::sqlite::get_objects(pool, collection_id).await?;
    let enabled_mod_paths =
        crate::modules::library::adapters::sqlite::mods::get_enabled_mods_paths(pool, game_id)
            .await?;
    let mut paths = BTreeSet::new();
    for relative_path in mods
        .into_iter()
        .map(|member| member.mod_path)
        .chain(objects.into_iter().filter_map(|member| member.path_key))
        .chain(enabled_mod_paths.into_iter().map(|path| path.into_stored()))
    {
        let path = PathBuf::from(relative_path);
        paths.insert(
            if path.is_absolute() {
                path
            } else {
                mods_path.join(path)
            }
            .to_string_lossy()
            .to_string(),
        );
    }
    Ok(paths.into_iter().collect())
}

pub struct ApplyCollectionRequest<'a> {
    pub pool: &'a SqlitePool,
    pub game_id: &'a str,
    pub collection_id: &'a str,
    pub capture_last_changes: bool,
    pub mods_path: std::path::PathBuf,
    pub suppressor:
        std::sync::Arc<crate::modules::workspace::application::scanner::watcher::WatcherSuppressor>,
    pub ignore_missing: bool,
    pub settings: crate::modules::settings::application::config::AppSettings,
}

#[cfg(test)]
#[derive(Clone)]
pub(crate) struct ApplyExecutionBarrier {
    pub game_id: String,
    pub entered: std::sync::Arc<tokio::sync::Barrier>,
    pub release: std::sync::Arc<tokio::sync::Barrier>,
}

#[cfg(test)]
fn apply_execution_barrier() -> &'static std::sync::Mutex<Option<ApplyExecutionBarrier>> {
    static BARRIER: std::sync::OnceLock<std::sync::Mutex<Option<ApplyExecutionBarrier>>> =
        std::sync::OnceLock::new();
    BARRIER.get_or_init(|| std::sync::Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn set_apply_execution_barrier(barrier: Option<ApplyExecutionBarrier>) {
    *apply_execution_barrier()
        .lock()
        .expect("apply execution barrier lock") = barrier;
}

#[cfg(test)]
async fn wait_for_apply_execution_barrier(game_id: &str) {
    let barrier = apply_execution_barrier()
        .lock()
        .expect("apply execution barrier lock")
        .clone()
        .filter(|barrier| barrier.game_id == game_id);
    if let Some(barrier) = barrier {
        barrier.entered.wait().await;
        barrier.release.wait().await;
    }
}

pub async fn apply_collection(
    request: ApplyCollectionRequest<'_>,
) -> Result<ApplyResult, CollectionError> {
    apply_collection_with_finalization(
        request,
        ActiveCollectionFinalization::RequestDefault,
        None,
        false,
    )
    .await
}

pub async fn apply_collection_durable(
    request: ApplyCollectionRequest<'_>,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
) -> Result<ApplyResult, CollectionError> {
    apply_collection_with_finalization(
        request,
        ActiveCollectionFinalization::RequestDefault,
        Some(coordinator),
        false,
    )
    .await
}

/// Reapply a collection for a Safe Mode transition without touching enabled
/// roots that are not owned by the selected managed collection.
pub async fn apply_collection_durable_for_safe_mode(
    request: ApplyCollectionRequest<'_>,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
) -> Result<ApplyResult, CollectionError> {
    apply_collection_with_finalization(
        request,
        ActiveCollectionFinalization::RequestDefault,
        Some(coordinator),
        true,
    )
    .await
}

/// Apply the Last changes draft while restoring its original active baseline.
/// The baseline pointer and durable task completion are committed by the apply
/// pipeline in the same transaction after the filesystem mutation succeeds.
pub async fn restore_collection_with_baseline(
    request: ApplyCollectionRequest<'_>,
    active_baseline_id: Option<String>,
) -> Result<ApplyResult, CollectionError> {
    apply_collection_with_finalization(
        request,
        ActiveCollectionFinalization::Explicit(active_baseline_id),
        None,
        false,
    )
    .await
}

pub async fn restore_collection_with_baseline_durable(
    request: ApplyCollectionRequest<'_>,
    active_baseline_id: Option<String>,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
) -> Result<ApplyResult, CollectionError> {
    apply_collection_with_finalization(
        request,
        ActiveCollectionFinalization::Explicit(active_baseline_id),
        Some(coordinator),
        false,
    )
    .await
}

enum ActiveCollectionFinalization {
    RequestDefault,
    Explicit(Option<String>),
}

async fn apply_collection_with_finalization(
    request: ApplyCollectionRequest<'_>,
    finalization: ActiveCollectionFinalization,
    coordinator: Option<&crate::modules::mutation::coordinator::MutationCoordinator>,
    restrict_current_state_to_target_scope: bool,
) -> Result<ApplyResult, CollectionError> {
    let pool = request.pool;
    let final_active_collection_id = match finalization {
        ActiveCollectionFinalization::Explicit(active_baseline_id) => active_baseline_id,
        ActiveCollectionFinalization::RequestDefault if request.capture_last_changes => {
            Some(request.collection_id.to_string())
        }
        ActiveCollectionFinalization::RequestDefault => {
            crate::modules::collections::adapters::sqlite::runtime::get(
                request.pool,
                request.game_id,
            )
            .await?
            .and_then(|runtime| runtime.active_collection_id)
        }
    };
    let task_id = uuid::Uuid::new_v4().to_string();
    crate::modules::workspace::adapters::sqlite::task::create_claimed_task_with_final_active(
        request.pool,
        &task_id,
        request.game_id,
        TASK_TYPE_APPLY_COLLECTION,
        Some(request.collection_id),
        final_active_collection_id.as_deref(),
    )
    .await
    .map_err(task_error)?;

    let mut ctx = match prepare_apply_context(request, &task_id).await {
        Ok(ctx) => ctx,
        Err(error) => {
            let _ = crate::modules::workspace::adapters::sqlite::task::compare_and_set_status(
                pool,
                &task_id,
                TaskStatus::Running,
                TaskStatus::Failed,
            )
            .await;
            return Err(error);
        }
    };
    ctx.finalize_active_collection = true;
    ctx.final_active_collection_id = final_active_collection_id;
    ctx.restrict_current_state_to_target_scope = restrict_current_state_to_target_scope;
    #[cfg(test)]
    wait_for_apply_execution_barrier(&ctx.game_id).await;
    crate::modules::collections::application::apply::apply_pipeline::execute(
        &mut ctx,
        &task_id,
        TaskStatus::Running,
        true,
        coordinator,
    )
    .await
}

async fn prepare_apply_context(
    request: ApplyCollectionRequest<'_>,
    task_id: &str,
) -> Result<
    crate::modules::collections::application::apply::apply_pipeline::ApplyContext,
    CollectionError,
> {
    let capture_last_changes = request.capture_last_changes;
    let rollback_active_collection_id =
        crate::modules::collections::adapters::sqlite::runtime::get(request.pool, request.game_id)
            .await?
            .and_then(|runtime| runtime.active_collection_id);
    let captured_draft_id = if capture_last_changes {
        super::runtime::capture_last_changes_if_needed(request.pool, request.game_id).await?
    } else {
        None
    };
    let rollback_collection_id =
        captured_draft_id.or_else(|| rollback_active_collection_id.clone());
    crate::modules::workspace::adapters::sqlite::task::update_rollback_intent(
        request.pool,
        task_id,
        rollback_collection_id.as_deref(),
        rollback_active_collection_id.as_deref(),
        TaskStatus::Running,
    )
    .await
    .map_err(task_error)?;
    let mut ctx =
        crate::modules::collections::application::apply::apply_pipeline::ApplyContext::new(request);
    ctx.rollback_collection_id = rollback_collection_id;
    ctx.rollback_active_collection_id = rollback_active_collection_id;
    Ok(ctx)
}

pub(crate) async fn apply_collection_with_existing_task(
    request: ApplyCollectionRequest<'_>,
    task_id: &str,
    final_active_collection_id: Option<String>,
    coordinator: &crate::modules::mutation::coordinator::MutationCoordinator,
) -> Result<ApplyResult, CollectionError> {
    let mut ctx =
        crate::modules::collections::application::apply::apply_pipeline::ApplyContext::new(request);
    ctx.finalize_active_collection = true;
    ctx.final_active_collection_id = final_active_collection_id;
    crate::modules::collections::application::apply::apply_pipeline::execute(
        &mut ctx,
        task_id,
        TaskStatus::Running,
        false,
        Some(coordinator),
    )
    .await
}

fn task_error(error: AppError) -> CollectionError {
    match error {
        AppError::Validation(message) => CollectionError::Validation(message),
        other => CollectionError::Db(other.to_string()),
    }
}
