use crate::modules::settings::application::config::ConfigService;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::guard::validate_path;
use crate::shared::errors::AppError;

#[cfg(test)]
#[path = "tests/thumbnail_cmds_tests.rs"]
mod tests;

/// Lazily resolve thumbnail for a single mod folder.
/// Called per-card from the frontend after the folder list is rendered.
/// Delegates to ThumbnailCache::resolve() which caps concurrency (4 max),
/// checks folder-keyed L1, and falls back to FS traversal + image processing.
#[tauri::command]
#[specta::specta]
pub async fn get_mod_thumbnail(
    game_id: String,
    folder_path: String,
) -> Result<Option<String>, AppError> {
    use crate::platform::images::thumbnail_cache::ThumbnailCache;
    ThumbnailCache::resolve(&game_id, &folder_path).await
}

/// Delete the thumbnail file for a mod folder (if found) and invalidate cache.
#[tauri::command]
#[specta::specta]
pub async fn delete_mod_thumbnail(
    app: tauri::AppHandle,
    config: tauri::State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: tauri::State<'_, WatcherState>,
    op_lock: tauri::State<'_, crate::modules::mutation::coordinator::MutationCoordinator>,
    game_id: String,
    folder_path: String,
) -> Result<(), AppError> {
    use crate::modules::workspace::application::scanner::core::thumbnail::find_thumbnail;
    use crate::platform::images::thumbnail_cache::ThumbnailCache;

    let path = validate_path(&config, &game_id, &folder_path)?;
    let preflight_paths = [path.to_string_lossy().to_string()];
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
    let guard = watcher.suppressor.suppress_paths([path.as_ref()]);
    if let Some(thumb_path) = find_thumbnail(&path) {
        crate::platform::fs::recycle_bin::move_path_to_recycle_bin(&thumb_path)?;
        ThumbnailCache::invalidate(&thumb_path);
    }

    // Always invalidate the folder-keyed cache entry regardless of whether a file was found.
    ThumbnailCache::invalidate_folder(&folder_path);
    drop(lock);
    drop(guard);
    Ok(())
}
