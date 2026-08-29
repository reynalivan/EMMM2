//! Rename a mod folder on disk. The command's trailing disk reconcile owns
//! the DB, stable-ID, runtime projection, and collection rewrite.

use super::naming::{
    find_existing_sibling_case_insensitive, rename_conflict_error, validate_folder_base_name,
};
use crate::modules::collections::domain::collection::CollectionReferenceImpact;
use crate::shared::errors::AppError;
use crate::modules::system::application::config::ConfigService;
use crate::platform::fs::guard::ValidatedPath;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct RenameResult {
    pub old_path: String,
    pub new_path: String,
    pub new_name: String,
    pub collection_impact: CollectionReferenceImpact,
    pub sync_warning: Option<crate::modules::workspace::application::disk_reconcile::types::CommittedMutationSyncWarning>,
}

pub async fn rename_mod_folder_inner(
    state: &WatcherState,
    folder_path: String,
    new_name: String,
) -> Result<RenameResult, AppError> {
    let path = Path::new(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(AppError::Io(format!(
            "Folder does not exist: {folder_path}"
        )));
    }

    validate_folder_base_name(&new_name)?;

    let parent = path
        .parent()
        .ok_or_else(|| AppError::Io("Cannot determine parent directory".to_string()))?;
    let old_folder_name = path
        .file_name()
        .ok_or_else(|| AppError::Io("Invalid folder name".to_string()))?
        .to_string_lossy()
        .to_string();

    let new_folder_name = if crate::modules::workspace::domain::normalizer::is_disabled_folder(&old_folder_name) {
        format!("{}{}", crate::DISABLED_PREFIX, new_name)
    } else {
        new_name.clone()
    };

    let new_path = parent.join(&new_folder_name);
    if let Some(existing_path) =
        find_existing_sibling_case_insensitive(parent, &new_folder_name, path)
    {
        let base_name = crate::modules::workspace::domain::normalizer::normalize_display_name(&old_folder_name);
        return Err(rename_conflict_error(&new_path, &existing_path, &base_name));
    }

    // A real rename changes identity, so both sides need their own entry;
    // the guard's tail keeps suppressing the async event pair after return.
    let _guard = state.suppressor.suppress_paths([path, new_path.as_path()]);

    crate::platform::fs::file_utils::rename_cross_drive_fallback(path, &new_path).map_err(
        |e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                let processes = crate::platform::fs::locking::get_locking_processes(path);
                if !processes.is_empty() {
                    return AppError::FileInUse {
                        path: folder_path.clone(),
                        processes,
                    };
                }

                return AppError::PathBusy {
                    path: folder_path.clone(),
                };
            }
            AppError::Io(format!("Failed to rename folder: {e}"))
        },
    )?;

    update_info_json_name(&new_path, &new_name);

    log::info!("Renamed '{}' -> '{}'", old_folder_name, new_folder_name);

    Ok(RenameResult {
        old_path: folder_path,
        new_path: new_path.to_string_lossy().to_string(),
        new_name,
        collection_impact: CollectionReferenceImpact::default(),
        sync_warning: None,
    })
}

fn update_info_json_name(folder_path: &Path, new_name: &str) {
    use crate::modules::library::application::mods::info_json;
    if folder_path.join("info.json").exists() {
        let update = info_json::ModInfoUpdate {
            actual_name: Some(new_name.to_string()),
            ..Default::default()
        };
        let _ = info_json::update_info_json(folder_path, &update);
    }
}

pub async fn rename_mod_folder_inner_service(
    _config: &ConfigService,
    _pool: &sqlx::SqlitePool,
    state: &WatcherState,
    _op_guard: &crate::platform::fs::operation_lock::OpGuard,
    old_path: &ValidatedPath,
    new_name: String,
    _game_id: &str,
) -> Result<RenameResult, AppError> {
    let canonical_path = old_path;

    // AC-21.1.6: Windows path limit check (260 characters)
    #[cfg(target_os = "windows")]
    {
        let parent = canonical_path.parent().unwrap_or_else(|| Path::new(""));
        let new_abs_path = parent.join(&new_name);
        let path_str = new_abs_path.to_string_lossy();
        if path_str.len() >= 260 {
            return Err(AppError::Io(format!(
                "Windows path limit exceeded ({} chars). Please use a shorter name.",
                path_str.len()
            )));
        }
    }

    rename_mod_folder_inner(
        state,
        canonical_path.to_string_lossy().to_string(),
        new_name.clone(),
    )
    .await
}
