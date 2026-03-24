use std::path::Path;

use crate::domain::errors::CollectionError;
use crate::pipeline::apply_pipeline::ApplyContext;
use crate::DISABLED_PREFIX;

/// Step 6: Batch rename mod folders on the filesystem.
///
/// - Enabling: remove DISABLED prefix from folder name
/// - Disabling: add DISABLED prefix to folder name
///
/// Uses Tokio semaphore to limit concurrent FS operations.
pub async fn rename(ctx: &mut ApplyContext) -> Result<(), CollectionError> {
    let _guard = crate::services::scanner::watcher::SuppressionGuard::new(&ctx.suppressor);
    let mods_path = &ctx.mods_path;

    // Resolve path_keys to actual folder paths
    let to_enable_paths = resolve_paths(mods_path, &ctx.to_enable);
    let to_disable_paths = resolve_paths(mods_path, &ctx.to_disable);

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(16));
    let mut handles = Vec::new();

    // Enable: remove DISABLED prefix
    for (path_key, folder_path) in &to_enable_paths {
        let sem = semaphore.clone();
        let folder = folder_path.clone();
        let key = path_key.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            rename_enable(&folder, &key)
        }));
    }

    // Disable: add DISABLED prefix
    for (path_key, folder_path) in &to_disable_paths {
        let sem = semaphore.clone();
        let folder = folder_path.clone();
        let key = path_key.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            rename_disable(&folder, &key)
        }));
    }

    let mut enabled_count = 0usize;
    let mut disabled_count = 0usize;
    let mut errors = Vec::new();

    for handle in handles {
        match handle.await {
            Ok(Ok(RenameAction::Enabled)) => enabled_count += 1,
            Ok(Ok(RenameAction::Disabled)) => disabled_count += 1,
            Ok(Ok(RenameAction::Skipped)) => {}
            Ok(Err(e)) => errors.push(e),
            Err(join_err) => errors.push(format!("Task join error: {}", join_err)),
        }
    }

    ctx.mods_enabled = enabled_count;
    ctx.mods_disabled = disabled_count;

    if !errors.is_empty() {
        log::warn!(
            "apply_pipeline[batch_rename]: {} errors during rename: {:?}",
            errors.len(),
            errors
        );
    }

    log::info!(
        "apply_pipeline[batch_rename]: {} enabled, {} disabled, {} errors",
        enabled_count,
        disabled_count,
        errors.len()
    );

    Ok(())
}

enum RenameAction {
    Enabled,
    Disabled,
    Skipped,
}

/// Resolve path_keys to filesystem paths by searching the mods directory.
fn resolve_paths(mods_path: &Path, path_keys: &[String]) -> Vec<(String, std::path::PathBuf)> {
    path_keys
        .iter()
        .filter_map(|key| {
            // Try to find the folder by key (case-insensitive on Windows)
            let candidate = mods_path.join(key);
            if candidate.exists() {
                return Some((key.clone(), candidate));
            }
            // Try with DISABLED prefix
            let file_name = Path::new(key).file_name()?.to_string_lossy().to_string();
            let parent = Path::new(key).parent().unwrap_or(Path::new(""));
            let disabled_name = format!("{}{}", DISABLED_PREFIX, file_name);
            let disabled_candidate = mods_path.join(parent).join(&disabled_name);
            if disabled_candidate.exists() {
                return Some((key.clone(), disabled_candidate));
            }
            None
        })
        .collect()
}

fn rename_enable(folder_path: &Path, _path_key: &str) -> Result<RenameAction, String> {
    let file_name = folder_path
        .file_name()
        .ok_or_else(|| "No file name".to_string())?
        .to_string_lossy();

    if !file_name.starts_with(DISABLED_PREFIX) {
        return Ok(RenameAction::Skipped); // Already enabled
    }

    let new_name = file_name.trim_start_matches(DISABLED_PREFIX);
    let new_path = folder_path.with_file_name(new_name);

    std::fs::rename(folder_path, &new_path).map_err(|e| {
        format!(
            "Enable rename failed for '{}': {}",
            folder_path.display(),
            e
        )
    })?;

    Ok(RenameAction::Enabled)
}

fn rename_disable(folder_path: &Path, _path_key: &str) -> Result<RenameAction, String> {
    let file_name = folder_path
        .file_name()
        .ok_or_else(|| "No file name".to_string())?
        .to_string_lossy();

    if file_name.starts_with(DISABLED_PREFIX) {
        return Ok(RenameAction::Skipped); // Already disabled
    }

    let new_name = format!("{}{}", DISABLED_PREFIX, file_name);
    let new_path = folder_path.with_file_name(&new_name);

    std::fs::rename(folder_path, &new_path).map_err(|e| {
        format!(
            "Disable rename failed for '{}': {}",
            folder_path.display(),
            e
        )
    })?;

    Ok(RenameAction::Disabled)
}
