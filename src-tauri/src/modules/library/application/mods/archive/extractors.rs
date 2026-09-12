use super::is_cancelled;
use super::progress::emit_throttled_progress;
use super::security::{
    map_archive_error, reject_multi_volume_archive, validate_archive_signature,
    validate_entry_path, validate_entry_type, EntryKind, ExtractionBudget, OutputPathRegistry,
};
use super::types::{ArchiveFormat, ExtractionEvent};
use super::zip_reader::extract_zip_to_dir;
use crate::shared::errors::AppError;
use compress_tools::{ArchiveContents, ArchiveIteratorBuilder, ArchivePassword};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use tauri::ipc::Channel;

#[allow(clippy::too_many_arguments)]
pub(super) fn extract_to_dir_with_budget(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
    format: ArchiveFormat,
    cancel_token: Option<Arc<AtomicBool>>,
    on_progress: Option<&Channel<ExtractionEvent>>,
    budget: &mut ExtractionBudget,
) -> Result<usize, AppError> {
    if is_cancelled(&cancel_token) {
        return Err(AppError::Cancelled);
    }
    reject_multi_volume_archive(archive_path)?;
    validate_archive_signature(archive_path, format)?;

    if format == ArchiveFormat::Zip {
        return extract_zip_to_dir(
            archive_path,
            dest_path,
            password,
            cancel_token,
            on_progress,
            budget,
        );
    }

    let file = fs::File::open(archive_path)?;
    let password_supplied = password.is_some();
    let mut builder = ArchiveIteratorBuilder::new(file).mtree_format(false);
    if let Some(value) = password {
        let password = ArchivePassword::new(value)
            .map_err(|error| map_archive_error(error, password_supplied))?;
        builder = builder.with_password(password);
    }
    let mut iterator = builder
        .build()
        .map_err(|error| map_archive_error(error, password_supplied))?;
    let root = dest_path.canonicalize()?;
    let mut current: Option<OpenEntry> = None;
    let mut files_extracted = 0_usize;
    let mut last_progress = Instant::now();
    let mut last_file_name = String::new();
    let mut output_paths = OutputPathRegistry::default();

    for content in iterator.by_ref() {
        if is_cancelled(&cancel_token) {
            return Err(AppError::Cancelled);
        }
        match content {
            ArchiveContents::StartOfEntry(name, stat) => {
                if current.is_some() {
                    return Err(AppError::Io(
                        "Archive started an entry before ending the previous entry".to_string(),
                    ));
                }
                let kind = validate_entry_type(stat.st_mode.into(), stat.st_nlink.max(0) as u64)?;
                let relative_path = validate_entry_path(Path::new(""), &name, kind)?;
                output_paths.register(&relative_path, kind)?;
                let output_path = root.join(relative_path);
                budget.start_entry(stat.st_size.max(0) as u64)?;
                let file = match kind {
                    EntryKind::Directory => {
                        fs::create_dir_all(&output_path)?;
                        None
                    }
                    EntryKind::File => {
                        let parent = output_path.parent().ok_or_else(|| {
                            AppError::Security("Archive output has no parent".to_string())
                        })?;
                        fs::create_dir_all(parent)?;
                        let canonical_parent = parent.canonicalize()?;
                        if !canonical_parent.starts_with(&root) {
                            return Err(AppError::Security(
                                "Archive output escaped the extraction root".to_string(),
                            ));
                        }
                        Some(
                            fs::OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(&output_path)?,
                        )
                    }
                };
                current = Some(OpenEntry {
                    name,
                    output_path,
                    file,
                    kind,
                });
            }
            ArchiveContents::DataChunk(chunk) => {
                let entry = current.as_mut().ok_or_else(|| {
                    AppError::Io("Archive produced data outside an entry".to_string())
                })?;
                if entry.kind != EntryKind::File {
                    return Err(AppError::Security(
                        "Archive directory entry contained file data".to_string(),
                    ));
                }
                budget.consume_chunk(chunk.len(), &cancel_token)?;
                entry
                    .file
                    .as_mut()
                    .ok_or_else(|| AppError::Io("Archive output file was not open".to_string()))?
                    .write_all(&chunk)?;
            }
            ArchiveContents::EndOfEntry => {
                let Some(mut entry) = current.take() else {
                    continue;
                };
                if let Some(file) = entry.file.as_mut() {
                    file.flush()?;
                    files_extracted += 1;
                    last_file_name = entry
                        .output_path
                        .file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                        .unwrap_or(entry.name);
                    if let Some(channel) = on_progress {
                        emit_throttled_progress(
                            channel,
                            &mut last_progress,
                            last_file_name.clone(),
                            files_extracted,
                            0,
                        );
                    }
                }
            }
            ArchiveContents::Err(error) => {
                return Err(map_archive_error(error, password_supplied));
            }
        }
    }
    iterator
        .close()
        .map_err(|error| map_archive_error(error, password_supplied))?;
    if current.is_some() {
        return Err(AppError::Io(
            "Archive ended before the current entry completed".to_string(),
        ));
    }
    if let Some(channel) = on_progress {
        if files_extracted > 0 {
            emit_throttled_progress(
                channel,
                &mut last_progress,
                last_file_name,
                files_extracted,
                files_extracted,
            );
        }
    }
    Ok(files_extracted)
}

struct OpenEntry {
    name: String,
    output_path: PathBuf,
    file: Option<fs::File>,
    kind: EntryKind,
}

pub(super) fn unpack_nested_archives(
    dir: &Path,
    current_depth: usize,
    max_depth: usize,
    cancel_token: &Option<Arc<AtomicBool>>,
    password: Option<&str>,
    budget: &mut ExtractionBudget,
) -> Result<usize, AppError> {
    if current_depth >= max_depth {
        let contains_more_archives = walkdir::WalkDir::new(dir)
            .min_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .any(|entry| {
                entry.file_type().is_file() && ArchiveFormat::detect(entry.path()).is_some()
            });
        if contains_more_archives {
            return Err(AppError::Validation(format!(
                "nested_archive_depth_limit: nested archive exceeds the {max_depth}-layer extraction limit in {}",
                dir.display()
            )));
        }
        return Ok(0);
    }

    let entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, std::io::Error>>()?;
    let mut total_extracted = 0_usize;
    for entry in entries {
        if is_cancelled(cancel_token) {
            return Err(AppError::Cancelled);
        }
        let path = entry.path();
        if path.is_dir() {
            total_extracted += unpack_nested_archives(
                &path,
                current_depth,
                max_depth,
                cancel_token,
                password,
                budget,
            )?;
            continue;
        }
        let Some(format) = ArchiveFormat::detect(&path) else {
            continue;
        };
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let sub_dest = dir.join(stem);
        if sub_dest.exists() {
            return Err(AppError::Validation(format!(
                "nested_archive_destination_collision: nested archive '{}' conflicts with existing folder '{}'",
                path.display(),
                sub_dest.display()
            )));
        }
        fs::create_dir_all(&sub_dest)?;
        let extracted = match extract_to_dir_with_budget(
            &path,
            &sub_dest,
            password,
            format,
            cancel_token.clone(),
            None,
            budget,
        ) {
            Ok(extracted) => extracted,
            Err(error) => {
                fs::remove_dir_all(&sub_dest).ok();
                return Err(error);
            }
        };
        total_extracted += extracted;
        if let Err(error) = crate::platform::fs::recycle_bin::move_path_to_recycle_bin(&path) {
            log::warn!("Failed to move extracted nested archive to the Recycle Bin: {error}");
        }
        total_extracted += unpack_nested_archives(
            &sub_dest,
            current_depth + 1,
            max_depth,
            cancel_token,
            password,
            budget,
        )?;
    }
    Ok(total_extracted)
}
