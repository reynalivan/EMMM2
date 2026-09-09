use super::is_cancelled;
use super::progress::emit_throttled_progress;
use super::security::{
    validate_entry_path, validate_entry_type, ArchiveLimits, EntryKind, ExtractionBudget,
    OutputPathRegistry,
};
use super::types::{ArchiveAnalysis, ArchiveEntryInfo, ArchiveFormat, ExtractionEvent};
use crate::shared::errors::{AppError, ArchiveErrorKind};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use tauri::ipc::Channel;
use zip::result::ZipError;
use zip::{CompressionMethod, ZipArchive};

const MAX_ENTRIES: usize = 500;

pub(super) fn analyze_zip(
    archive_path: &Path,
    limits: ArchiveLimits,
    cancel_token: Option<Arc<AtomicBool>>,
) -> Result<ArchiveAnalysis, AppError> {
    let file_size_bytes = fs::metadata(archive_path)?.len();
    let file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file).map_err(map_zip_error)?;
    let mut budget = ExtractionBudget::new(limits, file_size_bytes);
    let entries = preflight_zip(&mut archive, &mut budget, &cancel_token, None, false)?;
    let mut root_dirs = HashSet::new();
    let mut file_count = 0_usize;
    let mut has_ini = false;
    let mut uncompressed_size = 0_u64;
    let mut contains_nested_archives = false;
    let mut is_encrypted = false;
    let mut analysis_entries = Vec::new();

    for entry in entries {
        file_count = file_count
            .checked_add(1)
            .ok_or_else(|| AppError::Validation("Archive entry count overflow".to_string()))?;
        uncompressed_size = uncompressed_size
            .checked_add(entry.size)
            .ok_or_else(|| AppError::Validation("Archive size overflow".to_string()))?;
        has_ini |= entry.name.to_ascii_lowercase().ends_with(".ini");
        contains_nested_archives |= is_nested_archive_name(&entry.name);
        is_encrypted |= entry.encrypted;

        if analysis_entries.len() < MAX_ENTRIES {
            analysis_entries.push(ArchiveEntryInfo {
                path: entry.name.clone(),
                is_dir: entry.kind == EntryKind::Directory,
                size: entry.size,
            });
        }

        if let Some(root) = entry.name.replace('\\', "/").split('/').next() {
            if !root.is_empty() {
                root_dirs.insert(root.to_string());
            }
        }
    }

    Ok(ArchiveAnalysis {
        format: ArchiveFormat::Zip,
        file_count,
        has_ini,
        uncompressed_size,
        file_size_bytes,
        single_root_folder: (root_dirs.len() == 1)
            .then(|| root_dirs.into_iter().next())
            .flatten(),
        is_encrypted,
        contains_nested_archives,
        entries: analysis_entries,
    })
}

pub(super) fn extract_zip_to_dir(
    archive_path: &Path,
    dest_path: &Path,
    password: Option<&str>,
    cancel_token: Option<Arc<AtomicBool>>,
    on_progress: Option<&Channel<ExtractionEvent>>,
    budget: &mut ExtractionBudget,
) -> Result<usize, AppError> {
    let file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file).map_err(map_zip_error)?;
    let mut preflight_budget = budget.preflight();
    let entries = preflight_zip(
        &mut archive,
        &mut preflight_budget,
        &cancel_token,
        password,
        true,
    )?;
    let root = dest_path.canonicalize()?;
    let total_entries = entries.len();
    let mut files_extracted = 0_usize;
    let mut last_progress = Instant::now();
    let mut last_file_name = String::new();
    let mut buffer = [0_u8; 64 * 1024];

    for entry in entries {
        if is_cancelled(&cancel_token) {
            return Err(AppError::Cancelled);
        }

        let output_path = root.join(&entry.relative_path);
        budget.start_entry(entry.size)?;
        if entry.kind == EntryKind::Directory {
            fs::create_dir_all(&output_path)?;
            continue;
        }

        let parent = output_path
            .parent()
            .ok_or_else(|| AppError::Security("Archive output has no parent".to_string()))?;
        fs::create_dir_all(parent)?;
        if !parent.canonicalize()?.starts_with(&root) {
            return Err(AppError::Security(
                "Archive output escaped the extraction root".to_string(),
            ));
        }

        let mut source = if entry.encrypted {
            let password = password.ok_or(AppError::ArchivePasswordRequired)?;
            archive
                .by_index_decrypt(entry.index, password.as_bytes())
                .map_err(map_zip_error)?
        } else {
            archive.by_index(entry.index).map_err(map_zip_error)?
        };
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output_path)?;
        loop {
            if is_cancelled(&cancel_token) {
                return Err(AppError::Cancelled);
            }
            let bytes_read = source.read(&mut buffer).map_err(map_zip_io_error)?;
            if bytes_read == 0 {
                break;
            }
            budget.consume_chunk(bytes_read, &cancel_token)?;
            output.write_all(&buffer[..bytes_read])?;
        }
        output.flush()?;
        files_extracted += 1;
        last_file_name = output_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or(entry.name);
        if let Some(channel) = on_progress {
            emit_throttled_progress(
                channel,
                &mut last_progress,
                last_file_name.clone(),
                files_extracted,
                total_entries,
            );
        }
    }

    if let Some(channel) = on_progress {
        if files_extracted > 0 {
            emit_throttled_progress(
                channel,
                &mut last_progress,
                last_file_name,
                files_extracted,
                total_entries,
            );
        }
    }
    Ok(files_extracted)
}

struct ZipEntry {
    index: usize,
    name: String,
    relative_path: PathBuf,
    kind: EntryKind,
    size: u64,
    encrypted: bool,
}

fn preflight_zip(
    archive: &mut ZipArchive<fs::File>,
    budget: &mut ExtractionBudget,
    cancel_token: &Option<Arc<AtomicBool>>,
    password: Option<&str>,
    validate_decoder: bool,
) -> Result<Vec<ZipEntry>, AppError> {
    let mut entries = Vec::with_capacity(archive.len());
    let mut output_paths = OutputPathRegistry::default();

    for index in 0..archive.len() {
        if is_cancelled(cancel_token) {
            return Err(AppError::Cancelled);
        }
        let raw = archive.by_index_raw(index).map_err(map_zip_error)?;
        let name = raw.name().to_string();
        let relative_path = raw.enclosed_name().ok_or_else(|| {
            AppError::Security("Archive entry path escapes the extraction root".to_string())
        })?;
        let validated_path = validate_entry_path(Path::new(""), &name)?;
        if relative_path != validated_path {
            return Err(AppError::Security(
                "Archive entry path is not normalized safely".to_string(),
            ));
        }
        let kind = validate_zip_entry_type(&raw)?;
        output_paths.register(&relative_path, kind)?;
        let compression = raw.compression();
        let encrypted = raw.encrypted();
        let size = raw.size();
        drop(raw);

        ensure_supported_compression(compression)?;
        budget.start_entry(size)?;
        budget.consume_bytes(size, cancel_token)?;
        if validate_decoder {
            if encrypted {
                let password = password.ok_or(AppError::ArchivePasswordRequired)?;
                archive
                    .by_index_decrypt(index, password.as_bytes())
                    .map_err(map_zip_error)?;
            } else {
                archive.by_index(index).map_err(map_zip_error)?;
            }
        }
        entries.push(ZipEntry {
            index,
            name,
            relative_path,
            kind,
            size,
            encrypted,
        });
    }
    Ok(entries)
}

fn validate_zip_entry_type(entry: &zip::read::ZipFile<'_>) -> Result<EntryKind, AppError> {
    if entry.is_symlink() {
        return Err(AppError::Security(
            "Archive links and special-file entries are not allowed".to_string(),
        ));
    }
    let fallback_mode = if entry.is_dir() { 0o040755 } else { 0o100644 };
    validate_entry_type(entry.unix_mode().unwrap_or(fallback_mode), 1)
}

fn ensure_supported_compression(method: CompressionMethod) -> Result<(), AppError> {
    match method {
        CompressionMethod::Stored
        | CompressionMethod::Deflated
        | CompressionMethod::Deflate64
        | CompressionMethod::Bzip2
        | CompressionMethod::Aes
        | CompressionMethod::Zstd
        | CompressionMethod::Lzma
        | CompressionMethod::Xz => Ok(()),
        _ => Err(AppError::ArchiveUnsupported {
            reason: ArchiveErrorKind::UnsupportedCompression,
        }),
    }
}

fn map_zip_error(error: ZipError) -> AppError {
    match error {
        ZipError::InvalidPassword => AppError::ArchivePasswordIncorrect,
        ZipError::UnsupportedArchive(ZipError::PASSWORD_REQUIRED) => AppError::ArchivePasswordRequired,
        ZipError::UnsupportedArchive(_) => AppError::ArchiveUnsupported {
            reason: ArchiveErrorKind::UnsupportedCompression,
        },
        ZipError::InvalidArchive(_) => AppError::Validation("ZIP archive is corrupt".to_string()),
        ZipError::Io(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            AppError::Validation("ZIP archive is corrupt".to_string())
        }
        ZipError::Io(error) => AppError::Io(error.to_string()),
        ZipError::FileNotFound => AppError::Io("ZIP entry was not found".to_string()),
        _ => AppError::Io("ZIP reader failed unexpectedly".to_string()),
    }
}

fn map_zip_io_error(error: std::io::Error) -> AppError {
    if error.kind() == std::io::ErrorKind::InvalidData {
        return AppError::Validation("ZIP archive is corrupt".to_string());
    }
    AppError::Io(error.to_string())
}

fn is_nested_archive_name(name: &str) -> bool {
    matches!(
        name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase().as_str(),
        "zip" | "rar" | "7z"
    )
}

#[cfg(test)]
mod tests {
    use super::{analyze_zip, ensure_supported_compression, extract_zip_to_dir, ArchiveLimits, ExtractionBudget};
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;
    use zip::CompressionMethod;

    #[test]
    fn supports_every_configured_zip_compression_method() {
        for method in [
            CompressionMethod::Stored,
            CompressionMethod::Deflated,
            CompressionMethod::Deflate64,
            CompressionMethod::Bzip2,
            CompressionMethod::Aes,
            CompressionMethod::Zstd,
            CompressionMethod::Lzma,
            CompressionMethod::Xz,
        ] {
            assert!(ensure_supported_compression(method).is_ok(), "{method:?}");
        }
    }

    #[test]
    fn extracts_every_generated_configured_compression_method() {
        let dir = TempDir::new().unwrap();
        for method in [
            CompressionMethod::Stored,
            CompressionMethod::Deflated,
            CompressionMethod::Bzip2,
            CompressionMethod::Zstd,
        ] {
            let archive_path = dir.path().join(format!("{method:?}.zip"));
            let file = fs::File::create(&archive_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default().compression_method(method);
            writer.start_file("Mod/config.ini", options).unwrap();
            writer.write_all(b"[TextureOverride]\n").unwrap();
            writer.finish().unwrap();

            let analysis = analyze_zip(&archive_path, ArchiveLimits::default(), None).unwrap();
            assert_eq!(analysis.file_count, 1, "{method:?}");
            let destination = dir.path().join(format!("{method:?}-out"));
            fs::create_dir_all(&destination).unwrap();
            let mut budget = ExtractionBudget::new(
                ArchiveLimits::default(),
                fs::metadata(&archive_path).unwrap().len(),
            );
            let extracted = extract_zip_to_dir(
                &archive_path,
                &destination,
                None,
                None,
                None,
                &mut budget,
            )
            .unwrap();
            assert_eq!(extracted, 1, "{method:?}");
            assert_eq!(
                fs::read(destination.join("Mod/config.ini")).unwrap(),
                b"[TextureOverride]\n",
                "{method:?}"
            );
        }
    }

    #[test]
    fn extracts_aes_zip_with_the_supplied_password() {
        let dir = TempDir::new().unwrap();
        let archive_path = dir.path().join("aes.zip");
        let file = fs::File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .with_aes_encryption(zip::AesMode::Aes256, "secret");
        writer.start_file("Mod/config.ini", options).unwrap();
        writer.write_all(b"[TextureOverride]\n").unwrap();
        writer.finish().unwrap();

        let analysis = analyze_zip(&archive_path, ArchiveLimits::default(), None).unwrap();
        assert!(analysis.is_encrypted);
        let destination = dir.path().join("aes-out");
        fs::create_dir_all(&destination).unwrap();
        let mut budget = ExtractionBudget::new(
            ArchiveLimits::default(),
            fs::metadata(&archive_path).unwrap().len(),
        );
        let extracted = extract_zip_to_dir(
            &archive_path,
            &destination,
            Some("secret"),
            None,
            None,
            &mut budget,
        )
        .unwrap();

        assert_eq!(extracted, 1);
        assert_eq!(
            fs::read(destination.join("Mod/config.ini")).unwrap(),
            b"[TextureOverride]\n"
        );
    }
}
