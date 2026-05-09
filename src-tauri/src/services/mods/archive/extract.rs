use super::classify::{collect_loose_files_recursive, find_mod_roots, resolve_unique_dest};
use super::destination::{
    check_disk_space, move_to_extracted_dir, parent_dir_join, remove_existing_dest,
};
use super::extractors::{extract_to_dir, unpack_nested_archives};
use super::progress::aborted_result;
use super::staging::{cleanup_temp_extract_parent, TempDirGuard};
use super::types::{ArchiveFormat, ExtractionEvent, ExtractionResult};
use crate::services::fs_utils::file_utils::rename_cross_drive_fallback;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;

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
#[allow(clippy::too_many_arguments)] // Archive extraction keeps user options explicit at the service boundary.
pub fn extract_archive(
    archive_path: &Path,
    mods_dir: &Path,
    password: Option<&str>,
    overwrite: bool,
    cancel_token: Option<Arc<AtomicBool>>,
    custom_name: Option<&str>,
    disable_after: bool,
    unpack_nested: bool,
    on_progress: Option<&Channel<ExtractionEvent>>,
) -> Result<ExtractionResult, String> {
    let format = ArchiveFormat::detect(archive_path)
        .ok_or_else(|| format!("Unsupported archive format: {}", archive_path.display()))?;
    let archive_name = archive_display_name(archive_path, custom_name);

    let analysis = crate::services::mods::archive::analyze_archive(archive_path)?;
    check_disk_space(mods_dir, analysis.uncompressed_size + (50 * 1024 * 1024))?;

    let temp_path = mods_dir
        .join(".temp_extract")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&temp_path)
        .map_err(|error| format!("Failed to create temp dir: {error}"))?;
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
        Err(error) if error == "ABORTED" => return Ok(aborted_result(archive_name, 0)),
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
        return Err("Not a valid 3DMigoto mod archive (no valid .ini found)".into());
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

fn is_cancelled(cancel_token: &Option<Arc<AtomicBool>>) -> bool {
    cancel_token
        .as_ref()
        .map(|token| token.load(Ordering::SeqCst))
        .unwrap_or(false)
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
) -> Result<Vec<String>, String> {
    if mod_roots.len() == 1 && mod_roots[0] == temp_path {
        let dest = destination_for(mods_dir, archive_name, overwrite);
        move_root_to_dest(guard.path(), &dest, overwrite, archive_name)?;
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
        move_root_to_dest(root, &dest, overwrite, &name)?;

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

fn move_root_to_dest(root: &Path, dest: &Path, overwrite: bool, name: &str) -> Result<(), String> {
    if overwrite {
        remove_existing_dest(dest)?;
    }

    rename_cross_drive_fallback(root, dest)
        .map_err(|error| format!("Failed to move mod '{}' to destination: {error}", name))
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

        if folder_name.starts_with("DISABLED ") {
            renamed_paths.push(dest_path);
            continue;
        }

        let disabled_path = path.with_file_name(format!("DISABLED {folder_name}"));
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
