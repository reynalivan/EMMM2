//! Folder classification engine for the Navigable FolderGrid system.
//!
//! Classifies mod folders as one of:
//! - `ContainerFolder` — navigable, contains subfolders
//! - `ModPackRoot` — has valid 3DMigoto mod ini + assets
//! - `VariantContainer` — orchestrator with multiple variant subfolders
//! - `InternalAssets` — child folder referenced by parent's `filename=` directives
//!
//! # Covers: navigablefoldergrid.md §5

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::path_key::{canonical_name_key, names_equal_by_key, path_file_name_lossy};

/// The classification result for a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    ContainerFolder,
    ModPackRoot,
    VariantContainer,
    InternalAssets,
    FlatModRoot,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ContainerFolder => "ContainerFolder",
            Self::ModPackRoot => "ModPackRoot",
            Self::VariantContainer => "VariantContainer",
            Self::InternalAssets => "InternalAssets",
            Self::FlatModRoot => "FlatModRoot",
        }
    }
}

/// File extensions that indicate 3DMigoto mod assets.
const MOD_ASSET_EXTENSIONS: &[&str] = &["buf", "ib", "dds", "hlsl", "vb"];

/// Section prefixes that indicate a valid 3DMigoto mod ini.
const MOD_SECTION_PREFIXES: &[&str] = &["textureoverride", "shaderoverride", "resource"];

/// A folder with a root mod ini becomes a `VariantContainer` at this many
/// ini-bearing children — or at the lower bar when its ini names subfolders.
const VARIANT_CONTAINER_MIN_CHILDREN: usize = 3;
const VARIANT_CONTAINER_MIN_CHILDREN_REFERENCED: usize = 2;

/// One directory pass: the folder's mod ini candidates, child dirs and asset presence.
struct FolderScan {
    /// `.ini` files directly inside the folder, `desktop.ini` excluded.
    ini_files: Vec<PathBuf>,
    child_dirs: Vec<PathBuf>,
    has_assets: bool,
}

/// An inspection failure that cannot be treated as an empty/non-mod folder
/// while building an authoritative disk projection.
#[derive(Debug)]
pub enum ClassificationError {
    ReadDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    ReadDirectoryEntry {
        path: PathBuf,
        source: std::io::Error,
    },
    ReadEntryMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ReadIni {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidIniEncoding {
        path: PathBuf,
    },
}

impl fmt::Display for ClassificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadDirectory { path, source } => {
                write!(
                    formatter,
                    "Failed to read directory '{}': {source}",
                    path.display()
                )
            }
            Self::ReadDirectoryEntry { path, source } => write!(
                formatter,
                "Failed to read a directory entry in '{}': {source}",
                path.display()
            ),
            Self::ReadEntryMetadata { path, source } => write!(
                formatter,
                "Failed to inspect directory entry '{}': {source}",
                path.display()
            ),
            Self::ReadIni { path, source } => {
                write!(
                    formatter,
                    "Failed to read INI file '{}': {source}",
                    path.display()
                )
            }
            Self::InvalidIniEncoding { path } => write!(
                formatter,
                "Unsupported INI encoding in '{}'; use UTF-8 or Shift-JIS",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ClassificationError {}

fn scan_folder(path: &Path) -> Option<FolderScan> {
    let entries = fs::read_dir(path).ok()?;

    let mut scan = FolderScan {
        ini_files: Vec::new(),
        child_dirs: Vec::new(),
        has_assets: false,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        // `file_type()` comes free from the directory read; `is_dir()`/`is_file()`
        // would each cost a fresh stat, and this runs per entry of every folder.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let p = entry.path();
        if file_type.is_dir() {
            let fname = path_file_name_lossy(&p).unwrap_or_default();
            if !fname.starts_with('.') {
                scan.child_dirs.push(p);
            }
        } else if file_type.is_file() {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();

            if ext == "ini" {
                let fname = path_file_name_lossy(&p).unwrap_or_default();
                if !names_equal_by_key(&fname, "desktop.ini") {
                    scan.ini_files.push(p);
                }
            } else if !scan.has_assets && MOD_ASSET_EXTENSIONS.contains(&ext.as_str()) {
                scan.has_assets = true;
            }
        }
    }

    Some(scan)
}

fn scan_folder_strict(path: &Path) -> Result<FolderScan, ClassificationError> {
    let entries = fs::read_dir(path).map_err(|source| ClassificationError::ReadDirectory {
        path: path.to_path_buf(),
        source,
    })?;
    let mut scan = FolderScan {
        ini_files: Vec::new(),
        child_dirs: Vec::new(),
        has_assets: false,
    };

    for entry in entries {
        let entry = entry.map_err(|source| ClassificationError::ReadDirectoryEntry {
            path: path.to_path_buf(),
            source,
        })?;
        let entry_path = entry.path();
        let file_type =
            entry
                .file_type()
                .map_err(|source| ClassificationError::ReadEntryMetadata {
                    path: entry_path.clone(),
                    source,
                })?;
        if file_type.is_dir() {
            let fname = path_file_name_lossy(&entry_path).unwrap_or_default();
            if !fname.starts_with('.') {
                scan.child_dirs.push(entry_path);
            }
        } else if file_type.is_file() {
            let ext = entry_path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if ext == "ini" {
                let fname = path_file_name_lossy(&entry_path).unwrap_or_default();
                if !names_equal_by_key(&fname, "desktop.ini") {
                    scan.ini_files.push(entry_path);
                }
            } else if !scan.has_assets && MOD_ASSET_EXTENSIONS.contains(&ext.as_str()) {
                scan.has_assets = true;
            }
        }
    }

    Ok(scan)
}

/// What a single `.ini` file inside a mod folder contributes to classification.
struct IniScan {
    is_mod: bool,
    /// A 0 KB ini counts as a mod ini, but is reported as corrupt.
    is_corrupt: bool,
    referenced_subs: Vec<String>,
}

fn scan_ini_file(path: &Path) -> IniScan {
    let mut scan = IniScan {
        is_mod: false,
        is_corrupt: false,
        referenced_subs: Vec::new(),
    };

    if fs::metadata(path).map(|m| m.len() == 0).unwrap_or(false) {
        scan.is_mod = true;
        scan.is_corrupt = true;
        return scan;
    }

    let Ok(bytes) = fs::read(path) else {
        return scan;
    };
    let (content, _, _) = crate::services::ini::document::decode_ini_bytes(&bytes);

    let (is_mod, referenced_subs) = scan_ini_content(&content);
    scan.is_mod = is_mod;
    scan.referenced_subs = referenced_subs;
    scan
}

fn scan_ini_file_strict(path: &Path) -> Result<IniScan, ClassificationError> {
    if fs::metadata(path)
        .map_err(|source| ClassificationError::ReadIni {
            path: path.to_path_buf(),
            source,
        })?
        .len()
        == 0
    {
        return Ok(IniScan {
            is_mod: true,
            is_corrupt: true,
            referenced_subs: Vec::new(),
        });
    }

    let bytes = fs::read(path).map_err(|source| ClassificationError::ReadIni {
        path: path.to_path_buf(),
        source,
    })?;
    let (content, _, clean) = crate::services::ini::document::decode_ini_bytes(&bytes);
    if !clean {
        return Err(ClassificationError::InvalidIniEncoding {
            path: path.to_path_buf(),
        });
    }
    let (is_mod, referenced_subs) = scan_ini_content(&content);
    Ok(IniScan {
        is_mod,
        is_corrupt: false,
        referenced_subs,
    })
}

fn scan_folder_for_mode(
    path: &Path,
    strict: bool,
) -> Result<Option<FolderScan>, ClassificationError> {
    if strict {
        scan_folder_strict(path).map(Some)
    } else {
        Ok(scan_folder(path))
    }
}

fn scan_ini_file_for_mode(path: &Path, strict: bool) -> Result<IniScan, ClassificationError> {
    if strict {
        scan_ini_file_strict(path)
    } else {
        Ok(scan_ini_file(path))
    }
}

/// Returns the node type, a list of diagnostic reasons, and a list of warnings.
fn classify_folder_with_mode(
    path: &Path,
    strict: bool,
) -> Result<(NodeType, Vec<String>, Vec<String>), ClassificationError> {
    if !path.is_dir() {
        return Ok((NodeType::ContainerFolder, vec![], vec![]));
    }

    let Some(FolderScan {
        ini_files,
        child_dirs,
        has_assets,
    }) = scan_folder_for_mode(path, strict)?
    else {
        return Ok((NodeType::ContainerFolder, vec![], vec![]));
    };

    // Scan ini files for mod sections and referenced subfolders
    let mut has_mod_ini = false;
    let mut reasons: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut referenced_subs: Vec<String> = Vec::new();

    for ini_path in &ini_files {
        let fname = path_file_name_lossy(ini_path).unwrap_or_default();
        let scan = scan_ini_file_for_mode(ini_path, strict)?;

        if scan.is_corrupt {
            has_mod_ini = true;
            warnings.push(format!("[WARNING] Corrupt INI file: {} (0 KB)", fname));
            reasons.push(format!("Corrupt Mod ini: {fname}"));
            continue;
        }

        if scan.is_mod {
            has_mod_ini = true;
            reasons.push(format!("Mod ini: {fname}"));
        }
        referenced_subs.extend(scan.referenced_subs);
    }

    if !referenced_subs.is_empty() {
        reasons.push(format!(
            "References subfolders: {}",
            referenced_subs.join(", ")
        ));
    }

    // Each `has_any_mod_ini` is a full child scan plus an ini read, so only run
    // it when a root ini makes the answer reachable, and stop at the threshold
    // the check below compares against.
    let child_dirs_with_ini = if has_mod_ini {
        let mut count = 0;
        for dir in &child_dirs {
            if has_any_mod_ini_for_mode(dir, strict)? {
                count += 1;
                if count == VARIANT_CONTAINER_MIN_CHILDREN {
                    break;
                }
            }
        }
        count
    } else {
        0
    };

    // 2. VariantContainer explicit check
    // MUST have a root mod ini AND enough variant subfolders
    if has_mod_ini
        && (child_dirs_with_ini >= VARIANT_CONTAINER_MIN_CHILDREN
            || (!referenced_subs.is_empty()
                && child_dirs_with_ini >= VARIANT_CONTAINER_MIN_CHILDREN_REFERENCED))
    {
        reasons.push(format!(
            "{child_dirs_with_ini} child dirs with mod ini -> VariantContainer"
        ));
        return Ok((NodeType::VariantContainer, reasons, warnings));
    }

    // 1. ModPackRoot explicit check (Has INI and Assets)
    if has_mod_ini && has_assets {
        reasons.push("Has mod ini and mod assets -> ModPackRoot".into());
        return Ok((NodeType::ModPackRoot, reasons, warnings));
    }

    // 3. Fallback for non-Mod folders
    if !has_mod_ini {
        reasons.push("No root mod ini and not enough variant subfolders -> ContainerFolder".into());
        return Ok((NodeType::ContainerFolder, reasons, warnings));
    }

    // 4. Meaningful children check for FlatModRoot (Requires Mod INI)
    let has_meaningful_children = child_dirs.iter().any(|dir| {
        let fname = path_file_name_lossy(dir).unwrap_or_default();
        !referenced_subs
            .iter()
            .any(|sub| names_equal_by_key(sub, &fname))
    });

    if !has_meaningful_children {
        reasons.push(
            "No meaningful subfolders (all children are internal/assets) -> FlatModRoot".into(),
        );
        return Ok((NodeType::FlatModRoot, reasons, warnings));
    }

    // 5. Fallback ModPackRoot (Has Mod INI but no assets, yet has meaningful subfolders)
    reasons.push("Fallback -> ModPackRoot (no assets, but has ini and meaningful folders)".into());
    Ok((NodeType::ModPackRoot, reasons, warnings))
}

/// Returns a best-effort folder classification for non-authoritative UI discovery.
/// Read and decode failures intentionally remain non-fatal here.
pub fn classify_folder(path: &Path) -> (NodeType, Vec<String>, Vec<String>) {
    classify_folder_with_mode(path, false).unwrap_or((NodeType::ContainerFolder, vec![], vec![]))
}

/// Returns an authoritative classification for reconcile and source inspection.
/// Filesystem and lossy decoding failures are returned to the caller.
pub fn classify_folder_strict(
    path: &Path,
) -> Result<(NodeType, Vec<String>, Vec<String>), ClassificationError> {
    classify_folder_with_mode(path, true)
}

fn has_any_mod_ini_for_mode(path: &Path, strict: bool) -> Result<bool, ClassificationError> {
    let Some(scan) = scan_folder_for_mode(path, strict)? else {
        return Ok(false);
    };
    for ini_path in &scan.ini_files {
        if scan_ini_file_for_mode(ini_path, strict)?.is_mod {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Scan INI content for mod section headers and `filename=` references.
///
/// Returns: (has_mod_section, referenced_subfolder_names)
fn scan_ini_content(content: &str) -> (bool, Vec<String>) {
    let mut has_mod_section = false;
    let mut referenced_subs: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Check section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            let lower = canonical_name_key(section);
            if MOD_SECTION_PREFIXES.iter().any(|p| lower.starts_with(p)) {
                has_mod_section = true;
            }
            continue;
        }

        // Check filename= references for subfolder detection
        if let Some((key, value)) = trimmed.split_once('=') {
            if !key.trim().eq_ignore_ascii_case("filename") {
                continue;
            }

            let val = value.trim();
            // Extract first path component (subfolder name)
            if let Some(sub) = val.split(['/', '\\']).next() {
                let sub = sub.trim();
                if !sub.is_empty()
                    && !sub.contains('.')
                    && !sub.starts_with('$')
                    && !referenced_subs
                        .iter()
                        .any(|item| names_equal_by_key(item, sub))
                {
                    referenced_subs.push(sub.to_string());
                }
            }
        }
    }

    (has_mod_section, referenced_subs)
}

#[cfg(test)]
#[path = "tests/classifier_tests.rs"]
mod tests;
