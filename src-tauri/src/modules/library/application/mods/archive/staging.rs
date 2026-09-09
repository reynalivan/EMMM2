use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::shared::errors::AppError;
use tauri::ipc::Channel;

use super::destination::{
    check_disk_space, effective_archive_limits, ARCHIVE_DISK_RESERVE_BYTES,
};
use super::extractors::{extract_to_dir_with_budget, unpack_nested_archives};
use super::is_cancelled;
use super::security::{ArchiveLimits, ExtractionBudget};
use super::types::{ExtractionEvent, StagedArchive};

#[derive(Clone)]
pub struct StagingExtractOptions {
    pub password: Option<String>,
    pub cancel_token: Option<Arc<AtomicBool>>,
    pub unpack_nested: bool,
    pub on_progress: Option<Channel<ExtractionEvent>>,
}

impl Default for StagingExtractOptions {
    fn default() -> Self {
        Self {
            password: None,
            cancel_token: None,
            unpack_nested: true,
            on_progress: None,
        }
    }
}

pub fn extract_archive_to_staging_with_options(
    archive_path: &Path,
    staging_dir: &Path,
    options: StagingExtractOptions,
) -> Result<StagedArchive, AppError> {
    extract_archive_to_staging_with_limits(
        archive_path,
        staging_dir,
        options,
        ArchiveLimits::default(),
    )
}

pub fn extract_archive_to_staging_with_limits(
    archive_path: &Path,
    staging_dir: &Path,
    options: StagingExtractOptions,
    limits: ArchiveLimits,
) -> Result<StagedArchive, AppError> {
    let analysis = super::analyze::analyze_archive_with_limits(
        archive_path,
        limits,
        options.cancel_token.clone(),
    )?;
    let required_space = analysis
        .uncompressed_size
        .checked_add(ARCHIVE_DISK_RESERVE_BYTES)
        .ok_or_else(|| AppError::Validation("Archive staging size overflow".to_string()))?;
    check_disk_space(staging_dir, required_space)?;
    if staging_dir.exists() {
        return Err(AppError::Validation(format!(
            "Import staging destination already exists: {}",
            staging_dir.display()
        )));
    }
    if is_cancelled(&options.cancel_token) {
        return Err(AppError::Cancelled);
    }

    fs::create_dir_all(staging_dir)?;
    let mut guard = TempDirGuard::new(staging_dir.to_path_buf());
    let effective_limits = effective_archive_limits(staging_dir, limits)?;
    let mut budget = ExtractionBudget::new(effective_limits, analysis.file_size_bytes);
    let mut extracted = extract_to_dir_with_budget(
        archive_path,
        guard.path(),
        options.password.as_deref(),
        analysis.format,
        options.cancel_token.clone(),
        options.on_progress.as_ref(),
        &mut budget,
    )?;
    if is_cancelled(&options.cancel_token) {
        return Err(AppError::Cancelled);
    }
    if options.unpack_nested {
        extracted += unpack_nested_archives(
            guard.path(),
            0,
            2,
            &options.cancel_token,
            options.password.as_deref(),
            &mut budget,
        )?;
    }
    if is_cancelled(&options.cancel_token) {
        return Err(AppError::Cancelled);
    }

    let mod_roots = super::classify::find_mod_roots(guard.path(), 5);
    if mod_roots.is_empty() {
        return Err(AppError::Validation(
            "Not a valid 3DMigoto mod archive (no valid .ini found)".to_string(),
        ));
    }
    guard.commit();
    Ok(StagedArchive {
        mod_roots,
        files_extracted: extracted,
    })
}

pub(super) struct TempDirGuard {
    path: PathBuf,
    committed: bool,
}

impl TempDirGuard {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            committed: false,
        }
    }

    pub(super) fn commit(&mut self) {
        self.committed = true;
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        if let Err(error) = fs::remove_dir_all(&self.path) {
            log::warn!(
                target: "emmm.archive.cleanup",
                "Deferred cleanup for app-owned archive staging {}: {error}",
                self.path.display()
            );
            return;
        }
        cleanup_temp_extract_parent(&self.path);
    }
}

pub(super) fn cleanup_temp_extract_parent(temp_dir: &Path) {
    let Some(parent) = temp_dir.parent() else {
        return;
    };

    if !parent
        .file_name()
        .map(|name| name == ".temp_extract")
        .unwrap_or(false)
    {
        return;
    }

    if let Ok(mut entries) = fs::read_dir(parent) {
        if entries.next().is_none() {
            fs::remove_dir(parent).ok();
        }
    }
}
