//! Walking one mod folder into the facts the similarity model scores.
//!
//! Split out of `signals.rs`, which had grown to hold three separate jobs:
//! this walk, the BLAKE3 hashing beside it, and the scoring model itself.
//! Someone tuning the model should not have to scroll past file IO.

use crate::shared::errors::ScannerError;
use crate::modules::workspace::application::scanner::core::walker::ModCandidate;
use std::collections::{BTreeSet, HashMap};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub(crate) struct ModSnapshot {
    pub candidate: ModCandidate,
    pub files: Vec<FileEntry>,
    pub total_size_bytes: u64,
    pub ini_headers: BTreeSet<String>,
    pub keybindings: BTreeSet<String>,
    pub target_hashes: BTreeSet<String>,
    pub extensions: HashMap<String, u64>,
    /// The three fields below are per-*snapshot* facts that used to be
    /// recomputed per *pair*. A mod that lands in k candidate pairs rebuilt its
    /// file set and re-ran two name normalizations k times; the folder walk
    /// that produces the snapshot already has everything they need.
    pub file_set: BTreeSet<String>,
    pub normalized_name: String,
    pub version_stripped_name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct FileEntry {
    pub rel_path: String,
    pub abs_path: PathBuf,
    pub size_bytes: u64,
    pub extension: String,
}

pub(crate) fn collect_snapshot(candidate: &ModCandidate) -> Result<ModSnapshot, ScannerError> {
    let mut files = Vec::new();
    let mut total_size = 0_u64;
    let mut ini_headers = BTreeSet::new();
    let mut keybindings = BTreeSet::new();
    let mut target_hashes = BTreeSet::new();
    let mut extensions: HashMap<String, u64> = HashMap::new();

    for entry in WalkDir::new(&candidate.path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || (!entry.file_type().is_dir()
                    || !entry.file_name().to_string_lossy().starts_with('.'))
        })
        .filter_map(|item| item.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if is_ignored_file_name(entry.file_name()) {
            continue;
        }
        let path = entry.path().to_path_buf();
        let rel = path
            .strip_prefix(&candidate.path)
            .map(|value| value.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let size = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
        total_size = total_size.saturating_add(size);
        *extensions.entry(extension.clone()).or_insert(0) += 1;
        if extension == "ini" {
            let (headers, bindings, hashes) = read_ini_signals(&path);
            ini_headers.extend(headers);
            keybindings.extend(bindings);
            target_hashes.extend(hashes);
        }

        files.push(FileEntry {
            rel_path: rel,
            abs_path: path,
            size_bytes: size,
            extension,
        });
    }

    let file_set: BTreeSet<String> = files.iter().map(|file| file.rel_path.clone()).collect();

    Ok(ModSnapshot {
        normalized_name: normalize_name(&candidate.display_name),
        version_stripped_name: strip_version(&candidate.display_name),
        candidate: candidate.clone(),
        files,
        file_set,
        total_size_bytes: total_size,
        ini_headers,
        keybindings,
        target_hashes,
        extensions,
    })
}

pub(super) fn is_ignored_file_name(name: &OsStr) -> bool {
    matches!(
        name.to_string_lossy().to_ascii_lowercase().as_str(),
        "desktop.ini" | "thumbs.db" | ".ds_store"
    )
}

static RE_VERSION: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(v|ver|version)\s*\d+(\.\d+)*\b").unwrap()
});

fn strip_version(name: &str) -> String {
    RE_VERSION
        .replace_all(name, "")
        .to_string()
        .replace("  ", " ")
        .trim()
        .to_lowercase()
}

fn read_ini_signals(path: &Path) -> (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>) {
    let bytes = match fs::read(path) {
        Ok(value) => value,
        Err(_) => return (BTreeSet::new(), BTreeSet::new(), BTreeSet::new()),
    };
    let (content, _, _) = crate::modules::library::application::ini::document::decode_ini_bytes(&bytes);
    let mut headers = BTreeSet::new();
    let mut keybindings = BTreeSet::new();
    let mut target_hashes = BTreeSet::new();
    let mut hash_kind: Option<(&str, usize)> = None;

    for line in content.lines() {
        let trimmed = line.trim().to_ascii_lowercase();
        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') {
            headers.insert(trimmed.clone());
            hash_kind = if trimmed.starts_with("[textureoverride") {
                Some(("texture", 8))
            } else if trimmed.starts_with("[shaderoverride") {
                Some(("shader", 16))
            } else {
                None
            };
        } else if trimmed.contains("$swapvar") || trimmed.starts_with("key") {
            keybindings.insert(trimmed.clone());
        } else if trimmed.starts_with("hash") {
            if let (Some((kind, expected_len)), Some((_, hash_val))) =
                (hash_kind, trimmed.split_once('='))
            {
                let hash_val = hash_val.split(';').next().unwrap_or_default().trim();
                if hash_val.len() == expected_len
                    && hash_val
                        .chars()
                        .all(|character| character.is_ascii_hexdigit())
                {
                    target_hashes.insert(format!("{kind}:{hash_val}"));
                }
            }
        }
    }

    (headers, keybindings, target_hashes)
}

fn normalize_name(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

#[cfg(test)]
#[path = "tests/dedup_snapshot_tests.rs"]
mod tests;
