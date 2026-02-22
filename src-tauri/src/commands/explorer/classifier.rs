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

/// The classification result for a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    ContainerFolder,
    ModPackRoot,
    VariantContainer,
    InternalAssets,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ContainerFolder => "ContainerFolder",
            Self::ModPackRoot => "ModPackRoot",
            Self::VariantContainer => "VariantContainer",
            Self::InternalAssets => "InternalAssets",
        }
    }
}

/// File extensions that indicate 3DMigoto mod assets.
const MOD_ASSET_EXTENSIONS: &[&str] = &["buf", "ib", "dds", "hlsl", "vb"];

/// Section prefixes that indicate a valid 3DMigoto mod ini.
const MOD_SECTION_PREFIXES: &[&str] = &["textureoverride", "shaderoverride", "resource"];

/// Classify a folder based on its contents.
///
/// Returns the node type and a list of human-readable reasons for the classification.
pub fn classify_folder(path: &Path) -> (NodeType, Vec<String>) {
    if !path.is_dir() {
        return (NodeType::ContainerFolder, vec![]);
    }

    let (has_mod_ini, referenced_subs, mut reasons) = scan_ini_files(path);

    if !has_mod_ini {
        return (NodeType::ContainerFolder, reasons);
    }

    // Check for variant pattern: ≥3 child dirs each with their own mod ini
    let child_dirs_with_ini = count_child_dirs_with_mod_ini(path);
    if child_dirs_with_ini >= 3 || !referenced_subs.is_empty() && child_dirs_with_ini >= 2 {
        reasons.push(format!("{child_dirs_with_ini} child dirs with mod ini"));
        return (NodeType::VariantContainer, reasons);
    }

    // It has a mod ini — classify as ModPackRoot
    if has_mod_assets(path) {
        reasons.push("Has mod asset files (.buf/.ib/.dds/.hlsl/.vb)".into());
    }
    (NodeType::ModPackRoot, reasons)
}

/// Scan `.ini` files at the root of a folder for 3DMigoto mod sections.
///
/// Returns: (has_valid_mod_ini, referenced_subfolder_names, reasons)
fn scan_ini_files(path: &Path) -> (bool, Vec<String>, Vec<String>) {
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return (false, vec![], vec![]),
    };

    let mut has_mod_ini = false;
    let mut referenced_subs: Vec<String> = Vec::new();
    let mut reasons: Vec<String> = Vec::new();

    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or_default();
        if !ext.eq_ignore_ascii_case("ini") {
            continue;
        }
        // Skip desktop.ini
        let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if fname.eq_ignore_ascii_case("desktop.ini") {
            continue;
        }

        // Read and scan the file (lightweight line-based scan, not full parse)
        let content = match fs::read_to_string(&p) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (found_mod, subs) = scan_ini_content(&content, fname);
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

    (has_mod_ini, referenced_subs, reasons)
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
            let lower = section.to_lowercase();
            if MOD_SECTION_PREFIXES.iter().any(|p| lower.starts_with(p)) {
                has_mod_section = true;
            }
            continue;
        }

        // Check filename= references for subfolder detection
        let lower = trimmed.to_lowercase();
        if lower.starts_with("filename") {
            if let Some((_key, value)) = trimmed.split_once('=') {
                let val = value.trim();
                // Extract first path component (subfolder name)
                if let Some(sub) = val.split(['/', '\\']).next() {
                    let sub = sub.trim();
                    if !sub.is_empty()
                        && !sub.contains('.')
                        && !sub.starts_with('$')
                        && !referenced_subs.contains(&sub.to_string())
                    {
                        referenced_subs.push(sub.to_string());
                    }
                }
            }
        }
    }

    (has_mod_section, referenced_subs)
}

/// Check if a folder contains typical 3DMigoto mod asset files at root.
fn has_mod_assets(path: &Path) -> bool {
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if MOD_ASSET_EXTENSIONS.contains(&ext.as_str()) {
            return true;
        }
    }
    false
}

/// Count how many immediate child directories contain a valid mod ini.
fn count_child_dirs_with_mod_ini(path: &Path) -> usize {
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            let (has_ini, _, _) = scan_ini_files(&e.path());
            has_ini
        })
        .count()
}

#[cfg(test)]
#[path = "tests/classifier_tests.rs"]
mod tests;
