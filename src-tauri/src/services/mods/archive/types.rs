use serde::{Deserialize, Serialize};
use std::path::Path;

/// Progress events streamed to frontend during archive extraction via `Channel<ExtractionEvent>`.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ExtractionEvent {
    /// One file has been extracted (or skipped as directory).
    #[serde(rename_all = "camelCase")]
    FileProgress {
        file_name: String,
        file_index: usize,
        total_files: usize,
    },
}

/// Supported archive format.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ArchiveFormat {
    Zip,
    SevenZ,
    Rar,
}

impl ArchiveFormat {
    /// Detect format from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntryInfo {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

/// Result of analyzing an archive before extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveAnalysis {
    pub format: ArchiveFormat,
    pub file_count: usize,
    pub has_ini: bool,
    pub uncompressed_size: u64,
    /// The physical size of the archive file on disk.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub archive_name: String,
    /// Paths to all extracted mod root folders
    pub dest_paths: Vec<String>,
    pub files_extracted: usize,
    /// Number of independent mod roots found and moved.
    pub mod_count: usize,
    pub success: bool,
    pub error: Option<String>,
    pub aborted: bool,
}
