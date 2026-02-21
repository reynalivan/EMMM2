use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::services::scanner::deep_matcher::MatchMode;
use crate::services::scanner::normalizer;
use crate::services::scanner::walker::FolderContent;

use super::{
    decode_ini_content_with_cap, extract_hashes_from_ini_text, extract_structural_ini_tokens,
    IniTokenizationConfig,
};

pub const QUICK_MAX_INI_FILES: usize = 2;
pub const QUICK_MAX_INI_BYTES_PER_FILE: usize = 256 * 1024;
pub const QUICK_MAX_NAME_ITEMS: usize = 150;
pub const FULL_MAX_INI_FILES: usize = 10;
pub const FULL_MAX_TOTAL_INI_BYTES: usize = 1024 * 1024;
pub const FULL_MAX_NAME_ITEMS: usize = 500;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderSignals {
    pub folder_tokens: Vec<String>,
    pub deep_name_tokens: Vec<String>,
    pub ini_section_tokens: Vec<String>,
    pub ini_content_tokens: Vec<String>,
    pub ini_hashes: Vec<String>,
    pub scanned_ini_files: usize,
    pub scanned_name_items: usize,
    pub scanned_ini_bytes: usize,
}

#[derive(Debug, Clone, Copy)]
struct SignalBudget {
    max_depth: usize,
    root_ini_only: bool,
    max_ini_files: usize,
    max_ini_bytes_per_file: Option<usize>,
    max_ini_bytes_total: Option<usize>,
    max_name_items: usize,
}

impl SignalBudget {
    fn for_mode(mode: MatchMode) -> Self {
        match mode {
            MatchMode::Quick => Self {
                max_depth: 1,
                root_ini_only: true,
                max_ini_files: QUICK_MAX_INI_FILES,
                max_ini_bytes_per_file: Some(QUICK_MAX_INI_BYTES_PER_FILE),
                max_ini_bytes_total: None,
                max_name_items: QUICK_MAX_NAME_ITEMS,
            },
            MatchMode::FullScoring => Self {
                max_depth: 3,
                root_ini_only: false,
                max_ini_files: FULL_MAX_INI_FILES,
                max_ini_bytes_per_file: None,
                max_ini_bytes_total: Some(FULL_MAX_TOTAL_INI_BYTES),
                max_name_items: FULL_MAX_NAME_ITEMS,
            },
        }
    }
}

pub fn collect_deep_signals(
    folder: &Path,
    content: &FolderContent,
    mode: MatchMode,
    ini_config: &IniTokenizationConfig,
) -> FolderSignals {
    let budget = SignalBudget::for_mode(mode);

    let mut folder_tokens = BTreeSet::new();
    if let Some(folder_name) = folder
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    {
        for token in normalizer::preprocess_text(&folder_name) {
            folder_tokens.insert(token);
        }
    }

    let name_items = collect_name_items(content, budget.max_name_items);
    let mut deep_name_tokens = BTreeSet::new();
    for name_item in &name_items {
        for token in normalizer::preprocess_text(name_item) {
            deep_name_tokens.insert(token);
        }
    }

    let selected_ini_files = select_ini_files(folder, &content.ini_files, budget);
    let mut ini_section_tokens = BTreeSet::new();
    let mut ini_content_tokens = BTreeSet::new();
    let mut ini_hashes = BTreeSet::new();
    let mut scanned_ini_files = 0_usize;
    let mut scanned_ini_bytes = 0_usize;

    for ini_path in selected_ini_files {
        if let Some(total_cap) = budget.max_ini_bytes_total {
            if scanned_ini_bytes >= total_cap {
                break;
            }
        }

        let mut per_file_cap = budget.max_ini_bytes_per_file;
        if let Some(total_cap) = budget.max_ini_bytes_total {
            let remaining = total_cap.saturating_sub(scanned_ini_bytes);
            if remaining == 0 {
                break;
            }

            per_file_cap = Some(match per_file_cap {
                Some(existing_cap) => existing_cap.min(remaining),
                None => remaining,
            });
        }

        let file_size = file_size_bytes(&ini_path);
        let read_budget = per_file_cap.unwrap_or(file_size);
        let bytes_read = file_size.min(read_budget);
        if bytes_read == 0 && file_size > 0 {
            continue;
        }

        let ini_text = match decode_ini_content_with_cap(&ini_path, per_file_cap) {
            Ok(text) => text,
            Err(_) => continue,
        };

        scanned_ini_files += 1;
        scanned_ini_bytes = scanned_ini_bytes.saturating_add(bytes_read);

        for hash in extract_hashes_from_ini_text(&ini_text) {
            ini_hashes.insert(hash);
        }

        let buckets = extract_structural_ini_tokens(&ini_text, ini_config);
        for token in buckets.section_tokens {
            ini_section_tokens.insert(token);
        }
        for token in buckets.key_tokens {
            ini_content_tokens.insert(token);
        }
        for token in buckets.path_tokens {
            ini_content_tokens.insert(token);
        }
    }

    FolderSignals {
        folder_tokens: folder_tokens.into_iter().collect(),
        deep_name_tokens: deep_name_tokens.into_iter().collect(),
        ini_section_tokens: ini_section_tokens.into_iter().collect(),
        ini_content_tokens: ini_content_tokens.into_iter().collect(),
        ini_hashes: ini_hashes.into_iter().collect(),
        scanned_ini_files,
        scanned_name_items: name_items.len(),
        scanned_ini_bytes,
    }
}

fn collect_name_items(content: &FolderContent, max_name_items: usize) -> Vec<String> {
    let mut unique_items = BTreeSet::new();
    for subfolder_name in &content.subfolder_names {
        unique_items.insert(subfolder_name.to_string());
    }

    for file in &content.files {
        let stem = file
            .path
            .file_stem()
            .or_else(|| Path::new(&file.name).file_stem())
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        if !stem.is_empty() {
            unique_items.insert(stem);
        }
    }

    unique_items.into_iter().take(max_name_items).collect()
}

fn select_ini_files(folder: &Path, ini_files: &[PathBuf], budget: SignalBudget) -> Vec<PathBuf> {
    let mut ini_candidates = Vec::new();
    for ini_file in ini_files {
        let Some(relative_path) = relative_path(folder, ini_file) else {
            continue;
        };

        let Some(depth) = relative_depth(folder, ini_file) else {
            continue;
        };

        if budget.root_ini_only {
            if depth != 1 {
                continue;
            }
        } else if depth > budget.max_depth {
            continue;
        }

        ini_candidates.push((relative_path, ini_file.clone()));
    }

    ini_candidates.sort_by(|(a, _), (b, _)| a.cmp(b));
    ini_candidates
        .into_iter()
        .take(budget.max_ini_files)
        .map(|(_, path)| path)
        .collect()
}

fn relative_path(folder: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(folder).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn relative_depth(folder: &Path, path: &Path) -> Option<usize> {
    let relative = path.strip_prefix(folder).ok()?;
    Some(relative.components().count())
}

fn file_size_bytes(path: &Path) -> usize {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| usize::try_from(metadata.len()).ok())
        .unwrap_or(0)
}
