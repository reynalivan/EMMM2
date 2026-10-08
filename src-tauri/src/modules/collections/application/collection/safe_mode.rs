use sqlx::SqlitePool;

use super::{
    load_game_mods_path, load_live_runtime_state, load_projected_collection_state,
    require_collection,
};
use crate::modules::collections::adapters::sqlite::{self as storage, safe_mode::SafeModeIntent};
use crate::modules::collections::domain::collection::{ApplyResult, ProjectedCollectionState};
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::adapters::sqlite::task;
use crate::modules::workspace::application::projected_state;
use crate::modules::workspace::domain::task::{TaskStatus, TASK_TYPE_APPLY_COLLECTION};
use crate::shared::errors::AppError;

pub(crate) async fn prepare_safe_mode_transition(
    pool: &SqlitePool,
    game_id: &str,
    previous_enabled: bool,
) -> Result<String, AppError> {
    let mods_path = load_game_mods_path(pool, game_id).await?;
    let (mods, objects) = load_live_runtime_state(pool, game_id).await?;
    let live = projected_state::build_projected_state(&mods, &objects, mods_path.as_deref());
    let runtime = storage::runtime::get(pool, game_id).await?;
    let candidate = runtime.and_then(|state| state.active_collection_id);
    let baseline = super::valid_active_baseline(pool, game_id, candidate.as_deref()).await?;
    let previous_restore = storage::safe_mode::get_snapshot(pool, game_id).await?;
    let restore = match previous_restore.as_ref() {
        Some(state) => Some(state.clone()),
        None if previous_enabled => match baseline.as_deref() {
            Some(id) => Some(
                load_projected_collection_state(
                    pool,
                    &require_collection(pool, id).await?,
                    mods_path.as_deref(),
                )
                .await?,
            ),
            None => None,
        },
        None => None,
    };
    let target = if previous_enabled {
        restore_requested_unsafe_roots(live.clone(), restore.as_ref())
    } else {
        live.clone()
    };
    let intent = SafeModeIntent {
        previous_enabled,
        target_enabled: !previous_enabled,
        target,
        rollback: live,
        previous_restore,
        rollback_requested: false,
    };
    let id = uuid::Uuid::new_v4().to_string();
    let mut tx = pool.begin().await?;
    task::create_claimed_task_tx(
        &mut tx,
        &id,
        game_id,
        TASK_TYPE_APPLY_COLLECTION,
        baseline.as_deref(),
        baseline.as_deref(),
    )
    .await?;
    storage::safe_mode::insert_intent_tx(&mut tx, &id, &intent).await?;
    tx.commit().await?;
    Ok(id)
}

fn restore_requested_unsafe_roots(
    mut live: ProjectedCollectionState,
    restore: Option<&ProjectedCollectionState>,
) -> ProjectedCollectionState {
    let Some(restore) = restore else {
        return live;
    };
    let mut keys = live
        .active_roots
        .iter()
        .map(|root| root.root_key.clone())
        .collect::<std::collections::HashSet<_>>();
    for root in &restore.active_roots {
        if !super::is_safe_mode_eligible(root.is_safe, root.safety_source.as_deref())
            && keys.insert(root.root_key.clone())
        {
            live.active_roots.push(root.clone());
        }
    }
    live.summary.active_root_count = live.active_roots.len();
    live.summary.missing_root_count = live
        .active_roots
        .iter()
        .filter(|root| root.is_missing)
        .count();
    for object in &mut live.object_states {
        object.active_root_count = live
            .active_roots
            .iter()
            .filter(|root| root.object_id == object.object_id)
            .count();
    }
    live
}

pub(crate) async fn execute_safe_mode_transition(
    mut request: super::ApplyCollectionRequest<'_>,
    task_id: &str,
    baseline: Option<String>,
    config: &ConfigService,
    coordinator: &MutationCoordinator,
) -> Result<ApplyResult, AppError> {
    let pool = request.pool;
    let game_id = request.game_id.to_string();
    let claimed = task::get_task_by_id(pool, task_id)
        .await?
        .is_some_and(|task| task.game_id == game_id && task.status == TaskStatus::Running);
    if !claimed {
        return Err(AppError::Validation(format!(
            "Safe Mode task '{task_id}' must be claimed before execution"
        )));
    }
    let mut intent = storage::safe_mode::get_intent(pool, task_id)
        .await?
        .ok_or_else(|| AppError::Validation("Safe Mode recovery intent is missing".to_string()))?;
    let enabled = if intent.rollback_requested {
        intent.previous_enabled
    } else {
        intent.target_enabled
    };
    request
        .settings
        .safety
        .set_runtime_safe_mode(game_id.clone(), enabled);
    let target = if intent.rollback_requested {
        intent.rollback.clone()
    } else {
        intent.target.clone()
    };
    let mut ctx =
        crate::modules::collections::application::apply::apply_pipeline::ApplyContext::new(request);
    ctx.runtime_target_state = Some(target);
    ctx.finalize_active_collection = true;
    ctx.final_active_collection_id = baseline;
    ctx.defer_task_completion = true;
    ctx.restrict_current_state_to_target_scope = true;
    ctx.safe_mode_scope_path_keys = intent
        .target
        .active_roots
        .iter()
        .chain(&intent.rollback.active_roots)
        .map(|root| root.root_key.clone())
        .collect();
    let outcome = crate::modules::collections::application::apply::apply_pipeline::execute(
        &mut ctx,
        task_id,
        TaskStatus::Running,
        true,
        Some(coordinator),
    )
    .await
    .map_err(AppError::from);
    match outcome {
        Ok(result) => {
            if let Some(requested) = ctx
                .requested_target_state
                .take()
                .filter(|_| !intent.rollback_requested)
            {
                intent.target = requested;
            }
            if let Err(error) =
                complete_safe_mode_transition(pool, &game_id, task_id, &intent, config).await
            {
                release_transition(pool, task_id).await?;
                return Err(error);
            }
            Ok(result)
        }
        Err(error) => Err(error),
    }
}

pub(crate) async fn complete_safe_mode_transition(
    pool: &SqlitePool,
    game_id: &str,
    task_id: &str,
    intent: &SafeModeIntent,
    config: &ConfigService,
) -> Result<(), AppError> {
    let enabled = if intent.rollback_requested {
        intent.previous_enabled
    } else {
        intent.target_enabled
    };
    config
        .set_runtime_safe_mode(game_id, enabled)
        .map_err(|error| {
            AppError::Internal(format!(
                "Safe Mode mutation committed but runtime state sync is pending: {error}"
            ))
        })?;
    let restore = if intent.rollback_requested {
        intent.previous_restore.as_ref()
    } else {
        enabled.then_some(&intent.target)
    };
    let mut tx = pool.begin().await?;
    storage::safe_mode::set_snapshot_tx(&mut tx, game_id, restore).await?;
    let completed = task::compare_and_set_status_tx(
        &mut tx,
        task_id,
        TaskStatus::Running,
        TaskStatus::Completed,
    )
    .await?;
    if !completed {
        return Err(AppError::Validation(format!(
            "Safe Mode task '{task_id}' lost its claim"
        )));
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn release_transition(pool: &SqlitePool, task_id: &str) -> Result<(), AppError> {
    if !task::compare_and_set_status(pool, task_id, TaskStatus::Running, TaskStatus::Pending)
        .await?
    {
        return Err(AppError::Validation(format!(
            "Safe Mode task '{task_id}' could not be released"
        )));
    }
    Ok(())
}

pub(crate) async fn release_safe_mode_task_if_running(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<(), AppError> {
    let task = task::get_task_by_id(pool, task_id)
        .await?
        .ok_or_else(|| AppError::Validation(format!("Safe Mode task '{task_id}' is missing")))?;
    if task.status == TaskStatus::Running {
        release_transition(pool, task_id).await?;
    }
    Ok(())
}

pub(crate) async fn safe_mode_preflight_scope_paths(
    pool: &SqlitePool,
    task_id: &str,
    mods_path: &std::path::Path,
) -> Result<Option<Vec<String>>, AppError> {
    let Some(intent) = storage::safe_mode::get_intent(pool, task_id).await? else {
        return Ok(None);
    };
    let paths = [&intent.target, &intent.rollback]
        .into_iter()
        .flat_map(|state| {
            state
                .active_roots
                .iter()
                .map(|root| root.source_path.as_str())
                .chain(
                    state
                        .object_states
                        .iter()
                        .map(|object| object.path_key.as_str()),
                )
        })
        .map(|path| {
            let path = std::path::Path::new(path);
            let absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                mods_path.join(path)
            };
            absolute.to_string_lossy().to_string()
        })
        .collect();
    Ok(Some(paths))
}

#[cfg(test)]
#[path = "tests/safe_mode_tests.rs"]
mod tests;
