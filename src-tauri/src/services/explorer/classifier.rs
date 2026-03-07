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

/// Classify a folder based on its contents.
///
/// Returns the node type and a list of human-readable reasons for the classification.
///
/// **Opt-G:** Single-pass implementation — does ONE `fs::read_dir` to collect:
///   - ini files (scanned for mod sections + filename= references)
///   - mod asset files (.buf/.ib/.dds/.hlsl/.vb)
///   - child directory names (for meaningful-subdirs and variant-container checks)
pub fn classify_folder(path: &Path) -> (NodeType, Vec<String>) {
    if !path.is_dir() {
        return (NodeType::ContainerFolder, vec![]);
    }

    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return (NodeType::ContainerFolder, vec![]),
    };

    // Single pass: collect ini files, asset presence, and child dir paths
    let mut ini_files: Vec<std::path::PathBuf> = Vec::new();
    let mut has_assets = false;
    let mut child_dirs: Vec<std::path::PathBuf> = Vec::new();

    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_dir() {
            let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if !fname.starts_with('.') {
                child_dirs.push(p);
            }
        } else if p.is_file() {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_lowercase();

            if ext == "ini" {
                let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                if !fname.eq_ignore_ascii_case("desktop.ini") {
                    ini_files.push(p);
                }
            } else if !has_assets && MOD_ASSET_EXTENSIONS.contains(&ext.as_str()) {
                has_assets = true;
            }
        }
    }

    // Scan ini files for mod sections and referenced subfolders
    let mut has_mod_ini = false;
    let mut referenced_subs: Vec<String> = Vec::new();
    let mut reasons: Vec<String> = Vec::new();

    for ini_path in &ini_files {
        let fname = ini_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        let content = match fs::read_to_string(ini_path) {
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

    if !has_mod_ini {
        return (NodeType::ContainerFolder, reasons);
    }

    if has_assets {
        reasons.push("Has mod asset files (.buf/.ib/.dds/.hlsl/.vb)".into());
    }

    // According to TC-11-16: ModPackRoot > VariantContainer
    // If it has mod assets, it explicitly resolves to ModPackRoot (or FlatModRoot)
    if !has_assets {
        // Count child dirs with mod ini (variant check)
        // This still needs to read_dir each child, but only if we got this far (no assets scenario)
        let child_dirs_with_ini = child_dirs.iter().filter(|dir| has_any_mod_ini(dir)).count();
        if child_dirs_with_ini >= 3 || (!referenced_subs.is_empty() && child_dirs_with_ini >= 2) {
            reasons.push(format!("{child_dirs_with_ini} child dirs with mod ini"));
            return (NodeType::VariantContainer, reasons);
        }
    }

    // Check for meaningful child dirs (not internal/referenced by INI)
    let has_meaningful_children = child_dirs.iter().any(|dir| {
        let fname = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        !referenced_subs
            .iter()
            .any(|sub| sub.eq_ignore_ascii_case(fname))
    });

    if !has_meaningful_children {
        reasons.push("No meaningful subfolders (all children are internal/assets)".into());
        return (NodeType::FlatModRoot, reasons);
    }

    (NodeType::ModPackRoot, reasons)
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
        let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if fname.eq_ignore_ascii_case("desktop.ini") {
            continue;
        }
        // Quick scan: just check for section headers, don't parse filename= refs
        if let Ok(content) = fs::read_to_string(&p) {
            let (has_mod, _) = scan_ini_content(&content, fname);
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

#[cfg(test)]
#[path = "tests/classifier_tests.rs"]
mod tests;
