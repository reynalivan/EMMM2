use super::is_cancelled;
use super::security::{
    map_archive_error, reject_multi_volume_archive, validate_archive_signature,
    validate_entry_path, validate_entry_type, ArchiveLimits, EntryKind, ExtractionBudget,
    OutputPathRegistry,
};
use super::types::{ArchiveAnalysis, ArchiveEntryInfo, ArchiveFormat};
use super::zip_reader::analyze_zip;
use crate::shared::errors::AppError;
use compress_tools::{ArchiveContents, ArchiveIteratorBuilder};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

const MAX_ENTRIES: usize = 500;

pub fn analyze_archive(archive_path: &Path) -> Result<ArchiveAnalysis, AppError> {
    analyze_archive_with_limits(archive_path, ArchiveLimits::default(), None)
}

pub fn analyze_archive_with_limits(
    archive_path: &Path,
    limits: ArchiveLimits,
    cancel_token: Option<Arc<AtomicBool>>,
) -> Result<ArchiveAnalysis, AppError> {
    reject_multi_volume_archive(archive_path)?;
    let format = ArchiveFormat::detect(archive_path).ok_or_else(|| {
        AppError::Validation(format!(
            "Unsupported archive format: {}",
            archive_path.display()
        ))
    })?;
    validate_archive_signature(archive_path, format)?;

    if format == ArchiveFormat::Zip {
        return analyze_zip(archive_path, limits, cancel_token);
    }

    let file_size_bytes = fs::metadata(archive_path)?.len();
    let file = fs::File::open(archive_path)?;
    let mut iterator = ArchiveIteratorBuilder::new(file)
        .mtree_format(false)
        .build()
        .map_err(|error| map_archive_error(error, false))?;
    let mut summary = ArchiveSummary::new(format, file_size_bytes);
    let mut budget = ExtractionBudget::new(limits, file_size_bytes);
    let mut current: Option<PendingEntry> = None;
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
                let relative_path = validate_entry_path(Path::new(""), &name)?;
                let kind = validate_entry_type(stat.st_mode.into(), stat.st_nlink.max(0) as u64)?;
                output_paths.register(&relative_path, kind)?;
                budget.start_entry(stat.st_size.max(0) as u64)?;
                current = Some(PendingEntry {
                    name,
                    is_dir: kind == EntryKind::Directory,
                    announced_size: stat.st_size.max(0) as u64,
                    actual_size: 0,
                });
            }
            ArchiveContents::DataChunk(chunk) => {
                let entry = current.as_mut().ok_or_else(|| {
                    AppError::Io("Archive produced data outside an entry".to_string())
                })?;
                budget.consume_chunk(chunk.len(), &cancel_token)?;
                entry.actual_size = entry
                    .actual_size
                    .checked_add(chunk.len() as u64)
                    .ok_or_else(|| AppError::Validation("Archive size overflow".to_string()))?;
            }
            ArchiveContents::EndOfEntry => {
                if let Some(entry) = current.take() {
                    summary.push_entry(
                        &entry.name,
                        entry.is_dir,
                        entry.announced_size.max(entry.actual_size),
                    )?;
                }
            }
            ArchiveContents::Err(error) => {
                let mapped = map_archive_error(error, false);
                if is_password_error(&mapped) {
                    return Ok(encrypted_analysis(format, file_size_bytes));
                }
                return Err(mapped);
            }
        }
    }
    iterator
        .close()
        .map_err(|error| map_archive_error(error, false))?;
    if current.is_some() {
        return Err(AppError::Io(
            "Archive ended before the current entry completed".to_string(),
        ));
    }
    Ok(summary.finish())
}

struct PendingEntry {
    name: String,
    is_dir: bool,
    announced_size: u64,
    actual_size: u64,
}

struct ArchiveSummary {
    format: ArchiveFormat,
    file_count: usize,
    has_ini: bool,
    uncompressed_size: u64,
    file_size_bytes: u64,
    root_dirs: HashSet<String>,
    contains_nested_archives: bool,
    entries: Vec<ArchiveEntryInfo>,
}

impl ArchiveSummary {
    fn new(format: ArchiveFormat, file_size_bytes: u64) -> Self {
        Self {
            format,
            file_count: 0,
            has_ini: false,
            uncompressed_size: 0,
            file_size_bytes,
            root_dirs: HashSet::new(),
            contains_nested_archives: false,
            entries: Vec::new(),
        }
    }

    fn push_entry(&mut self, name: &str, is_dir: bool, size: u64) -> Result<(), AppError> {
        self.file_count = self
            .file_count
            .checked_add(1)
            .ok_or_else(|| AppError::Validation("Archive entry count overflow".to_string()))?;
        self.uncompressed_size = self
            .uncompressed_size
            .checked_add(size)
            .ok_or_else(|| AppError::Validation("Archive size overflow".to_string()))?;
        self.has_ini |= name.to_ascii_lowercase().ends_with(".ini");
        self.contains_nested_archives |= is_nested_archive_name(name);

        if self.entries.len() < MAX_ENTRIES {
            self.entries.push(ArchiveEntryInfo {
                path: name.to_string(),
                is_dir,
                size,
            });
        }

        let normalized = name.replace('\\', "/");
        if let Some(first) = normalized.split('/').next() {
            if !first.is_empty() {
                self.root_dirs.insert(first.to_string());
            }
        }
        Ok(())
    }

    fn finish(self) -> ArchiveAnalysis {
        ArchiveAnalysis {
            format: self.format,
            file_count: self.file_count,
            has_ini: self.has_ini,
            uncompressed_size: self.uncompressed_size,
            file_size_bytes: self.file_size_bytes,
            single_root_folder: if self.root_dirs.len() == 1 {
                self.root_dirs.into_iter().next()
            } else {
                None
            },
            is_encrypted: false,
            contains_nested_archives: self.contains_nested_archives,
            entries: self.entries,
        }
    }
}

fn encrypted_analysis(format: ArchiveFormat, file_size_bytes: u64) -> ArchiveAnalysis {
    ArchiveAnalysis {
        format,
        file_count: 0,
        has_ini: false,
        uncompressed_size: 0,
        file_size_bytes,
        single_root_folder: None,
        is_encrypted: true,
        contains_nested_archives: false,
        entries: Vec::new(),
    }
}

fn is_password_error(error: &AppError) -> bool {
    matches!(
        error,
        AppError::ArchivePasswordRequired | AppError::ArchivePasswordIncorrect
    )
}

fn is_nested_archive_name(name: &str) -> bool {
    matches!(
        name.rsplit('.')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "zip" | "rar" | "7z"
    )
}
