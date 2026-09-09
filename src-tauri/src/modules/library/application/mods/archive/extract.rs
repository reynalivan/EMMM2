use super::classify::{collect_loose_files_recursive, find_mod_roots};
use super::destination::{
    check_disk_space, effective_archive_limits, move_to_extracted_dir, parent_dir_join,
    remove_existing_dest, ARCHIVE_DISK_RESERVE_BYTES,
};
use super::extractors::{extract_to_dir_with_budget, unpack_nested_archives};
use super::progress::aborted_result;
use super::staging::{cleanup_temp_extract_parent, TempDirGuard};
use super::types::{ExtractionEvent, ExtractionResult};
use crate::platform::fs::file_utils::rename_cross_drive_fallback;
use crate::shared::errors::AppError;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::ipc::Channel;

use super::is_cancelled;

pub type DestinationPreflight<'a> = dyn Fn(&[PathBuf]) -> Result<(), AppError> + 'a;

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
    /// Validate the final destination plan after staging and before any
    /// existing directory can be replaced or a staged root is committed.
    pub before_commit: Option<&'a DestinationPreflight<'a>>,
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
        before_commit,
        on_progress,
    } = options;
    let archive_name = archive_display_name(archive_path, custom_name);
    crate::modules::library::application::mods::core_ops::validate_folder_name_component(
        &archive_name,
    )?;

    let analysis = match super::analyze::analyze_archive_with_limits(
        archive_path,
        super::security::ArchiveLimits::default(),
        cancel_token.clone(),
    ) {
        Ok(analysis) => analysis,
        Err(AppError::Cancelled) => return Ok(aborted_result(archive_name, 0)),
        Err(error) => return Err(error),
    };
    let required_space = analysis
        .uncompressed_size
        .checked_add(ARCHIVE_DISK_RESERVE_BYTES)
        .ok_or_else(|| AppError::Validation("Archive staging size overflow".to_string()))?;
    check_disk_space(mods_dir, required_space)?;

    let temp_path = mods_dir
        .join(".temp_extract")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&temp_path)?;
    let mut guard = TempDirGuard::new(temp_path.clone());

    let effective_limits =
        effective_archive_limits(mods_dir, super::security::ArchiveLimits::default())?;
    let mut budget =
        super::security::ExtractionBudget::new(effective_limits, analysis.file_size_bytes);
    let mut files_extracted = match extract_to_dir_with_budget(
        archive_path,
        guard.path(),
        password,
        analysis.format,
        cancel_token.clone(),
        on_progress,
        &mut budget,
    ) {
        Ok(result) => result,
        Err(AppError::Cancelled) => return Ok(aborted_result(archive_name, 0)),
        Err(error) => return Err(error),
    };

    if is_cancelled(&cancel_token) {
        return Ok(aborted_result(archive_name, files_extracted));
    }

    if unpack_nested {
        match unpack_nested_archives(guard.path(), 0, 2, &cancel_token, password, &mut budget) {
            Ok(count) => files_extracted += count,
            Err(AppError::Cancelled) => return Ok(aborted_result(archive_name, files_extracted)),
            Err(error) => return Err(error),
        }
    }

    let mod_roots = find_mod_roots(guard.path(), 5);
    if mod_roots.is_empty() {
        return Err(AppError::Validation(
            "Not a valid 3DMigoto mod archive (no valid .ini found)".to_string(),
        ));
    }

    let loose_files = collect_loose_files_recursive(guard.path(), &mod_roots);
    let dest_paths = move_mod_roots(
        archive_path,
        mods_dir,
        &archive_name,
        &temp_path,
        &mod_roots,
        &loose_files,
        overwrite,
        disable_after,
        before_commit,
        &mut guard,
    )?;

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
        sync_warning: None,
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
    disable_after: bool,
    before_commit: Option<&DestinationPreflight<'_>>,
    guard: &mut TempDirGuard,
) -> Result<Vec<String>, AppError> {
    if mod_roots.len() == 1 && mod_roots[0] == temp_path {
        let destination_name = final_destination_name(archive_name, disable_after);
        let dest = destination_for(mods_dir, &destination_name, overwrite, &[])?;
        ensure_direct_child_destination(mods_dir, &dest)?;
        if let Some(validate) = before_commit {
            validate(std::slice::from_ref(&dest))?;
        }
        move_root_to_dest(guard.path(), &dest, overwrite)?;
        guard.commit();
        cleanup_temp_extract_parent(temp_path);
        return Ok(vec![dest.to_string_lossy().to_string()]);
    }

    let mut move_plan = Vec::with_capacity(mod_roots.len());
    let mut planned_destinations = Vec::with_capacity(mod_roots.len());
    for root in mod_roots {
        let name = root
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| archive_name.to_string());
        crate::modules::library::application::mods::core_ops::validate_folder_name_component(
            &name,
        )?;
        let destination_name = final_destination_name(&name, disable_after);
        let dest = destination_for(
            mods_dir,
            &destination_name,
            overwrite,
            &planned_destinations,
        )?;
        ensure_direct_child_destination(mods_dir, &dest)?;
        planned_destinations.push(dest.clone());
        move_plan.push((root, dest));
    }

    if let Some(validate) = before_commit {
        validate(&planned_destinations)?;
    }

    let mut dest_paths = Vec::with_capacity(move_plan.len());
    let mut loose_files_moved = false;
    for (root, dest) in move_plan {
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

fn destination_for(
    mods_dir: &Path,
    name: &str,
    overwrite: bool,
    planned_destinations: &[PathBuf],
) -> Result<PathBuf, AppError> {
    if overwrite {
        let destination = parent_dir_join(mods_dir, name);
        if contains_planned_identity(planned_destinations, &destination) {
            return Err(AppError::Validation(
                "Archive contains multiple mod roots with the same folder identity".to_string(),
            ));
        }
        return Ok(destination);
    }

    const MAX_NUMBERED_DESTINATIONS: u32 = 999;
    for counter in 1..=MAX_NUMBERED_DESTINATIONS {
        let candidate_name = if counter == 1 {
            name.to_string()
        } else {
            format!("{name} ({counter})")
        };
        let candidate = parent_dir_join(mods_dir, &candidate_name);
        let sibling_collision =
            crate::modules::library::application::mods::core_ops::find_sibling_identity_collision(
                mods_dir,
                &candidate_name,
                None,
            )
            .is_some();
        if !sibling_collision && !contains_planned_identity(planned_destinations, &candidate) {
            return Ok(candidate);
        }
    }

    loop {
        let candidate = parent_dir_join(mods_dir, &format!("{name} ({})", uuid::Uuid::new_v4()));
        if crate::modules::library::application::mods::core_ops::find_sibling_identity_collision(
            mods_dir,
            candidate
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default(),
            None,
        )
        .is_none()
            && !contains_planned_identity(planned_destinations, &candidate)
        {
            return Ok(candidate);
        }
    }
}

fn contains_planned_identity(planned: &[PathBuf], candidate: &Path) -> bool {
    let Some(candidate_name) = candidate.file_name().and_then(|value| value.to_str()) else {
        return true;
    };
    planned.iter().any(|path| {
        path.file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| crate::shared::path_key::names_equal_by_key(name, candidate_name))
    })
}

fn final_destination_name(name: &str, disable_after: bool) -> String {
    if disable_after && !crate::modules::workspace::domain::normalizer::is_disabled_folder(name) {
        return format!("{}{name}", crate::DISABLED_PREFIX);
    }
    name.to_string()
}

fn ensure_direct_child_destination(mods_dir: &Path, destination: &Path) -> Result<(), AppError> {
    if destination.parent() == Some(mods_dir) && destination.file_name().is_some() {
        return Ok(());
    }
    Err(AppError::Security(
        "Archive destination must be a direct child of the configured mods directory".to_string(),
    ))
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
