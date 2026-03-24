//! File system walker for mod directory scanning.
//! Uses `walkdir` crate for efficient recursive traversal per TRD §3.2.

use serde::{Deserialize, Serialize};
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
    /// Whether the folder has the `DISABLED ` prefix or is inside a disabled parent.
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
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ArchiveInfo {
    pub path: String,
    pub name: String,
    pub extension: String,
    #[specta(type = f64)]
    pub size_bytes: u64,
    pub has_ini: bool,
    #[specta(type = f64)]
    pub file_count: usize,
    /// Whether the archive requires a password for extraction.
    pub is_encrypted: bool,
    /// Whether the archive contains other archives (e.g. .zip, .rar, .7z).
    pub contains_nested_archives: bool,
}

/// Valid archive extensions we support.
const ARCHIVE_EXTENSIONS: &[&str] = &["zip", "7z", "rar"];

/// Extensions to check during content scanning per Epic 2 §B.2.
const SCAN_EXTENSIONS: &[&str] = &["ini", "dds", "txt", "buf", "ib", "vb"];

/// Scan the root Mods directory and return real mod folders as candidates.
///
/// Recurses up to a maximum depth of 6, using `classifier::classify_folder` to determine
/// if a path is a true mod (e.g. `ModPackRoot`, `FlatModRoot`, `VariantContainer`) or a container.
///
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

    use crate::services::explorer::classifier::{self, NodeType};

    let mut candidates = Vec::new();

    let mut it = WalkDir::new(mods_path)
        .min_depth(1)
        .max_depth(8)
        .follow_links(false)
        .into_iter();

    loop {
        let entry = match it.next() {
            None => break,
            Some(Ok(e)) => e,
            Some(Err(e)) => {
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

        // Skip hidden/system folders (.temp_extract, .extracted, .archive_backup, etc.)
        if raw_name.starts_with('.') {
            if entry.file_type().is_dir() {
                it.skip_current_dir();
            }
            continue;
        }

        let (node_type, _, _) = classifier::classify_folder(path);

        let is_disabled = path.components().any(|comp| {
            let comp_name = comp.as_os_str().to_string_lossy();
            normalizer::is_disabled_folder(&comp_name)
        });

        match node_type {
            NodeType::ModPackRoot | NodeType::FlatModRoot | NodeType::VariantContainer => {
                let display_name = normalizer::normalize_display_name(&raw_name);
                candidates.push(ModCandidate {
                    path: path.to_path_buf(),
                    raw_name,
                    display_name,
                    is_disabled,
                });

                // Do not recursively traverse inside actual mod folders
                it.skip_current_dir();
            }
            NodeType::ContainerFolder => {
                // Keep recursing deeper to find the actual mods inside
            }
            NodeType::InternalAssets => {
                // Internal assets should not be recursed into to find mods
                it.skip_current_dir();
            }
        }
    }

    Ok(candidates)
}

/// Scan specific subset of folders directly and deeply.
/// Useful for drag-and-drop operations on explicit folders.
pub fn scan_specific_folders(paths: &[PathBuf]) -> Result<Vec<ModCandidate>, String> {
    use crate::services::explorer::classifier::{self, NodeType};

    let mut candidates = Vec::new();

    for start_path in paths {
        if !start_path.is_dir() {
            continue;
        }

        let mut it = WalkDir::new(start_path)
            .min_depth(0) // Check the dropped path itself
            .max_depth(8)
            .follow_links(false)
            .into_iter();

        loop {
            let entry = match it.next() {
                None => break,
                Some(Ok(e)) => e,
                Some(Err(e)) => {
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

            if raw_name.starts_with('.') {
                if entry.file_type().is_dir() {
                    it.skip_current_dir();
                }
                continue;
            }

            let (node_type, _, _) = classifier::classify_folder(path);

            let is_disabled = path.components().any(|comp| {
                let comp_name = comp.as_os_str().to_string_lossy();
                normalizer::is_disabled_folder(&comp_name)
            });

            match node_type {
                NodeType::ModPackRoot | NodeType::FlatModRoot | NodeType::VariantContainer => {
                    let display_name = normalizer::normalize_display_name(&raw_name);
                    candidates.push(ModCandidate {
                        path: path.to_path_buf(),
                        raw_name,
                        display_name,
                        is_disabled,
                    });

                    it.skip_current_dir();
                }
                NodeType::ContainerFolder => {
                    // Dive in
                }
                NodeType::InternalAssets => {
                    it.skip_current_dir();
                }
            }
        }
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
            has_ini: false, // Determined later during pre-extraction analysis
            file_count: 0,
            is_encrypted: false,
            contains_nested_archives: false,
        });
    }

    Ok(archives)
}

#[cfg(test)]
#[path = "tests/walker_tests.rs"]
mod tests;
