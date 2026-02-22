use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::services::scanner::core::normalizer;
use crate::services::scanner::core::walker::FolderContent;
use crate::services::scanner::deep_matcher::MatchMode;

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
    /// Normalized continuous strings from subfolder names + file stems
    /// (for substring matching, NOT tokenized).
    pub deep_name_strings: Vec<String>,
    /// Normalized root folder name (for last-resort matching only).
    pub folder_name_normalized: String,
    /// Normalized continuous strings from INI section headers + path-like RHS
    /// (for substring matching Pass B, NOT tokenized).
    pub ini_derived_strings: Vec<String>,
    pub ini_section_tokens: Vec<String>,
    pub ini_content_tokens: Vec<String>,
    pub ini_hashes: Vec<String>,
    pub scanned_ini_files: usize,
    pub scanned_name_items: usize,
    pub scanned_ini_bytes: usize,
    /// blake3 fingerprint of all signal fields for cache-key support.
    pub fingerprint: String,
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
    let mut folder_name_normalized = String::new();
    if let Some(folder_name) = folder
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    {
        folder_name_normalized = normalizer::normalize_for_matching_default(&folder_name);
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
    let mut ini_derived_strings_set = BTreeSet::new();
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

        // Collect continuous strings from section headers + path stems
        // for substring matching Pass B
        for section_str in &buckets.section_strings {
            let normalized = normalizer::normalize_for_matching_default(section_str);
            if normalized.len() >= 3 {
                ini_derived_strings_set.insert(normalized);
            }
        }
        for path_str in &buckets.path_strings {
            let normalized = normalizer::normalize_for_matching_default(path_str);
            if normalized.len() >= 3 {
                ini_derived_strings_set.insert(normalized);
            }
        }
    }

    // Collect normalized continuous strings for substring matching
    let mut deep_name_strings = BTreeSet::new();
    for name_item in &name_items {
        let normalized = normalizer::normalize_for_matching_default(name_item);
        if !normalized.is_empty() {
            deep_name_strings.insert(normalized);
        }
    }

    let mut signals = FolderSignals {
        folder_tokens: folder_tokens.into_iter().collect(),
        deep_name_tokens: deep_name_tokens.into_iter().collect(),
        deep_name_strings: deep_name_strings.into_iter().collect(),
        folder_name_normalized,
        ini_derived_strings: ini_derived_strings_set.into_iter().collect(),
        ini_section_tokens: ini_section_tokens.into_iter().collect(),
        ini_content_tokens: ini_content_tokens.into_iter().collect(),
        ini_hashes: ini_hashes.into_iter().collect(),
        scanned_ini_files,
        scanned_name_items: name_items.len(),
        scanned_ini_bytes,
        fingerprint: String::new(),
    };
    signals.fingerprint = compute_fingerprint(&signals);
    signals
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

/// Compute a blake3 fingerprint of all signal fields for cache-key support.
fn compute_fingerprint(signals: &FolderSignals) -> String {
    let mut hasher = blake3::Hasher::new();
    hash_string_vec(&mut hasher, b"folder", &signals.folder_tokens);
    hash_string_vec(&mut hasher, b"deep", &signals.deep_name_tokens);
    hash_string_vec(&mut hasher, b"iniderived", &signals.ini_derived_strings);
    hash_string_vec(&mut hasher, b"section", &signals.ini_section_tokens);
    hash_string_vec(&mut hasher, b"content", &signals.ini_content_tokens);
    hash_string_vec(&mut hasher, b"hash", &signals.ini_hashes);
    hasher.update(&signals.scanned_ini_files.to_le_bytes());
    hasher.update(&signals.scanned_name_items.to_le_bytes());
    hasher.update(&signals.scanned_ini_bytes.to_le_bytes());
    hasher.finalize().to_hex().to_string()
}

fn hash_string_vec(hasher: &mut blake3::Hasher, label: &[u8], values: &[String]) {
    hasher.update(label);
    for value in values {
        let bytes = value.as_bytes();
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
}
