
// --- From commands/mods/mod_meta_cmds.rs ---
use crate::shared::errors::AppError;
use crate::modules::system::application::config::ConfigService;
use crate::platform::fs::guard::validate_path;
use crate::platform::fs::operation_lock::OperationLock;
use crate::modules::library::application::mods::{info_json, metadata};
use crate::modules::workspace::application::scanner::watcher::WatcherState;

async fn object_absolute_path(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<String, AppError> {
    let object = crate::modules::catalog::adapters::outbound::sqlite::object::get_game_object_by_id(pool, object_id)
        .await?
        .filter(|object| object.game_id == game_id)
        .ok_or_else(|| AppError::NotFound(format!("Object not found: {object_id}")))?;
    let mods_path = crate::modules::games::adapters::outbound::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game mods path not found".to_string()))?;
    Ok(std::path::Path::new(&mods_path)
        .join(object.folder_path)
        .to_string_lossy()
        .to_string())
}

fn restore_info_json(path: &std::path::Path, previous: Option<&[u8]>) -> Result<(), AppError> {
    match previous {
        Some(bytes) => crate::platform::fs::atomic_file::atomic_write(path, bytes),
        None if path.exists() => std::fs::remove_file(path).map_err(AppError::from),
        None => Ok(()),
    }
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the mutation payload.
pub async fn toggle_mod_safe(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    op_lock: tauri::State<'_, OperationLock>,
    game_id: String,
    folder_path: String,
    safe: bool,
) -> Result<crate::modules::workspace::application::disk_reconcile::types::CommittedMutationResult, AppError> {
    let folder = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [folder.to_string_lossy().to_string()];
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let lock = op_lock.acquire().await?;
    let suppression = watcher.suppressor.suppress_paths([folder.as_ref()]);
    metadata::toggle_mod_safe(pool.inner(), &game_id, &folder, safe).await?;
    drop(suppression);
    drop(lock);
    let settlement = crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(
        crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile(
            &app,
            pool.inner(),
            &game_id,
            vec![folder.join("info.json").to_string_lossy().to_string()],
        )
        .await,
    );
    Ok(
        crate::modules::workspace::application::disk_reconcile::types::CommittedMutationResult {
            sync_warning: settlement.sync_warning,
        },
    )
}

#[specta::specta]
#[tauri::command]
pub async fn suggest_random_mods(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<metadata::RandomModProposal>, AppError> {
    metadata::suggest_random_mods(pool.inner(), &game_id).await
}

#[specta::specta]
#[tauri::command]
pub async fn get_active_mod_conflicts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::modules::workspace::application::scanner::conflict::ConflictInfo>, AppError> {
    metadata::get_active_mod_conflicts(pool.inner(), &game_id).await
}

#[specta::specta]
#[tauri::command]
pub async fn read_mod_info(
    config: tauri::State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Option<info_json::ModInfo>, AppError> {
    let path = validate_path(&config, &game_id, &folder_path)?;
    Ok(info_json::read_info_json(&path)?)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the mutation payload.
pub async fn update_mod_info(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: tauri::State<'_, OperationLock>,
    game_id: String,
    folder_path: String,
    update: info_json::ModInfoUpdate,
) -> Result<info_json::ModInfo, AppError> {
    if update.is_safe.is_some() {
        return Err(AppError::Validation(
            "Safety changes must use toggle_mod_safe".to_string(),
        ));
    }
    let path = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [path.to_string_lossy().to_string()];
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let info_path = path.join("info.json");
    let changed_path = info_path.to_string_lossy().to_string();
    let lock = op_lock.acquire().await?;
    let guard = state.suppressor.suppress_paths([path.as_ref()]);
    let previous = match std::fs::read(&info_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let info = info_json::update_info_json(&path, &update)?;
    drop(guard);
    drop(lock);
    let reconcile = crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile(
        &app,
        pool.inner(),
        &game_id,
        vec![changed_path],
    )
    .await;
    let failure = match reconcile {
        Ok(result) if result.status.applied() => None,
        Ok(result) => Some(AppError::Io(result.error_message.unwrap_or_else(|| {
            format!(
                "Metadata reconcile was blocked with status {:?}",
                result.status
            )
        }))),
        Err(error) => Some(error),
    };
    if let Some(failure) = failure {
        let rollback_lock = op_lock.acquire().await?;
        let rollback_guard = state.suppressor.suppress_paths([path.as_ref()]);
        let rollback = restore_info_json(&info_path, previous.as_deref());
        drop(rollback_guard);
        drop(rollback_lock);
        let repair = crate::modules::workspace::application::disk_reconcile::emit::run_full_internal_disk_reconcile(
            &app,
            pool.inner(),
            &game_id,
        )
        .await;
        return match (rollback, repair) {
            (Ok(()), Ok(_)) => Err(failure),
            (rollback, repair) => Err(AppError::Io(format!(
                "{failure}; rollback result: {}; recovery reconcile result: {}",
                rollback
                    .err()
                    .map_or_else(|| "ok".to_string(), |error| error.to_string()),
                repair
                    .err()
                    .map_or_else(|| "ok".to_string(), |error| error.to_string())
            ))),
        };
    }

    Ok(info)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the mutation payload.
pub async fn set_mod_category(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: tauri::State<
        '_,
        crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: tauri::State<'_, OperationLock>,
    game_id: String,
    folder_path: String,
    category: String,
) -> Result<(), AppError> {
    let folder = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [folder.to_string_lossy().to_string()];
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let game_lock = disk_reconcile_state.game_lock(&game_id);
    let game_guard = game_lock.lock().await;
    let operation_guard = op_lock.acquire_for_reconcile().await;
    metadata::set_mod_category(&pool, &game_id, &folder, &category).await?;
    drop(operation_guard);
    drop(game_guard);
    let runtime_effects = crate::modules::system::application::app::runtime_effects::settle_committed_runtime_effects(
        disk_reconcile_state.inner(),
        crate::modules::system::application::app::runtime_effects::RuntimeSideEffects {
            pool: &pool,
            config: &config,
            game_id: &game_id,
            collections_dirty: false,
            overlay_refresh: true,
        },
    )
    .await;
    if let Some(warning) = runtime_effects.warning {
        log::warn!("Metadata category update committed with pending runtime effects: {warning}");
    }

    Ok(())
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the mutation payload.
pub async fn set_object_mods_category(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    disk_reconcile_state: tauri::State<
        '_,
        crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: tauri::State<'_, OperationLock>,
    game_id: String,
    object_id: String,
    category: String,
) -> Result<usize, AppError> {
    let preflight_paths = [object_absolute_path(pool.inner(), &game_id, &object_id).await?];
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let game_lock = disk_reconcile_state.game_lock(&game_id);
    let game_guard = game_lock.lock().await;
    let operation_guard = op_lock.acquire_for_reconcile().await;
    let updated = crate::modules::catalog::application::objects::mutate::set_object_and_mods_category(
        pool.inner(),
        &game_id,
        &object_id,
        &category,
    )
    .await?;
    drop(operation_guard);
    drop(game_guard);

    let runtime_effects = crate::modules::system::application::app::runtime_effects::settle_committed_runtime_effects(
        disk_reconcile_state.inner(),
        crate::modules::system::application::app::runtime_effects::RuntimeSideEffects {
            pool: pool.inner(),
            config: &config,
            game_id: &game_id,
            collections_dirty: true,
            overlay_refresh: true,
        },
    )
    .await;
    if let Some(warning) = runtime_effects.warning {
        log::warn!("Object metadata update committed with pending runtime effects: {warning}");
    }

    Ok(updated)
}

#[derive(serde::Deserialize, specta::Type)]
pub struct MoveModsToObjectInput {
    pub game_id: String,
    pub folder_paths: Vec<String>,
    pub target_object_id: String,
    pub target_subpath: Option<String>,
    pub status: Option<String>,
}

#[specta::specta]
#[tauri::command]
pub async fn list_move_targets_for_object(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
) -> Result<Vec<crate::modules::library::application::mods::organizer_ext::WorkspaceMoveTarget>, AppError> {
    crate::modules::library::application::mods::organizer_ext::list_move_targets_for_object_service(
        pool.inner(),
        &game_id,
        &object_id,
    )
    .await
}

#[specta::specta]
#[tauri::command]
pub async fn move_mods_to_object(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    op_lock: tauri::State<'_, OperationLock>,
    disk_reconcile_state: tauri::State<
        '_,
        crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    watcher: tauri::State<'_, WatcherState>,
    input: MoveModsToObjectInput,
) -> Result<crate::modules::library::application::mods::bulk::BulkResult, AppError> {
    let folders = crate::platform::fs::guard::validate_paths(
        &config,
        &input.game_id,
        &input.folder_paths,
    )?;
    let mut preflight_paths = folders
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    preflight_paths
        .push(object_absolute_path(pool.inner(), &input.game_id, &input.target_object_id).await?);
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &input.game_id,
        Some(&preflight_paths),
    )
    .await?;
    let mutation_lease = disk_reconcile_state
        .acquire_mutation_lease(&input.game_id, op_lock.inner())
        .await?;
    let organizer = crate::modules::library::application::mods::organizer_ext::move_mods_to_object_service(
        pool.inner(),
        mutation_lease.operation_guard(),
        &watcher,
        crate::modules::library::application::mods::organizer_ext::MoveModsToObjectParams {
            game_id: &input.game_id,
            folder_paths: &folders,
            target_object_id: &input.target_object_id,
            target_subpath: input.target_subpath.as_deref(),
            status: input.status.as_deref(),
        },
    )
    .await?;
    let mut result = organizer.result;

    // Convergence: reconcile source and destination roots after the move.
    // The target root is included explicitly: a partial failure can leave a
    // folder already renamed under the target while its path is absent from
    // `success`, and reconciling only the sources would prune its row.
    let mut changed_paths = input.folder_paths.clone();
    changed_paths.extend(result.success.iter().cloned());
    if let Some(target_obj) =
        crate::modules::catalog::adapters::outbound::sqlite::object::get_game_object_by_id(pool.inner(), &input.target_object_id)
            .await?
    {
        if let Some(mods_path) =
            crate::modules::games::adapters::outbound::sqlite::game::get_mod_path(pool.inner(), &input.game_id).await?
        {
            changed_paths.push(
                std::path::Path::new(&mods_path)
                    .join(&target_obj.folder_path)
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    // Quiet: the move's caller publishes its own refresh from the result.
    let settlement = crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(
        crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile_with_path_hints_under_lease(
            &app,
            pool.inner(),
            &input.game_id,
            changed_paths,
            organizer.path_hints,
            &mutation_lease,
        )
        .await,
    );
    drop(mutation_lease);
    if let Some(reconcile) = settlement.reconcile {
        result
            .collection_impact
            .merge(reconcile.collection_reference_impact);
        for update in reconcile.path_updates {
            if !result.path_rewrites.iter().any(|rewrite| {
                rewrite.old_path.eq_ignore_ascii_case(&update.from)
                    && rewrite.new_path.eq_ignore_ascii_case(&update.to)
            }) {
                result
                    .path_rewrites
                    .push(crate::modules::workspace::domain::workspace::WorkspacePathRewrite {
                        old_path: update.from,
                        new_path: update.to,
                    });
            }
        }
    }
    result.sync_warning = settlement.sync_warning;

    Ok(result)
}

#[cfg(test)]
#[path = "tests/mod_meta_cmds_tests.rs"]
mod tests;


// --- From commands/scanner/conflict_cmds.rs ---
//! Epic 5: Advanced Mod Management Commands
//!
//! Commands that require DB access for "Enable Only This" and conflict checks.
//! Separated from mod_cmds.rs to keep file sizes manageable.

use crate::shared::errors::AppError;
use crate::modules::workspace::application::scanner::conflict::ConflictInfo;
use std::path::PathBuf;

/// Detect shader/buffer hash conflicts across INI files.
///
/// # Covers: US-2.Z, TC-2.4-01
#[specta::specta]
#[tauri::command]
pub async fn detect_conflicts_cmd(ini_paths: Vec<String>) -> Result<Vec<ConflictInfo>, AppError> {
    let paths: Vec<(PathBuf, PathBuf)> = ini_paths
        .into_iter()
        .map(|p| {
            let pb = PathBuf::from(p);
            let root = pb
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| pb.clone());
            (root, pb)
        })
        .collect();
    Ok(crate::modules::workspace::application::scanner::conflict::detect_conflicts(&paths))
}

/// Detect conflicts by scanning the entire mods folder for INI files.
///
/// More efficient for frontend usage as it avoids passing thousands of paths.
/// # Covers: US-2.Z
#[specta::specta]
#[tauri::command]
pub async fn detect_conflicts_in_folder_cmd(
    mods_path: String,
    config: tauri::State<'_, crate::modules::system::application::config::ConfigService>,
) -> Result<Vec<ConflictInfo>, AppError> {
    let path =
        crate::platform::fs::guard::validate_dir_in_configured_roots(&config, &mods_path)?;
    Ok(crate::modules::workspace::application::scanner::conflict::detect::detect_conflicts_in_folder_service(&path)?)
}

#[cfg(test)]
#[path = "tests/conflict_cmds_tests.rs"]
mod tests;

