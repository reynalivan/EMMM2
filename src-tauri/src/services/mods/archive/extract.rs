use super::classify::{collect_loose_files_recursive, find_mod_roots, resolve_unique_dest};
use super::destination::{
    check_disk_space, move_to_extracted_dir, parent_dir_join, remove_existing_dest,
};
use super::extractors::{extract_to_dir, unpack_nested_archives};
use super::progress::aborted_result;
use super::staging::{cleanup_temp_extract_parent, TempDirGuard};
use super::types::{ArchiveFormat, ExtractionEvent, ExtractionResult};
use crate::domain::errors::AppError;
use crate::services::fs_utils::file_utils::rename_cross_drive_fallback;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::ipc::Channel;

use super::is_cancelled;

/// Caller-supplied knobs for [`extract_archive`].
///
/// A struct rather than seven more positional parameters: almost every call
/// site wants the defaults, and passing them positionally made each one an
/// unreadable run of `None`/`false` that had to be re-counted whenever an
/// option was added.
#[derive(Default)]
pub struct ExtractOptions<'a> {
    /// Password for an encrypted archive.
    pub password: Option<&'a str>,
    /// Replace an existing destination folder instead of uniquifying the name.
    pub overwrite: bool,
    /// Cooperative cancellation, polled between entries.
    pub cancel_token: Option<Arc<AtomicBool>>,
    /// Overrides the name derived from the archive file.
    pub custom_name: Option<&'a str>,
    /// Land the extracted mod folders disabled.
    pub disable_after: bool,
    /// Recursively unpack archives found inside the archive.
    pub unpack_nested: bool,
    pub on_progress: Option<&'a Channel<ExtractionEvent>>,
}

/// Extract any supported archive with smart mod root detection.
///
/// Pipeline:
/// 1. Extract to `{mods_dir}/.temp_extract/<uuid>/`
/// 2. Find mod roots (shallowest folders with valid 3DMigoto .ini)
/// 3. Collect loose files (readme, images) from wrapper layers
/// 4. Route based on classification:
///    - Single mod -> move to `mods_dir/{name}/`
///    - Multi-mod pack -> move each subfolder independently
///    - Invalid -> delete temp, return error
/// 5. Move source archive to `{source_dir}/.extracted/`
pub fn extract_archive(
    archive_path: &Path,
    mods_dir: &Path,
    options: ExtractOptions<'_>,
) -> Result<ExtractionResult, AppError> {
    let ExtractOptions {
        password,
        overwrite,
        cancel_token,
        custom_name,
        disable_after,
        unpack_nested,
        on_progress,
    } = options;
    let format = ArchiveFormat::detect(archive_path).ok_or_else(|| {
        AppError::Internal(format!(
            "Unsupported archive format: {}",
            archive_path.display()
        ))
    })?;
    let archive_name = archive_display_name(archive_path, custom_name);

    let analysis = crate::services::mods::archive::analyze_archive(archive_path)?;
    check_disk_space(mods_dir, analysis.uncompressed_size + (50 * 1024 * 1024))?;

    let temp_path = mods_dir
        .join(".temp_extract")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&temp_path)?;
    let mut guard = TempDirGuard::new(temp_path.clone());

    let mut files_extracted = match extract_to_dir(
        archive_path,
        guard.path(),
        password,
        format,
        cancel_token.clone(),
        on_progress,
    ) {
        Ok(count) => count,
        Err(AppError::Cancelled) => return Ok(aborted_result(archive_name, 0)),
        Err(error) => return Err(error),
    };

    if is_cancelled(&cancel_token) {
        return Ok(aborted_result(archive_name, files_extracted));
    }

    if unpack_nested {
        files_extracted += unpack_nested_archives(guard.path(), 0, 2, &cancel_token);
    }

    let mod_roots = find_mod_roots(guard.path(), 5);
    if mod_roots.is_empty() {
        return Err(AppError::Validation(
            "Not a valid 3DMigoto mod archive (no valid .ini found)".to_string(),
        ));
    }

    let loose_files = collect_loose_files_recursive(guard.path(), &mod_roots);
    let mut dest_paths = move_mod_roots(
        archive_path,
        mods_dir,
        &archive_name,
        &temp_path,
        &mod_roots,
        &loose_files,
        overwrite,
        &mut guard,
    )?;

    if disable_after {
        dest_paths = apply_disabled_prefix(dest_paths);
    }

    if !dest_paths.is_empty() {
        if let Err(error) = move_to_extracted_dir(archive_path) {
            log::warn!("Failed to move archive to .extracted/ (non-fatal): {error}");
        }
    }

    let mod_count = dest_paths.len();
    Ok(ExtractionResult {
        archive_name,
        dest_paths,
        files_extracted,
        mod_count,
        success: true,
        error: None,
        aborted: false,
        collisions: Vec::new(),
    })
}

fn archive_display_name(archive_path: &Path, custom_name: Option<&str>) -> String {
    custom_name.map(str::to_string).unwrap_or_else(|| {
        archive_path
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "extracted_mod".to_string())
    })
}

#[allow(clippy::too_many_arguments)] // Archive staging carries source, target, collision, and progress context.
fn move_mod_roots(
    archive_path: &Path,
    mods_dir: &Path,
    archive_name: &str,
    temp_path: &Path,
    mod_roots: &[PathBuf],
    loose_files: &[PathBuf],
    overwrite: bool,
    guard: &mut TempDirGuard,
) -> Result<Vec<String>, AppError> {
    if mod_roots.len() == 1 && mod_roots[0] == temp_path {
        let dest = destination_for(mods_dir, archive_name, overwrite);
        move_root_to_dest(guard.path(), &dest, overwrite)?;
        guard.commit();
        cleanup_temp_extract_parent(temp_path);
        return Ok(vec![dest.to_string_lossy().to_string()]);
    }

    let mut dest_paths = Vec::new();
    let mut loose_files_moved = false;
    for root in mod_roots {
        let name = root
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| archive_name.to_string());
        let dest = destination_for(mods_dir, &name, overwrite);
        move_root_to_dest(root, &dest, overwrite)?;

        if !loose_files_moved {
            move_loose_files(loose_files, &dest);
            loose_files_moved = true;
        }

        dest_paths.push(dest.to_string_lossy().to_string());
    }

    if dest_paths.is_empty() {
        log::warn!(
            "Archive '{}' produced mod roots but none were moved",
            archive_path.display()
        );
    }

    Ok(dest_paths)
}

fn destination_for(mods_dir: &Path, name: &str, overwrite: bool) -> PathBuf {
    if overwrite {
        return parent_dir_join(mods_dir, name);
    }

    resolve_unique_dest(mods_dir, name)
}

fn move_root_to_dest(root: &Path, dest: &Path, overwrite: bool) -> Result<(), AppError> {
    if overwrite {
        remove_existing_dest(dest)?;
    }

    Ok(rename_cross_drive_fallback(root, dest)?)
}

fn move_loose_files(loose_files: &[PathBuf], dest: &Path) {
    for loose_file in loose_files {
        let Some(file_name) = loose_file.file_name() else {
            continue;
        };
        let target = dest.join(file_name);
        if target.exists() {
            continue;
        }
        if let Err(error) = rename_cross_drive_fallback(loose_file, &target) {
            log::warn!(
                "Failed to move loose file '{}' into '{}': {}",
                loose_file.display(),
                dest.display(),
                error
            );
        }
    }
}

fn apply_disabled_prefix(dest_paths: Vec<String>) -> Vec<String> {
    let mut renamed_paths = Vec::new();
    for dest_path in dest_paths {
        let path = Path::new(&dest_path);
        let Some(folder_name) = path.file_name().and_then(|value| value.to_str()) else {
            renamed_paths.push(dest_path);
            continue;
        };

        // The canonical matcher also covers the legacy `disabled_`/`Disabled-`
        // spellings; a plain starts_with would prefix those a second time.
        if crate::common::normalizer::is_disabled_folder(folder_name) {
            renamed_paths.push(dest_path);
            continue;
        }

        let disabled_path = path.with_file_name(format!("{}{folder_name}", crate::DISABLED_PREFIX));
        match fs::rename(path, &disabled_path) {
            Ok(()) => renamed_paths.push(disabled_path.to_string_lossy().to_string()),
            Err(error) => {
                log::warn!("Failed to apply DISABLED prefix to {folder_name}: {error}");
                renamed_paths.push(dest_path);
            }
        }
    }

    renamed_paths
}
