use tauri::State;

use crate::shared::errors::AppError;

use crate::modules::catalog::domain::objects::{
    CategoryCount, CreateObjectInput, GetObjectsResult, ObjectFilter, UpdateObjectInput,
};

async fn absolute_object_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    folder_path: &str,
) -> Result<String, AppError> {
    let mods_path = crate::modules::games::adapters::outbound::sqlite::game::get_mod_path(pool, game_id)
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
    pub sync_warning: Option<crate::modules::workspace::application::disk_reconcile::types::CommittedMutationSyncWarning>,
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
        crate::modules::catalog::application::objects::query::get_filtered_objects_with_conflict_check(pool, &filter)
            .await?;

    Ok(objects)
}

#[tauri::command]
#[specta::specta]
pub async fn get_category_counts_cmd(
    game_id: String,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<Vec<CategoryCount>, AppError> {
    let counts = crate::modules::catalog::application::objects::query::get_category_counts_service(&pool, &game_id)
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
    watcher: State<'_, crate::modules::workspace::application::scanner::watcher::WatcherState>,
    op_lock: State<'_, crate::platform::fs::operation_lock::OperationLock>,
) -> Result<CreateObjectResult, AppError> {
    let game_id = input.game_id.clone();
    let folder_path = input.folder_path.as_deref().unwrap_or(&input.name);
    let preflight_paths = [absolute_object_path(pool.inner(), &game_id, folder_path).await?];
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let lock = op_lock.acquire().await?;
    let guard = crate::modules::workspace::application::scanner::watcher::SuppressionGuard::new(&watcher.suppressor);
    let result =
        crate::modules::catalog::application::objects::mutate::create_object_cmd_inner(&pool, Some(&app), input).await;
    drop(guard);
    drop(lock);
    let reconcile = crate::modules::workspace::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
        &app,
        pool.inner(),
        &game_id,
    )
    .await;
    match result {
        Ok(id) => {
            let settlement =
                crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(reconcile);
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
        crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: State<'_, crate::platform::fs::operation_lock::OperationLock>,
) -> Result<(), AppError> {
    let (game_id, folder_path) =
        crate::modules::catalog::adapters::outbound::sqlite::object::get_game_id_and_folder_path(pool.inner(), &id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {id}")))?;
    let folder_path =
        folder_path.ok_or_else(|| AppError::NotFound(format!("Object folder not found: {id}")))?;
    let preflight_paths = [absolute_object_path(pool.inner(), &game_id, &folder_path).await?];
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
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
    crate::modules::catalog::application::objects::mutate::update_object(&pool, &id, &updates).await?;
    drop(operation_guard);
    drop(game_guard);

    // The MasterDB is cached parsed, with user aliases already folded in, so an
    // edited alias would otherwise not reach the matcher until a restart.
    if touches_aliases {
        crate::modules::workspace::application::scanner::master_db::MasterDbCache::invalidate(&app).await;
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
    state: State<'_, crate::modules::workspace::application::scanner::watcher::WatcherState>,
    op_lock: State<'_, crate::platform::fs::operation_lock::OperationLock>,
) -> Result<crate::modules::workspace::application::disk_reconcile::types::CommittedMutationResult, AppError> {
    let (game_id, folder_path) =
        crate::modules::catalog::adapters::outbound::sqlite::object::get_game_id_and_folder_path(pool.inner(), &id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Object not found: {id}")))?;
    let folder_path =
        folder_path.ok_or_else(|| AppError::NotFound(format!("Object folder not found: {id}")))?;
    let preflight_paths = [absolute_object_path(pool.inner(), &game_id, &folder_path).await?];
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let op_guard = op_lock.acquire().await?;
    crate::modules::catalog::application::objects::mutate::delete_object(&pool, &id, force, &state, &op_guard).await?;
    drop(op_guard);
    let settlement = crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(
        crate::modules::workspace::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
            &app,
            pool.inner(),
            &game_id,
        )
        .await,
    );
    Ok(
        crate::modules::workspace::application::disk_reconcile::types::CommittedMutationResult {
            sync_warning: settlement.sync_warning,
        },
    )
}

#[cfg(test)]
#[path = "tests/object_cmds_tests.rs"]
mod tests;
