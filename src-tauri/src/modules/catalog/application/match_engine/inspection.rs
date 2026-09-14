use super::types::SourceInspection;
use crate::modules::ingestion::application::import_batch::types::{
    MatchEvidence, SourceFingerprint,
};
use crate::modules::workspace::application::scanner::core::walker::{
    is_scannable_extension, FileInfo, FolderContent,
};
use crate::shared::errors::ScannerError;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub struct InspectionRequest {
    pub source_path: PathBuf,
    pub planned_name: Option<String>,
    pub match_extensions: Vec<String>,
}

/// A fresh, per-request disk snapshot for classification. It intentionally has
/// no cache lifetime: callers use it immediately for both fingerprinting and
/// matching, then discard it.
pub struct InspectedSource {
    pub inspection: SourceInspection,
    pub content: FolderContent,
}

pub fn normalized_match_name(value: &str) -> String {
    let without_disabled = value
        .trim()
        .strip_prefix(crate::DISABLED_PREFIX)
        .or_else(|| value.trim().strip_prefix("disabled "))
        .unwrap_or(value.trim());
    deunicode::deunicode(without_disabled)
        .chars()
        .filter(|character| character.is_alphabetic())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn source_display_name(value: &str) -> String {
    value
        .trim()
        .strip_prefix(crate::DISABLED_PREFIX)
        .or_else(|| value.trim().strip_prefix("disabled "))
        .unwrap_or(value.trim())
        .to_string()
}

pub fn inspect_source(request: &InspectionRequest) -> Result<SourceInspection, ScannerError> {
    Ok(inspect_source_with_content(request)?.inspection)
}

pub fn inspect_source_with_content(
    request: &InspectionRequest,
) -> Result<InspectedSource, ScannerError> {
    if !request.source_path.is_dir() {
        return Err(ScannerError::NotADirectory {
            path: request.source_path.to_string_lossy().into_owned(),
        });
    }

    let raw_name = request.planned_name.clone().unwrap_or_else(|| {
        request
            .source_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let source_name = source_display_name(&raw_name);
    let allowed_extensions = request
        .match_extensions
        .iter()
        .map(|value| value.trim_start_matches('.').to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut nested_names = BTreeSet::new();
    let mut matching_files = Vec::new();
    let mut ini_sections = BTreeSet::new();
    let mut total_size = 0_u64;
    let mut file_count = 0_u32;
    let mut latest_modified = request
        .source_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .ok();
    let mut subfolder_names = Vec::new();
    let mut files = Vec::new();
    let mut ini_files = Vec::new();

    for entry in walkdir::WalkDir::new(&request.source_path)
        .min_depth(1)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            nested_names.insert(name.clone());
            subfolder_names.push(name);
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if extension == "ini" {
            ini_files.push(path.to_path_buf());
        }
        if is_scannable_extension(&extension) {
            files.push(FileInfo {
                path: path.to_path_buf(),
                name,
                extension: extension.clone(),
            });
        }

        // `scan_folder_content` includes symlinks in matcher content, while
        // source inspection intentionally excludes them from fingerprints.
        if entry.file_type().is_symlink() || !entry.file_type().is_file() {
            continue;
        }
        file_count = file_count.saturating_add(1);
        if let Ok(metadata) = entry.metadata() {
            total_size = total_size.saturating_add(metadata.len());
            if let Ok(modified) = metadata.modified() {
                latest_modified =
                    Some(latest_modified.map_or(modified, |current| current.max(modified)));
            }
        }
        if let Some(stem) = path.file_stem() {
            nested_names.insert(stem.to_string_lossy().into_owned());
        }
        if !allowed_extensions.contains(&extension) {
            continue;
        }
        matching_files.push(path.to_string_lossy().into_owned());
        if extension == "ini" {
            collect_ini_sections(path, &mut ini_sections);
        }
    }

    matching_files.sort();
    let modified_unix_ms = latest_modified
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|| "0".to_string());
    let source_path = request.source_path.to_string_lossy().into_owned();
    Ok(InspectedSource {
        inspection: SourceInspection {
            source_path: source_path.clone(),
            source_name: source_name.clone(),
            normalized_name: normalized_match_name(&source_name),
            nested_names: nested_names.into_iter().collect(),
            matching_files,
            ini_sections: ini_sections.into_iter().collect(),
            evidence: vec![MatchEvidence {
                source: "folder_name".to_string(),
                value: source_name,
                score: 1.0,
            }],
            fingerprint: SourceFingerprint {
                path: source_path,
                modified_unix_ms,
                size_bytes: total_size.to_string(),
                file_count,
            },
        },
        content: FolderContent {
            subfolder_names,
            files,
            ini_files,
        },
    })
}

fn collect_ini_sections(path: &Path, sections: &mut BTreeSet<String>) {
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let text = String::from_utf8_lossy(&bytes);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
            sections.insert(trimmed[1..trimmed.len() - 1].to_string());
        }
    }
}
