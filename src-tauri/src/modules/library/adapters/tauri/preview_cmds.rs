use crate::modules::library::application::ini::document::IniDocument;
use crate::modules::library::application::mods::preview_ops::{
    clear_mod_preview_images_inner, ensure_image_size, list_mod_ini_files_inner,
    list_mod_preview_images_inner, read_mod_ini_inner, remove_mod_preview_image_inner,
    resolve_image_path, save_mod_preview_image_inner, write_mod_ini_locked_inner,
};
use crate::modules::mutation::coordinator::MutationCoordinator;
use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::validate_path;
use crate::shared::errors::AppError;
use tauri::State;

pub use crate::modules::library::application::mods::preview_ops::{IniFileEntry, IniLineUpdate};

#[specta::specta]
#[tauri::command]
pub async fn list_mod_ini_files(
    config: State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Vec<IniFileEntry>, AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)?;
    list_mod_ini_files_inner(&mod_root)
}

#[specta::specta]
#[tauri::command]
pub async fn read_mod_ini(
    config: State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
    file_name: String,
) -> Result<IniDocument, AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)?;
    read_mod_ini_inner(&mod_root, &file_name)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn write_mod_ini(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    file_name: String,
    expected_source_hash: String,
    line_updates: Vec<IniLineUpdate>,
) -> Result<
    crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationResult,
    AppError,
> {
    let mod_root = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [mod_root.to_string_lossy().to_string()];
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let changed_path = mod_root.join(&file_name).to_string_lossy().to_string();
    let op_guard = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::PreviewFile)
        .await?;
    let guard = watcher.suppressor.suppress_paths([mod_root.as_ref()]);
    write_mod_ini_locked_inner(
        op_guard.op_guard(),
        &mod_root,
        &file_name,
        &expected_source_hash,
        line_updates,
    )
    .await?;
    drop(op_guard);
    drop(guard);
    let settlement = crate::modules::reconciliation::application::disk_reconcile::emit::settle_committed_reconcile(
        crate::modules::reconciliation::application::disk_reconcile::emit::run_internal_disk_reconcile(
            &app,
            pool.inner(),
            &game_id,
            vec![changed_path],
        )
        .await,
    );
    Ok(
        crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationResult {
            sync_warning: settlement.sync_warning,
        },
    )
}

#[specta::specta]
#[tauri::command]
pub async fn list_mod_preview_images(
    config: State<'_, ConfigService>,
    game_id: String,
    folder_path: String,
) -> Result<Vec<String>, AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)?;
    list_mod_preview_images_inner(&mod_root)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn save_mod_preview_image(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    object_name: String,
    image_data: Vec<u8>,
) -> Result<String, AppError> {
    ensure_image_size(&image_data)?;

    let mod_root = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [mod_root.to_string_lossy().to_string()];
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::Thumbnail)
        .await?;
    let guard = watcher.suppressor.suppress_paths([mod_root.as_ref()]);
    let saved = save_mod_preview_image_inner(&mod_root, &object_name, &image_data)?;
    drop(lock);
    drop(guard);
    Ok(saved)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn remove_mod_preview_image(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
    image_path: String,
) -> Result<(), AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [mod_root.to_string_lossy().to_string()];
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    resolve_image_path(&mod_root, &image_path)?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::Thumbnail)
        .await?;
    let guard = watcher.suppressor.suppress_paths([mod_root.as_ref()]);
    remove_mod_preview_image_inner(&mod_root, &image_path)?;
    crate::platform::images::thumbnail_cache::ThumbnailCache::invalidate_folder(
        &mod_root.to_string_lossy(),
    );
    drop(lock);
    drop(guard);
    Ok(())
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn clear_mod_preview_images(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    op_lock: State<'_, MutationCoordinator>,
    watcher: State<'_, WatcherState>,
    game_id: String,
    folder_path: String,
) -> Result<Vec<String>, AppError> {
    let mod_root = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [mod_root.to_string_lossy().to_string()];
    crate::modules::reconciliation::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&preflight_paths),
    )
    .await?;
    let lock = op_lock
        .acquire_exempt(crate::modules::mutation::coordinator::MutationExemption::Thumbnail)
        .await?;
    let guard = watcher.suppressor.suppress_paths([mod_root.as_ref()]);
    let removed = clear_mod_preview_images_inner(&mod_root)?;
    crate::platform::images::thumbnail_cache::ThumbnailCache::invalidate_folder(
        &mod_root.to_string_lossy(),
    );
    drop(lock);
    drop(guard);
    Ok(removed)
}
