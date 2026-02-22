//! File system walker for mod directory scanning.
//! Uses `walkdir` crate for efficient recursive traversal per TRD §3.2.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::normalizer;

/// Represents a mod folder candidate discovered during scanning.
#[derive(Debug, Clone)]
pub struct ModCandidate {
    /// Absolute path to the mod folder.
    pub path: PathBuf,
    /// Raw folder name as-is from the filesystem.
    pub raw_name: String,
    /// Clean display name (without DISABLED prefix).
    pub display_name: String,
    /// Whether the folder has the `DISABLED ` prefix.
    pub is_disabled: bool,
}

/// Metadata about a single file found during content scanning.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub extension: String,
}

/// Content analysis of a mod folder's internals.
#[derive(Debug, Clone)]
pub struct FolderContent {
    /// Subfolder names found within the mod folder.
    pub subfolder_names: Vec<String>,
    /// All files found (with metadata).
    pub files: Vec<FileInfo>,
    /// `.ini` files specifically (for conflict detection).
    pub ini_files: Vec<PathBuf>,
}

/// Info about a detected archive file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArchiveInfo {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
    /// Whether the archive contains at least one `.ini` file.
    pub has_ini: Option<bool>,
}

/// Valid archive extensions we support.
const ARCHIVE_EXTENSIONS: &[&str] = &["zip", "7z", "rar"];

/// Extensions to check during content scanning per Epic 2 §B.2.
const SCAN_EXTENSIONS: &[&str] = &["ini", "dds", "txt", "buf", "ib", "vb"];

/// Scan the root Mods directory and return all immediate child folders as candidates.
///
/// Filters out non-directory entries and archive files.
/// # Covers: TC-2.3-01 (Folder listing)
pub fn scan_mod_folders(mods_path: &Path) -> Result<Vec<ModCandidate>, String> {
    if !mods_path.exists() {
        return Err(format!("Mods path does not exist: {}", mods_path.display()));
    }

    if !mods_path.is_dir() {
        return Err(format!(
            "Mods path is not a directory: {}",
            mods_path.display()
        ));
    }

    let entries =
        std::fs::read_dir(mods_path).map_err(|e| format!("Failed to read mods directory: {e}"))?;

    let mut candidates = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log::warn!("Skipping unreadable entry: {e}");
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let raw_name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        let is_disabled = normalizer::is_disabled_folder(&raw_name);
        let display_name = normalizer::normalize_display_name(&raw_name);

        candidates.push(ModCandidate {
            path,
            raw_name,
            display_name,
            is_disabled,
        });
    }

    Ok(candidates)
}

/// Scan folder content recursively up to `max_depth` levels.
///
/// Returns subfolder names, file info, and `.ini` files for the pipeline.
/// # Covers: Epic 2 §B.2 Pipeline B (Deep Content Scan)
pub fn scan_folder_content(folder: &Path, max_depth: usize) -> FolderContent {
    let mut subfolder_names = Vec::new();
    let mut files = Vec::new();
    let mut ini_files = Vec::new();

    // WalkDir with max_depth, follow_links disabled (EC-2.02 symlink safety)
    let walker = WalkDir::new(folder)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip the root folder itself
        if path == folder {
            continue;
        }

        if entry.file_type().is_dir() {
            if let Some(name) = path.file_name() {
                subfolder_names.push(name.to_string_lossy().to_string());
            }
            continue;
        }

        // File entry
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // Track ini files specifically for conflict detection
        if extension == "ini" {
            ini_files.push(path.to_path_buf());
        }

        // Only include files with extensions we care about
        if SCAN_EXTENSIONS.contains(&extension.as_str()) {
            files.push(FileInfo {
                path: path.to_path_buf(),
                name,
                extension,
            });
        }
    }

    FolderContent {
        subfolder_names,
        files,
        ini_files,
    }
}

/// Detect archive files in the root mods directory.
///
/// # Covers: TC-2.1-01, TC-2.1-03
pub fn detect_archives(mods_path: &Path) -> Result<Vec<ArchiveInfo>, String> {
    if !mods_path.exists() {
        return Err(format!("Mods path does not exist: {}", mods_path.display()));
    }

    let entries =
        std::fs::read_dir(mods_path).map_err(|e| format!("Failed to read mods directory: {e}"))?;

    let mut archives = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let extension = match path.extension() {
            Some(ext) => ext.to_string_lossy().to_lowercase(),
            None => continue,
        };

        if !ARCHIVE_EXTENSIONS.contains(&extension.as_str()) {
            continue;
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);

        archives.push(ArchiveInfo {
            path: path.to_string_lossy().to_string(),
            name,
            extension,
            size_bytes,
            has_ini: None, // Determined later during pre-extraction analysis
        });
    }

    Ok(archives)
}

#[cfg(test)]
#[path = "tests/walker_tests.rs"]
mod tests;
