use super::is_cancelled;
use super::types::ArchiveFormat;
use crate::shared::errors::{AppError, ArchiveErrorKind};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

const FILE_TYPE_MASK: u32 = 0o170000;
const REGULAR_FILE: u32 = 0o100000;
const DIRECTORY: u32 = 0o040000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    pub max_entries: usize,
    pub max_single_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_compression_ratio: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_single_file_bytes: 2 * 1024 * 1024 * 1024,
            max_total_bytes: 20 * 1024 * 1024 * 1024,
            max_compression_ratio: 1_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntryKind {
    File,
    Directory,
}

/// Tracks archive output names using Windows' case-insensitive semantics.
#[derive(Default)]
pub(super) struct OutputPathRegistry {
    paths: HashSet<String>,
    file_paths: HashSet<String>,
    paths_with_descendants: HashSet<String>,
}

impl OutputPathRegistry {
    pub(super) fn register(&mut self, path: &Path, kind: EntryKind) -> Result<(), AppError> {
        let components = path
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect::<Vec<_>>();
        let key = components.join("\\");
        if key.is_empty()
            || self.paths.contains(&key)
            || (kind == EntryKind::File && self.paths_with_descendants.contains(&key))
        {
            return Err(AppError::Validation(
                "Archive contains colliding output paths".to_string(),
            ));
        }

        let mut ancestor = String::new();
        for component in components.iter().take(components.len().saturating_sub(1)) {
            if !ancestor.is_empty() {
                ancestor.push('\\');
            }
            ancestor.push_str(component);
            if self.file_paths.contains(&ancestor) {
                return Err(AppError::Validation(
                    "Archive contains a file/directory output collision".to_string(),
                ));
            }
            self.paths_with_descendants.insert(ancestor.clone());
        }

        self.paths.insert(key.clone());
        if kind == EntryKind::File {
            self.file_paths.insert(key);
        }
        Ok(())
    }
}

pub(super) fn validate_entry_type(mode: u32, link_count: u64) -> Result<EntryKind, AppError> {
    if link_count > 1 {
        return Err(AppError::Security(
            "Archive hard-link entries are not allowed".to_string(),
        ));
    }
    match mode & FILE_TYPE_MASK {
        REGULAR_FILE => Ok(EntryKind::File),
        DIRECTORY => Ok(EntryKind::Directory),
        _ => Err(AppError::Security(
            "Archive links and special-file entries are not allowed".to_string(),
        )),
    }
}

pub(super) fn validate_entry_path(root: &Path, entry_name: &str) -> Result<PathBuf, AppError> {
    if entry_name.is_empty()
        || entry_name.contains('\0')
        || entry_name.starts_with('/')
        || entry_name.starts_with('\\')
    {
        return Err(unsafe_path(entry_name));
    }

    let mut output = root.to_path_buf();
    let mut component_count = 0_usize;
    for component in entry_name.split(['/', '\\']) {
        component_count += 1;
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.contains(':')
            || component
                .chars()
                .any(|character| matches!(character, '<' | '>' | '"' | '|' | '?' | '*'))
            || component.ends_with('.')
            || component.ends_with(' ')
            || is_reserved_windows_name(component)
        {
            return Err(unsafe_path(entry_name));
        }
        output.push(component);
    }

    if component_count == 0 || !output.starts_with(root) {
        return Err(unsafe_path(entry_name));
    }
    Ok(output)
}

fn is_reserved_windows_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem.strip_prefix("COM").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || stem.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
}

fn unsafe_path(entry_name: &str) -> AppError {
    AppError::Security(format!("Unsafe archive entry path: {entry_name:?}"))
}

pub(super) fn reject_multi_volume_archive(path: &Path) -> Result<(), AppError> {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let numbered_volume = extension.len() == 3
        && (extension.chars().all(|value| value.is_ascii_digit())
            || ((extension.starts_with('z') || extension.starts_with('r'))
                && extension[1..].chars().all(|value| value.is_ascii_digit())));

    if file_name.contains(".part") || numbered_volume {
        return Err(AppError::Validation(
            "Multi-volume archives not supported".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_archive_signature(
    archive_path: &Path,
    format: ArchiveFormat,
) -> Result<(), AppError> {
    let mut file = fs::File::open(archive_path)?;
    let mut header = [0_u8; 8];
    let bytes_read = file.read(&mut header)?;
    let valid = match format {
        // ZIP accepts empty archives, data descriptors, and self-extracting
        // payloads. `zip::ZipArchive` validates the central directory before
        // any entry can be written.
        ArchiveFormat::Zip => true,
        ArchiveFormat::SevenZ => {
            bytes_read >= 6 && header[..6] == [0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]
        }
        ArchiveFormat::Rar => {
            (bytes_read >= 7 && header[..7] == [0x52, 0x61, 0x72, 0x21, 0x1a, 0x07, 0x00])
                || (bytes_read >= 8 && header == [0x52, 0x61, 0x72, 0x21, 0x1a, 0x07, 0x01, 0x00])
        }
    };
    if valid {
        return Ok(());
    }
    Err(AppError::Validation(format!(
        "Unsupported or invalid {format:?} archive"
    )))
}

pub(super) fn map_archive_error(error: compress_tools::Error, password_supplied: bool) -> AppError {
    match error {
        compress_tools::Error::UnsupportedZipCompression(_) => AppError::ArchiveUnsupported {
            reason: ArchiveErrorKind::UnsupportedCompression,
        },
        compress_tools::Error::Io(error) => AppError::Io(error.to_string()),
        compress_tools::Error::Extraction { details, .. } => {
            map_archive_extraction_details(&details, password_supplied)
        }
        compress_tools::Error::Encoding(error) => AppError::Validation(error.into_owned()),
        compress_tools::Error::Utf(error) => AppError::Validation(error.to_string()),
        compress_tools::Error::NullArchive | compress_tools::Error::Unknown => {
            AppError::Io("Archive reader could not initialize".to_string())
        }
        _ => AppError::Io("Archive reader failed unexpectedly".to_string()),
    }
}

fn map_archive_extraction_details(message: &str, password_supplied: bool) -> AppError {
    let lower = message.to_ascii_lowercase();
    let mentions_encryption = lower.contains("password")
        || lower.contains("passphrase")
        || lower.contains("decrypt")
        || lower.contains("encrypted");
    if lower.contains("declared dictionary size") && lower.contains("not supported") {
        log::warn!("Archive compression dictionary is unsupported: {message}");
        return AppError::ArchiveUnsupported {
            reason: ArchiveErrorKind::DictionaryTooLarge,
        };
    }
    if lower.contains("unsupported") || lower.contains("not supported") {
        if mentions_encryption {
            return AppError::Validation(format!(
                "Encrypted archive is not supported by the bundled extractor: {message}"
            ));
        }
        log::warn!("Archive compression is unsupported: {message}");
        return AppError::ArchiveUnsupported {
            reason: ArchiveErrorKind::UnsupportedCompression,
        };
    }
    if mentions_encryption {
        return if password_supplied {
            AppError::ArchivePasswordIncorrect
        } else {
            AppError::ArchivePasswordRequired
        };
    }
    AppError::Io(format!("Archive read failed: {message}"))
}

pub(super) struct ExtractionBudget {
    limits: ArchiveLimits,
    archive_size: u64,
    entries: usize,
    current_file_bytes: u64,
    total_bytes: u64,
}

impl ExtractionBudget {
    pub(super) fn new(limits: ArchiveLimits, archive_size: u64) -> Self {
        Self {
            limits,
            archive_size,
            entries: 0,
            current_file_bytes: 0,
            total_bytes: 0,
        }
    }

    pub(super) fn start_entry(&mut self, announced_size: u64) -> Result<(), AppError> {
        self.entries = self.entries.checked_add(1).ok_or_else(|| {
            AppError::Validation("Archive entry-count limit exceeded".to_string())
        })?;
        if self.entries > self.limits.max_entries {
            return Err(AppError::Validation(
                "Archive entry-count limit exceeded".to_string(),
            ));
        }
        self.current_file_bytes = 0;
        self.check_sizes(
            announced_size,
            self.total_bytes.saturating_add(announced_size),
        )
    }

    pub(super) fn consume_chunk(
        &mut self,
        chunk_size: usize,
        cancel_token: &Option<Arc<AtomicBool>>,
    ) -> Result<(), AppError> {
        let chunk_size = u64::try_from(chunk_size)
            .map_err(|_| AppError::Validation("Archive chunk size overflow".to_string()))?;
        self.consume_bytes(chunk_size, cancel_token)
    }

    pub(super) fn consume_bytes(
        &mut self,
        byte_count: u64,
        cancel_token: &Option<Arc<AtomicBool>>,
    ) -> Result<(), AppError> {
        if is_cancelled(cancel_token) {
            return Err(AppError::Cancelled);
        }
        let file_bytes = self
            .current_file_bytes
            .checked_add(byte_count)
            .ok_or_else(|| {
                AppError::Validation("Archive single-file byte limit exceeded".to_string())
            })?;
        let total_bytes = self.total_bytes.checked_add(byte_count).ok_or_else(|| {
            AppError::Validation("Archive total-bytes limit exceeded".to_string())
        })?;
        self.check_sizes(file_bytes, total_bytes)?;
        self.current_file_bytes = file_bytes;
        self.total_bytes = total_bytes;
        Ok(())
    }

    pub(super) fn preflight(&self) -> Self {
        Self::new(self.limits.clone(), self.archive_size)
    }

    fn check_sizes(&self, file_bytes: u64, total_bytes: u64) -> Result<(), AppError> {
        if file_bytes > self.limits.max_single_file_bytes {
            return Err(AppError::Validation(
                "Archive single-file byte limit exceeded".to_string(),
            ));
        }
        if total_bytes > self.limits.max_total_bytes {
            return Err(AppError::Validation(
                "Archive total-bytes limit exceeded".to_string(),
            ));
        }
        let ratio_limit = self
            .archive_size
            .saturating_mul(self.limits.max_compression_ratio);
        if total_bytes > ratio_limit {
            return Err(AppError::Validation(
                "Archive compression-ratio limit exceeded".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{map_archive_extraction_details, EntryKind, OutputPathRegistry};
    use crate::shared::errors::{AppError, ArchiveErrorKind};
    use std::path::Path;

    #[test]
    fn oversized_dictionary_is_mapped_without_exposing_os_details() {
        let error = map_archive_extraction_details(
            "Extraction error: OS Error 42 (FormatMessageW returned error 317) 'Declared dictionary size is not supported'",
            false,
        );

        assert_eq!(
            error.to_string(),
            "Archive extraction is unsupported: DictionaryTooLarge"
        );
        assert!(matches!(
            error,
            AppError::ArchiveUnsupported {
                reason: ArchiveErrorKind::DictionaryTooLarge
            }
        ));
    }

    #[test]
    fn unsupported_encrypted_archive_is_not_reported_as_an_incorrect_password() {
        let error = map_archive_extraction_details(
            "The archive header is encrypted, but currently not supported",
            true,
        );

        assert!(matches!(
            error,
            AppError::Validation(message) if message.contains("Encrypted archive is not supported")
        ));
    }

    #[test]
    fn output_registry_rejects_case_insensitive_collisions() {
        let mut registry = OutputPathRegistry::default();
        registry
            .register(Path::new("Robin/Config.ini"), EntryKind::File)
            .unwrap();

        assert!(matches!(
            registry.register(Path::new("robin/config.ini"), EntryKind::File),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn output_registry_rejects_a_file_as_a_directory_parent() {
        let mut registry = OutputPathRegistry::default();
        registry.register(Path::new("Robin"), EntryKind::File).unwrap();

        assert!(matches!(
            registry.register(Path::new("Robin/Config.ini"), EntryKind::File),
            Err(AppError::Validation(_))
        ));
    }
}
