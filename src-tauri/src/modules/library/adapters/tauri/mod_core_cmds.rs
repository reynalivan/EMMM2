use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::validate_path;
use crate::shared::errors::AppError;
use std::path::{Component, Path, PathBuf};
use tauri::State;
use uuid::Uuid;

pub use crate::modules::library::application::mods::core_ops::{standardize_prefix, RenameResult};

#[derive(Debug)]
struct PreparedFolderCreate {
    stage: PathBuf,
    target: PathBuf,
}

impl PreparedFolderCreate {
    fn prepare(&self) -> Result<(), AppError> {
        std::fs::create_dir(&self.stage).map_err(AppError::from)
    }

    fn journal_step(&self) -> crate::modules::mutation::api::PlannedStep {
        crate::modules::mutation::api::PlannedStep::rename(
            0,
            self.stage.clone(),
            self.target.clone(),
        )
    }

    fn promote(&self) -> Result<(), AppError> {
        std::fs::rename(&self.stage, &self.target).map_err(AppError::from)
    }

    fn rollback(&self) -> Result<(), AppError> {
        if self.target.exists() && !self.stage.exists() {
            std::fs::rename(&self.target, &self.stage)?;
        }
        if self.stage.exists() {
            std::fs::remove_dir(&self.stage)?;
        }
        Ok(())
    }
}

fn prepare_folder_create(
    parent: &Path,
    folder_name: &str,
) -> Result<PreparedFolderCreate, AppError> {
    crate::modules::library::application::mods::core_ops::validate_folder_name_component(
        folder_name,
    )?;
    if !parent.is_dir() {
        return Err(AppError::Validation(format!(
            "Folder parent is not a directory: {}",
            parent.display()
        )));
    }

    let duplicate =
        std::fs::read_dir(parent)?.try_fold(false, |found, entry| -> Result<bool, AppError> {
            if found {
                return Ok(true);
            }
            let entry = entry?;
            Ok(entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(folder_name))
        })?;
    if duplicate {
        return Err(AppError::Validation(format!(
            "A folder named '{folder_name}' already exists in this location"
        )));
    }

    let target = parent.join(folder_name);
    let stage = parent.join(format!(".emmm-folder-create-{}", Uuid::new_v4().simple()));
    Ok(PreparedFolderCreate { stage, target })
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the IPC payload explicit.
pub async fn create_mod_folder(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: State<'_, MutationCoordinator>,
    parent_path: String,
    folder_name: String,
    game_id: String,
) -> Result<String, AppError> {
    let parent = validate_path(&config, &game_id, &parent_path)?;
    if !parent.is_dir() {
        return Err(AppError::Validation(
            "Folder parent is not a directory".to_string(),
        ));
    }
    let preflight_paths = [parent.to_string_lossy().to_string()];
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;

    let game_guard = disk_reconcile_state.game_lock(&game_id).lock_owned().await;
    let prepared = prepare_folder_create(&parent, &folder_name)?;
    prepared.prepare()?;
    let operation_guard = match op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "folder-create",
            &game_id,
            vec![prepared.journal_step()],
        ))
        .await
    {
        Ok(guard) => guard,
        Err(error) => {
            prepared.rollback()?;
            return Err(error);
        }
    };
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let watcher_guard =
        crate::modules::workspace::application::scanner::watcher::SuppressionGuard::new(
            &watcher.suppressor,
        );
    if let Err(error) = prepared.promote() {
        mutation_lease.begin_rollback()?;
        prepared.rollback()?;
        mutation_lease.mark_step_rolled_back(0)?;
        mutation_lease.finish_rollback()?;
        return Err(error);
    }
    mutation_lease.mark_step_applied(0)?;
    drop(watcher_guard);

    let changed_paths = vec![prepared.target.to_string_lossy().to_string()];
    let reconcile = crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile_with_path_hints_under_lease(
        &app,
        pool.inner(),
        &game_id,
        changed_paths.clone(),
        Vec::new(),
        &mutation_lease,
    )
    .await;
    let reconcile_error = match reconcile {
        Ok(result) if result.status.applied() => {
            mutation_lease.mark_db_committed()?;
            mutation_lease.commit()?;
            return Ok(prepared.target.to_string_lossy().to_string());
        }
        Ok(result) => AppError::Io(format!(
            "Folder create reconcile requires attention: {:?}",
            result.status
        )),
        Err(error) => error,
    };
    mutation_lease.begin_rollback()?;
    prepared.rollback()?;
    mutation_lease.mark_step_rolled_back(0)?;
    if let Err(rollback_reconcile_error) = crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile_with_path_hints_under_lease(
        &app,
        pool.inner(),
        &game_id,
        changed_paths,
        Vec::new(),
        &mutation_lease,
    )
    .await
    {
        let combined = format!(
            "{reconcile_error}; folder rollback reconcile failed: {rollback_reconcile_error}"
        );
        mutation_lease.fail(combined.clone())?;
        return Err(AppError::Io(combined));
    }
    mutation_lease.finish_rollback()?;
    Err(reconcile_error)
}

#[specta::specta]
#[tauri::command]
pub async fn open_in_explorer(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
    path: String,
) -> Result<(), AppError> {
    let canonical_path = validate_path(&config, &game_id, &path)?;
    ensure_path_can_be_opened(&app, pool.inner(), &game_id, &canonical_path).await?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&*canonical_path)
            .spawn()
            .map_err(|e| AppError::Io(format!("Failed to open explorer: {}", e)))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    Err(AppError::Io(
        "Open in explorer only supported on Windows".to_string(),
    ))
}

#[specta::specta]
#[tauri::command]
pub async fn open_ini_in_editor(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
    folder_path: String,
    file_name: String,
) -> Result<(), AppError> {
    let canonical_folder = validate_path(&config, &game_id, &folder_path)?;
    let canonical_ini = resolve_ini_editor_path(&canonical_folder, &file_name)?;
    ensure_path_can_be_opened(&app, pool.inner(), &game_id, &canonical_ini).await?;
    crate::platform::process::open_path(&canonical_ini)
}

fn resolve_ini_editor_path(folder: &Path, file_name: &str) -> Result<PathBuf, AppError> {
    if file_name.trim().is_empty() {
        return Err(AppError::Validation(
            "INI editor path must be a relative file path".to_string(),
        ));
    }

    let requested = Path::new(file_name);
    if requested.is_absolute()
        || requested
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AppError::Validation(
            "INI editor path must be a relative file path".to_string(),
        ));
    }
    if !requested
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"))
    {
        return Err(AppError::Validation(
            "Only .ini files can be opened from the INI editor".to_string(),
        ));
    }

    let canonical_folder = folder.canonicalize()?;
    let canonical_ini = canonical_folder.join(requested).canonicalize()?;
    if !canonical_ini.is_file() {
        return Err(AppError::Validation(format!(
            "INI editor target is not a file: {}",
            canonical_ini.display()
        )));
    }

    let folder_key = crate::shared::path_key::canonical_path_key_for_path(&canonical_folder);
    let ini_key = crate::shared::path_key::canonical_path_key_for_path(&canonical_ini);
    let child_prefix = format!("{folder_key}/");
    if !ini_key.starts_with(&child_prefix) {
        return Err(AppError::Security(
            "INI editor target escaped the selected mod folder".to_string(),
        ));
    }

    Ok(canonical_ini)
}

#[specta::specta]
#[tauri::command]
pub async fn reveal_object_in_explorer(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
    object_name: String,
) -> Result<String, AppError> {
    let mods_path =
        crate::modules::games::adapters::sqlite::game::get_mod_path(pool.inner(), &game_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;

    if let Some(path_str) =
        resolve_and_heal_db_path(pool.inner(), &object_id, Path::new(&mods_path)).await
    {
        let canonical = validate_path(&config, &game_id, &path_str)?;
        ensure_path_can_be_opened(&app, pool.inner(), &game_id, &canonical).await?;
        return open_explorer_select(&canonical.to_string_lossy());
    }

    let candidate_path = find_fallback_path(&mods_path, &object_name)?;
    let canonical = validate_path(&config, &game_id, &candidate_path)?;
    ensure_path_can_be_opened(&app, pool.inner(), &game_id, &canonical).await?;
    open_explorer_select(&canonical.to_string_lossy())
}

async fn ensure_path_can_be_opened(
    app: &tauri::AppHandle,
    _pool: &sqlx::SqlitePool,
    game_id: &str,
    path: &Path,
) -> Result<(), AppError> {
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_open_path_preflight(
        app, game_id, path,
    )
    .await
}

async fn resolve_and_heal_db_path(
    pool: &sqlx::SqlitePool,
    object_id: &str,
    mods_root: &Path,
) -> Option<String> {
    crate::modules::library::application::mods::stale_mod_service::resolve_mod_path_for_object(
        pool, object_id, mods_root,
    )
    .await
}

fn find_fallback_path(mods_path: &str, object_name: &str) -> Result<String, AppError> {
    let mods_dir = Path::new(mods_path);
    if !mods_dir.exists() || !mods_dir.is_dir() {
        return Err(AppError::Io(
            "Could not find any folder to reveal".to_string(),
        ));
    }

    let candidate = mods_dir.join(object_name);
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().to_string());
    }

    let disabled_candidate = mods_dir.join(standardize_prefix(object_name, false));
    if disabled_candidate.exists() {
        return Ok(disabled_candidate.to_string_lossy().to_string());
    }

    Ok(mods_path.to_string())
}

fn open_explorer_select(path: &str) -> Result<String, AppError> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", path])
            .spawn()
            .map_err(|e| AppError::Io(format!("Failed to open explorer: {}", e)))?;
        Ok(path.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    Err(AppError::Io(
        "Reveal in explorer only supported on Windows".to_string(),
    ))
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn rename_mod_folder(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: State<'_, WatcherState>,
    disk_reconcile_state: State<
        '_,
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState,
    >,
    op_lock: State<'_, MutationCoordinator>,
    folder_path: String,
    new_name: String,
    game_id: String,
) -> Result<RenameResult, AppError> {
    let folder = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [folder.to_string_lossy().to_string()];
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let source_name = folder
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::Validation("Mod folder has no valid UTF-8 name".to_string()))?;
    let target_name =
        standardize_prefix(&new_name, !source_name.starts_with(crate::DISABLED_PREFIX));
    let target = folder.with_file_name(target_name);
    let game_guard = disk_reconcile_state.game_lock(&game_id).lock_owned().await;
    let operation_guard = op_lock
        .acquire_operation(crate::modules::mutation::api::OperationPlan::new(
            "rename-mod-folder",
            game_id.clone(),
            vec![crate::modules::mutation::api::PlannedStep::rename(
                0,
                folder.to_path_buf(),
                target.clone(),
            )],
        ))
        .await?;
    let mutation_lease = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease::from_durable_guard(
        game_guard,
        operation_guard,
    );
    let mut result =
        crate::modules::library::application::mods::core_ops::rename_mod_folder_inner_service(
            &config,
            pool.inner(),
            &state,
            mutation_lease.operation_guard(),
            &folder,
            new_name.clone(),
            &game_id,
        )
        .await?;
    mutation_lease.mark_step_applied(0)?;

    let reconcile =
        crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile_with_path_hints_under_lease(
            &app,
            pool.inner(),
            &game_id,
            vec![result.old_path.clone(), result.new_path.clone()],
            Vec::new(),
            &mutation_lease,
        )
        .await?;
    if !reconcile.status.applied() {
        return Err(AppError::Io(format!(
            "Rename reconcile requires attention: {:?}",
            reconcile.status
        )));
    }
    result
        .collection_impact
        .merge(reconcile.collection_reference_impact);
    mutation_lease.mark_db_committed()?;
    mutation_lease.commit()?;
    result.sync_warning = None;

    Ok(result)
}

#[cfg(test)]
#[path = "tests/mod_core_cmds_tests.rs"]
mod tests;
