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
use std::path::Path;

use crate::services::path_key::{canonical_name_key, names_equal_by_key, path_file_name_lossy};

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

/// Returns the node type, a list of diagnostic reasons, and a list of warnings.
pub fn classify_folder(path: &Path) -> (NodeType, Vec<String>, Vec<String>) {
    if !path.is_dir() {
        return (NodeType::ContainerFolder, vec![], vec![]);
    }

    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return (NodeType::ContainerFolder, vec![], vec![]),
    };

    // Single pass: collect ini files, asset presence, and child dir paths
    let mut ini_files: Vec<std::path::PathBuf> = Vec::new();
    let mut has_assets = false;
    let mut child_dirs: Vec<std::path::PathBuf> = Vec::new();

    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            let fname = path_file_name_lossy(&p).unwrap_or_default();
            if !fname.starts_with('.') {
                child_dirs.push(p);
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
                    ini_files.push(p);
                }
            } else if !has_assets && MOD_ASSET_EXTENSIONS.contains(&ext.as_str()) {
                has_assets = true;
            }
        }
    }

    // Scan ini files for mod sections and referenced subfolders
    let mut has_mod_ini = false;
    let mut reasons: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut referenced_subs: Vec<String> = Vec::new();

    for ini_path in &ini_files {
        let fname = path_file_name_lossy(ini_path).unwrap_or_default();

        let meta = fs::metadata(ini_path);
        let is_empty = meta.as_ref().map(|m| m.len() == 0).unwrap_or(false);

        if is_empty {
            // A 0KB INI is treated as a mod INI but flagged as corrupt
            has_mod_ini = true;
            warnings.push(format!("[WARNING] Corrupt INI file: {} (0 KB)", fname));
            reasons.push(format!("Corrupt Mod ini: {fname}"));
            continue;
        }

        let content = match fs::read_to_string(ini_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (found_mod, subs) = scan_ini_content(&content, &fname);
        if found_mod {
            has_mod_ini = true;
            reasons.push(format!("Mod ini: {fname}"));
        }
        referenced_subs.extend(subs);
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
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or_default();
        if !ext.eq_ignore_ascii_case("ini") {
            continue;
        }
        let fname = path_file_name_lossy(&p).unwrap_or_default();
        if names_equal_by_key(&fname, "desktop.ini") {
            continue;
        }
        let meta = match fs::metadata(&p) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.len() == 0 {
            return true;
        }

        if let Ok(content) = fs::read_to_string(&p) {
            let (has_mod, _) = scan_ini_content(&content, &fname);
            if has_mod {
                return true;
            }
        }
    }
    false
}

/// Scan INI content for mod section headers and `filename=` references.
///
/// Returns: (has_mod_section, referenced_subfolder_names)
fn scan_ini_content(content: &str, _ini_filename: &str) -> (bool, Vec<String>) {
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
