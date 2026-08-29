use tauri::State;

use crate::domain::errors::AppError;

use crate::domain::objects::{
    CategoryCount, CreateObjectInput, GetObjectsResult, ObjectFilter, UpdateObjectInput,
};

async fn absolute_object_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    folder_path: &str,
) -> Result<String, AppError> {
    let mods_path = crate::repo::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    Ok(std::path::Path::new(&mods_path)
        .join(folder_path)
        .to_string_lossy()
        .to_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct CreateObjectResult {
    pub id: String,
    pub sync_warning: Option<crate::services::disk_reconcile::types::CommittedMutationSyncWarning>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_objects_cmd(
    filter: ObjectFilter,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<GetObjectsResult, AppError> {
    get_objects_cmd_inner(filter, &pool).await
}

pub async fn get_objects_cmd_inner(
    filter: ObjectFilter,
    pool: &sqlx::SqlitePool,
) -> Result<GetObjectsResult, AppError> {
    let objects =
        crate::services::objects::query::get_filtered_objects_with_conflict_check(pool, &filter)
            .await?;

    Ok(objects)
}

#[tauri::command]
#[specta::specta]
pub async fn get_category_counts_cmd(
    game_id: String,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<Vec<CategoryCount>, AppError> {
    let counts = crate::services::objects::query::get_category_counts_service(&pool, &game_id)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?;

    Ok(counts)
}

#[tauri::command]
#[specta::specta]
pub async fn create_object_cmd(
    input: CreateObjectInput,
    pool: State<'_, sqlx::SqlitePool>,
    app: tauri::AppHandle,
    watcher: State<'_, crate::services::scanner::watcher::WatcherState>,
    op_lock: State<'_, crate::services::fs_utils::operation_lock::OperationLock>,
) -> Result<CreateObjectResult, AppError> {
    let game_id = input.game_id.clone();
    let folder_path = input.folder_path.as_deref().unwrap_or(&input.name);
    let preflight_paths = [absolute_object_path(pool.inner(), &game_id, folder_path).await?];
    crate::services::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let lock = op_lock.acquire().await?;
    let guard = crate::services::scanner::watcher::SuppressionGuard::new(&watcher.suppressor);
    let result =
        crate::services::objects::mutate::create_object_cmd_inner(&pool, Some(&app), input).await;
    drop(guard);
    drop(lock);
    let reconcile = crate::services::disk_reconcile::emit::run_full_internal_disk_reconcile(
        &app,
        pool.inner(),
        &game_id,
    )
    .await;
    match result {
        Ok(id) => {
            let settlement =
                crate::services::disk_reconcile::emit::settle_committed_reconcile(reconcile);
            Ok(CreateObjectResult {
                id,
                sync_warning: settlement.sync_warning,
            })
        }
        Err(error) => match reconcile {
            Ok(_) => Err(error),
            Err(reconcile_error) => Err(AppError::Io(format!(
                "{error}; convergence also failed: {reconcile_error}"
            ))),
        },
    }
}

#[tauri::command]
#[specta::specta]
pub async fn update_object_cmd(
    id: String,
    updates: UpdateObjectInput,
    pool: State<'_, sqlx::SqlitePool>,
    app: tauri::AppHandle,
    disk_reconcile_state: State<
        '_,
        crate::services::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: State<'_, crate::services::fs_utils::operation_lock::OperationLock>,
) -> Result<(), AppError> {
    let (game_id, folder_path) =
        crate::repo::object::get_game_id_and_folder_path(pool.inner(), &id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {id}")))?;
    let folder_path =
        folder_path.ok_or_else(|| AppError::NotFound(format!("Object folder not found: {id}")))?;
    let preflight_paths = [absolute_object_path(pool.inner(), &game_id, &folder_path).await?];
    crate::services::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let touches_aliases = updates.custom_skins.is_some();
    let game_lock = disk_reconcile_state.game_lock(&game_id);
    let game_guard = game_lock.lock().await;
    let operation_guard = op_lock.acquire_for_reconcile().await;
    crate::services::objects::mutate::update_object(&pool, &id, &updates).await?;
    drop(operation_guard);
    drop(game_guard);

    // The MasterDB is cached parsed, with user aliases already folded in, so an
    // edited alias would otherwise not reach the matcher until a restart.
    if touches_aliases {
        crate::services::scanner::master_db::MasterDbCache::invalidate(&app).await;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_object_cmd(
    id: String,
    force: bool,
    app: tauri::AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
    state: State<'_, crate::services::scanner::watcher::WatcherState>,
    op_lock: State<'_, crate::services::fs_utils::operation_lock::OperationLock>,
) -> Result<crate::services::disk_reconcile::types::CommittedMutationResult, AppError> {
    let (game_id, folder_path) =
        crate::repo::object::get_game_id_and_folder_path(pool.inner(), &id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {id}")))?;
    let folder_path =
        folder_path.ok_or_else(|| AppError::NotFound(format!("Object folder not found: {id}")))?;
    let preflight_paths = [absolute_object_path(pool.inner(), &game_id, &folder_path).await?];
    crate::services::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let op_guard = op_lock.acquire().await?;
    crate::services::objects::mutate::delete_object(&pool, &id, force, &state, &op_guard).await?;
    drop(op_guard);
    let settlement = crate::services::disk_reconcile::emit::settle_committed_reconcile(
        crate::services::disk_reconcile::emit::run_full_internal_disk_reconcile(
            &app,
            pool.inner(),
            &game_id,
        )
        .await,
    );
    Ok(
        crate::services::disk_reconcile::types::CommittedMutationResult {
            sync_warning: settlement.sync_warning,
        },
    )
}

#[cfg(test)]
#[path = "tests/object_cmds_tests.rs"]
mod tests;
