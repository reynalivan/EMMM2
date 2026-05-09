use super::types::{ArchiveAnalysis, ArchiveEntryInfo, ArchiveFormat};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

const MAX_ENTRIES: usize = 500;

pub fn analyze_archive(archive_path: &Path) -> Result<ArchiveAnalysis, String> {
    let file_name = archive_path
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if file_name.contains(".part")
        || file_name.contains(".z0")
        || file_name.contains(".r0")
        || file_name.ends_with(".001")
    {
        return Err("Multi-volume archives not supported".into());
    }

    let format = ArchiveFormat::detect(archive_path)
        .ok_or_else(|| format!("Unsupported archive format: {}", archive_path.display()))?;

    match format {
        ArchiveFormat::Zip => analyze_zip(archive_path, format),
        ArchiveFormat::SevenZ => analyze_7z(archive_path, format),
        ArchiveFormat::Rar => analyze_rar(archive_path, format),
    }
}

fn analyze_zip(archive_path: &Path, format: ArchiveFormat) -> Result<ArchiveAnalysis, String> {
    let file =
        fs::File::open(archive_path).map_err(|error| format!("Failed to open archive: {error}"))?;
    let file_size_bytes = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("Failed to read ZIP: {error}"))?;

    let mut summary = ArchiveSummary::new(format, file_size_bytes);
    for i in 0..archive.len() {
        match archive.by_index(i) {
            Ok(entry) => summary.push_entry(
                entry.name(),
                entry.is_dir(),
                entry.size(),
                entry.encrypted(),
            ),
            Err(error) if is_password_error(&error.to_string()) => {
                summary.is_encrypted = true;
            }
            Err(error) => return Err(format!("Failed to read entry: {error}")),
        }
    }

    summary.file_count = archive.len();
    Ok(summary.finish())
}

fn analyze_7z(archive_path: &Path, format: ArchiveFormat) -> Result<ArchiveAnalysis, String> {
    let file_size_bytes = fs::metadata(archive_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let mut file =
        fs::File::open(archive_path).map_err(|error| format!("Failed to open 7z: {error}"))?;
    let archive = match sevenz_rust::Archive::read(&mut file, file_size_bytes, &[]) {
        Ok(archive) => archive,
        Err(error) if is_password_error(&error.to_string()) => {
            return Ok(encrypted_analysis(format, file_size_bytes));
        }
        Err(error) => return Err(format!("Failed to analyze 7z: {error}")),
    };

    let mut summary = ArchiveSummary::new(format, file_size_bytes);
    for entry in archive.files {
        summary.push_entry(entry.name(), entry.is_directory(), entry.size(), false);
    }

    Ok(summary.finish())
}

fn analyze_rar(archive_path: &Path, format: ArchiveFormat) -> Result<ArchiveAnalysis, String> {
    let path_str = archive_path
        .to_str()
        .ok_or("RAR path contains invalid UTF-8")?;
    let temp_dir = tempfile::tempdir()
        .map_err(|error| format!("Failed to create temp dir for RAR analysis: {error}"))?;
    let temp_str = temp_dir
        .path()
        .to_str()
        .ok_or("Temp path contains invalid UTF-8")?;
    let file_size_bytes = fs::metadata(archive_path)
        .map(|meta| meta.len())
        .unwrap_or(0);

    let archive = match rar::Archive::extract_all(path_str, temp_str, "") {
        Ok(archive) => archive,
        Err(error) if is_password_error(&format!("{error:?}")) => {
            return Ok(encrypted_analysis(format, file_size_bytes));
        }
        Err(error) => return Err(format!("Failed to parse RAR: {error:?}")),
    };

    let mut summary = ArchiveSummary::new(format, file_size_bytes);
    for entry in archive.files {
        let name = entry.name.to_string();
        let is_dir = name.ends_with('/') || name.ends_with('\\');
        summary.push_entry(&name, is_dir, entry.unpacked_size, false);
    }

    Ok(summary.finish())
}

struct ArchiveSummary {
    format: ArchiveFormat,
    file_count: usize,
    has_ini: bool,
    uncompressed_size: u64,
    file_size_bytes: u64,
    root_dirs: HashSet<String>,
    is_encrypted: bool,
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
            is_encrypted: false,
            contains_nested_archives: false,
            entries: Vec::new(),
        }
    }

    fn push_entry(&mut self, name: &str, is_dir: bool, size: u64, is_encrypted: bool) {
        self.file_count += 1;
        self.uncompressed_size += size;
        self.is_encrypted |= is_encrypted;
        self.has_ini |= name.to_lowercase().ends_with(".ini");
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
            is_encrypted: self.is_encrypted,
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

fn is_password_error(message: &str) -> bool {
    message.contains("password")
        || message.contains("Password")
        || message.contains("decrypt")
        || message.contains("encrypted")
}

fn is_nested_archive_name(name: &str) -> bool {
    let ext = name.split('.').next_back().unwrap_or("").to_lowercase();
    ext == "zip" || ext == "rar" || ext == "7z"
}
