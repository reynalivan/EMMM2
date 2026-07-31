//! Folder classification engine for the Navigable FolderGrid system.
//!
//! Classifies mod folders as one of:
//! - `ContainerFolder` — navigable, contains subfolders
//! - `ModPackRoot` — has valid 3DMigoto mod ini + assets
//! - `VariantContainer` — orchestrator with multiple variant subfolders
//! - `InternalAssets` — child folder referenced by parent's `filename=` directives
//!
//! # Covers: navigablefoldergrid.md §5

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

/// One directory pass: the folder's mod ini candidates, child dirs and asset presence.
struct FolderScan {
    /// `.ini` files directly inside the folder, `desktop.ini` excluded.
    ini_files: Vec<PathBuf>,
    child_dirs: Vec<PathBuf>,
    has_assets: bool,
}

fn scan_folder(path: &Path) -> Option<FolderScan> {
    let entries = fs::read_dir(path).ok()?;

    let mut scan = FolderScan {
        ini_files: Vec::new(),
        child_dirs: Vec::new(),
        has_assets: false,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            let fname = path_file_name_lossy(&p).unwrap_or_default();
            if !fname.starts_with('.') {
                scan.child_dirs.push(p);
            }
        } else if p.is_file() {
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

    let Ok(content) = fs::read_to_string(path) else {
        return scan;
    };

    let (is_mod, referenced_subs) = scan_ini_content(&content);
    scan.is_mod = is_mod;
    scan.referenced_subs = referenced_subs;
    scan
}

/// Returns the node type, a list of diagnostic reasons, and a list of warnings.
pub fn classify_folder(path: &Path) -> (NodeType, Vec<String>, Vec<String>) {
    if !path.is_dir() {
        return (NodeType::ContainerFolder, vec![], vec![]);
    }

    let Some(FolderScan {
        ini_files,
        child_dirs,
        has_assets,
    }) = scan_folder(path)
    else {
        return (NodeType::ContainerFolder, vec![], vec![]);
    };

    // Scan ini files for mod sections and referenced subfolders
    let mut has_mod_ini = false;
    let mut reasons: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut referenced_subs: Vec<String> = Vec::new();

    for ini_path in &ini_files {
        let fname = path_file_name_lossy(ini_path).unwrap_or_default();
        let scan = scan_ini_file(ini_path);

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

    let child_dirs_with_ini = child_dirs.iter().filter(|dir| has_any_mod_ini(dir)).count();

    // 2. VariantContainer explicit check
    // MUST have a root mod ini AND enough variant subfolders
    if has_mod_ini
        && (child_dirs_with_ini >= 3 || (!referenced_subs.is_empty() && child_dirs_with_ini >= 2))
    {
        reasons.push(format!(
            "{child_dirs_with_ini} child dirs with mod ini -> VariantContainer"
        ));
        return (NodeType::VariantContainer, reasons, warnings);
    }

    // 1. ModPackRoot explicit check (Has INI and Assets)
    if has_mod_ini && has_assets {
        reasons.push("Has mod ini and mod assets -> ModPackRoot".into());
        return (NodeType::ModPackRoot, reasons, warnings);
    }

    // 3. Fallback for non-Mod folders
    if !has_mod_ini {
        reasons.push("No root mod ini and not enough variant subfolders -> ContainerFolder".into());
        return (NodeType::ContainerFolder, reasons, warnings);
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
        return (NodeType::FlatModRoot, reasons, warnings);
    }

    // 5. Fallback ModPackRoot (Has Mod INI but no assets, yet has meaningful subfolders)
    reasons.push("Fallback -> ModPackRoot (no assets, but has ini and meaningful folders)".into());
    (NodeType::ModPackRoot, reasons, warnings)
}

/// Quick check: does a directory contain at least one valid mod ini file?
/// Used for variant-container detection (called on child dirs only when needed).
fn has_any_mod_ini(path: &Path) -> bool {
    scan_folder(path)
        .map(|scan| scan.ini_files.iter().any(|p| scan_ini_file(p).is_mod))
        .unwrap_or(false)
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
            if !names_equal_by_key(key.trim(), "filename") {
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
