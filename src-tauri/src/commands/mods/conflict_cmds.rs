use std::fs;
use std::path::Path;

use tauri::State;

use crate::services::fs_utils::operation_lock::OperationLock;
use crate::services::scanner::watcher::WatcherState;

use crate::domain::errors::AppError;
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::validate_path;
use crate::services::mods::core_ops::ConflictStrategy;

/// Resolve a naming conflict where both "X" and "DISABLED X" exist on disk.
#[allow(clippy::too_many_arguments)] // Tauri command boundary: states plus the IPC payload.
#[specta::specta]
#[tauri::command]
pub async fn resolve_conflict(
    app: tauri::AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
    state: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    keep_path: String,
    duplicate_path: String,
    strategy: ConflictStrategy,
) -> Result<String, AppError> {
    let keep = validate_path(&config, &game_id, &keep_path)?;
    let duplicate = validate_path(&config, &game_id, &duplicate_path)?;
    let op_guard = op_lock.acquire().await?;

    let renamed = crate::services::mods::core_ops::resolve_naming_conflict(
        crate::services::mods::core_ops::ResolveConflictRequest {
            config: &config,
            pool: pool.inner(),
            state: &state,
            op_guard: &op_guard,
            game_id: &game_id,
            keep: &keep,
            duplicate: &duplicate,
            strategy,
        },
    )
    .await?;

    // Single-writer convergence: the resolution renamed a folder on disk; a
    // previously untracked duplicate only gets its row through this reconcile.
    if let Err(error) = crate::services::disk_reconcile::emit::emit_internal_disk_reconcile(
        &app,
        pool.inner(),
        &game_id,
        vec![duplicate_path, renamed.clone()],
    )
    .await
    {
        log::warn!("Post-conflict-resolution disk reconcile failed: {error}");
    }

    Ok(renamed)
}

// ── Conflict Details (for comparison dialog) ─────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FolderDetail {
    pub path: String,
    pub folder_name: String,
    pub is_enabled: bool,
    #[specta(type = f64)]
    pub total_size: u64,
    #[specta(type = f64)]
    pub file_count: usize,
    pub files: Vec<FileEntry>,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FileEntry {
    pub name: String,
    #[specta(type = f64)]
    pub size: u64,
    pub is_ini: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct ConflictDetails {
    pub enabled: FolderDetail,
    pub disabled: FolderDetail,
}

/// Get detailed file listings for both enabled and disabled versions of a conflicting folder.
/// Used by the enhanced ConflictResolveDialog for side-by-side comparison.
#[specta::specta]
#[tauri::command]
pub async fn get_conflict_details(
    config: State<'_, ConfigService>,
    game_id: String,
    enabled_path: String,
    disabled_path: String,
) -> Result<ConflictDetails, AppError> {
    validate_path(&config, &game_id, &enabled_path)?;
    validate_path(&config, &game_id, &disabled_path)?;

    let enabled = scan_folder_detail(&enabled_path, true)?;
    let disabled = scan_folder_detail(&disabled_path, false)?;
    Ok(ConflictDetails { enabled, disabled })
}

fn scan_folder_detail(path_str: &str, is_enabled: bool) -> Result<FolderDetail, AppError> {
    let path = Path::new(path_str);
    if !path.exists() || !path.is_dir() {
        return Err(AppError::Io(format!(
            "Path does not exist or is not a directory: {path_str}"
        )));
    }

    let folder_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut files = Vec::new();
    let mut total_size: u64 = 0;
    let mut thumbnail_path: Option<String> = None;

    // Scan top-level files (non-recursive for performance)
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let lower = name.to_lowercase();
                let is_ini = lower.ends_with(".ini");

                if thumbnail_path.is_none() {
                    let thumb_exts = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];
                    if thumb_exts.iter().any(|ext| lower.ends_with(ext)) {
                        // Prefer "preview" or "thumbnail" named images
                        let stem = lower.rsplit('.').nth(1).unwrap_or("");
                        if stem.contains("preview")
                            || stem.contains("thumb")
                            || stem.contains("icon")
                        {
                            thumbnail_path = Some(entry_path.to_string_lossy().to_string());
                        }
                    }
                }

                total_size += size;
                files.push(FileEntry { name, size, is_ini });
            } else if entry_path.is_dir() {
                // Count subdir size (shallow)
                if let Ok(sub_entries) = fs::read_dir(&entry_path) {
                    for se in sub_entries.flatten() {
                        if se.path().is_file() {
                            total_size += se.metadata().map(|m| m.len()).unwrap_or(0);
                        }
                    }
                }
            }
        }
    }

    // If no priority thumbnail found, take the first image file
    if thumbnail_path.is_none() {
        for f in &files {
            let lower = f.name.to_lowercase();
            let thumb_exts = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];
            if thumb_exts.iter().any(|ext| lower.ends_with(ext)) {
                thumbnail_path = Some(path.join(&f.name).to_string_lossy().to_string());
                break;
            }
        }
    }

    // Sort files: INI first, then by name
    files.sort_by(|a, b| b.is_ini.cmp(&a.is_ini).then(a.name.cmp(&b.name)));

    Ok(FolderDetail {
        path: path_str.to_string(),
        folder_name: folder_name.clone(),
        is_enabled,
        total_size,
        file_count: files.len(),
        files,
        thumbnail_path,
    })
}

#[specta::specta]
#[tauri::command]
pub async fn ignore_object_conflict(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
    mod_ids: Vec<String>,
) -> Result<(), AppError> {
    crate::repo::conflict_repo::ignore_object_conflict(&pool, &game_id, &object_id, &mod_ids)
        .await?;
    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn revoke_object_conflict(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
    object_id: String,
) -> Result<(), AppError> {
    crate::repo::conflict_repo::revoke_object_conflict(&pool, &game_id, &object_id).await?;
    Ok(())
}

#[specta::specta]
#[tauri::command]
pub async fn list_ignored_object_conflicts(
    pool: State<'_, sqlx::SqlitePool>,
    game_id: String,
) -> Result<Vec<crate::domain::conflicts::IgnoredConflict>, AppError> {
    let list = crate::repo::conflict_repo::list_ignored_object_conflicts(&pool, &game_id).await?;
    Ok(list)
}
