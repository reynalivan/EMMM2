use serde::{Deserialize, Serialize};
use std::path::Path;

/// Progress events streamed to frontend during archive extraction via `Channel<ExtractionEvent>`.
#[derive(Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ExtractionEvent {
    /// One file has been extracted (or skipped as directory).
    #[serde(rename_all = "camelCase")]
    FileProgress {
        file_name: String,
        #[specta(type = f64)]
        file_index: usize,
        #[specta(type = f64)]
        total_files: usize,
    },
}

/// Supported archive format.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
pub enum ArchiveFormat {
    Zip,
    SevenZ,
    Rar,
}

impl ArchiveFormat {
    /// Detect format from file magic bytes, falling back to extension.
    pub fn detect(path: &Path) -> Option<Self> {
        let mut header = [0u8; 8];
        if let Ok(mut file) = std::fs::File::open(path) {
            use std::io::Read;
            if let Ok(bytes_read) = file.read(&mut header) {
                if bytes_read >= 4 && &header[0..4] == &[0x50, 0x4B, 0x03, 0x04] {
                    return Some(Self::Zip);
                }
                if bytes_read >= 6 && &header[0..6] == &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C] {
                    return Some(Self::SevenZ);
                }
                if bytes_read >= 7 && &header[0..7] == &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x00] {
                    return Some(Self::Rar);
                }
                // RAR 5+ magic signature
                if bytes_read >= 8
                    && &header[0..8] == &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00]
                {
                    return Some(Self::Rar);
                }
            }
        }

        // Fallback to extension check
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "zip" => Some(Self::Zip),
            "7z" => Some(Self::SevenZ),
            "rar" => Some(Self::Rar),
            _ => None,
        }
    }
}

/// A single entry in an archive (for file tree preview).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ArchiveEntryInfo {
    pub path: String,
    pub is_dir: bool,
    #[specta(type = f64)]
    pub size: u64,
}

/// Result of analyzing an archive before extraction.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ArchiveAnalysis {
    pub format: ArchiveFormat,
    #[specta(type = f64)]
    pub file_count: usize,
    pub has_ini: bool,
    #[specta(type = f64)]
    pub uncompressed_size: u64,
    /// The physical size of the archive file on disk.
    #[specta(type = f64)]
    pub file_size_bytes: u64,
    pub single_root_folder: Option<String>,
    /// Whether the archive requires a password for extraction.
    pub is_encrypted: bool,
    /// Whether the archive contains other archives (e.g. .zip, .rar, .7z) inside it.
    pub contains_nested_archives: bool,
    /// Top entries for file tree preview (capped at 500).
    pub entries: Vec<ArchiveEntryInfo>,
}

/// Result of an extraction operation.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionResult {
    pub archive_name: String,
    /// Paths to all extracted mod root folders
    pub dest_paths: Vec<String>,
    #[specta(type = f64)]
    pub files_extracted: usize,
    /// Number of independent mod roots found and moved.
    #[specta(type = f64)]
    pub mod_count: usize,
    pub success: bool,
    pub error: Option<String>,
    pub aborted: bool,
    pub collisions: Vec<crate::services::scanner::core::types::CollisionInfo>,
}
