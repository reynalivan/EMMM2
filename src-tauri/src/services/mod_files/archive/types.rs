use serde::{Deserialize, Serialize};
use std::path::Path;

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

/// Result of analyzing an archive before extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveAnalysis {
    pub format: ArchiveFormat,
    pub file_count: usize,
    pub has_ini: bool,
    pub uncompressed_size: u64,
    pub single_root_folder: Option<String>,
}

/// Result of an extraction operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub archive_name: String,
    pub dest_path: String,
    pub files_extracted: usize,
    pub success: bool,
    pub error: Option<String>,
}
