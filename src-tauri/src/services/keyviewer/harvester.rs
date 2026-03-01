//! Hash harvester — extracts `hash = XXXXXXXX` from enabled mods' INI files.
//!
//! Scans `TextureOverride*` and `ShaderOverride*` sections in `.ini` files,
//! extracting hash assignments. This is separate from `read_ini_document` which
//! focuses on key bindings and variables — the harvester only cares about hashes.

use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::services::ini::document::list_ini_files;

/// Regex matching `hash = XXXXXXXX` (8 hex digits, case-insensitive).
static HASH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*hash\s*=\s*([0-9a-f]{8})\s*(?:[;#].*)?$").expect("valid hash regex")
});

/// Regex matching section headers like `[TextureOverrideFoo]` or `[ShaderOverrideBar]`.
static OVERRIDE_SECTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*\[((?:TextureOverride|ShaderOverride)[^\]]*)\]\s*$")
        .expect("valid override section regex")
});

/// A hash extracted from an INI file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarvestedHash {
    /// The hash value (lowercase hex, 8 chars).
    pub hash: String,
    /// INI section name where this hash was found.
    pub section_name: String,
    /// Path to the INI file.
    pub file_path: PathBuf,
}

/// Signature for incremental scanning — if these match, skip re-harvesting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSignature {
    /// File size in bytes.
    pub size: u64,
    /// Last modification time as seconds since epoch.
    pub mtime_secs: u64,
}

/// Sections to deny-list from hash harvesting (case-insensitive prefixes).
/// These are system/utility sections that should not contribute object hashes.
const DENYLIST_PREFIXES: &[&str] = &[
    "textureoverridenotification",
    "textureoverrideui",
    "textureoverridecursor",
    "shaderoverrideui",
    "shaderoverrideshadow",
];

/// Compute a file signature for incremental scanning.
pub fn compute_file_signature(file_path: &Path) -> Result<FileSignature, String> {
    let meta = fs::metadata(file_path)
        .map_err(|e| format!("Failed to stat {}: {e}", file_path.display()))?;

    let mtime_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Ok(FileSignature {
        size: meta.len(),
        mtime_secs,
    })
}

/// Check whether a file should be re-scanned based on signature change.
pub fn should_rescan(old_sig: &FileSignature, new_sig: &FileSignature) -> bool {
    old_sig != new_sig
}

/// Check if a section name is deny-listed.
fn is_denylisted(section_name: &str) -> bool {
    let lower = section_name.to_ascii_lowercase();
    DENYLIST_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

/// Harvest hashes from a single INI file.
///
/// Only extracts from `TextureOverride*` / `ShaderOverride*` sections.
/// Deny-listed sections (UI, cursor, shadow, notification) are skipped.
pub fn harvest_hashes_from_ini(file_path: &Path) -> Result<Vec<HarvestedHash>, String> {
    let bytes =
        fs::read(file_path).map_err(|e| format!("Failed to read {}: {e}", file_path.display()))?;

    // Handle BOM
    let content_bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        &bytes[..]
    };

    let text = match String::from_utf8(content_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            let (cow, _encoding, _had_errors) = encoding_rs::SHIFT_JIS.decode(content_bytes);
            cow.into_owned()
        }
    };

    let mut results = Vec::new();
    let mut current_section: Option<String> = None;
    let mut in_override_section = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // Check for section header
        if let Some(caps) = OVERRIDE_SECTION_RE.captures(trimmed) {
            let section_name = caps[1].to_string();
            if is_denylisted(&section_name) {
                in_override_section = false;
                current_section = None;
            } else {
                in_override_section = true;
                current_section = Some(section_name);
            }
            continue;
        }

        // Any other section header resets override tracking
        if trimmed.starts_with('[') && trimmed.contains(']') {
            in_override_section = false;
            current_section = None;
            continue;
        }

        // Only extract hashes from override sections
        if !in_override_section {
            continue;
        }

        if let Some(caps) = HASH_RE.captures(trimmed) {
            let hash = caps[1].to_ascii_lowercase();
            results.push(HarvestedHash {
                hash,
                section_name: current_section.clone().unwrap_or_default(),
                file_path: file_path.to_path_buf(),
            });
        }
    }

    Ok(results)
}

/// Harvest hashes from all INI files in a mod folder.
///
/// Uses `list_ini_files` to discover INI files, then extracts hashes from each.
/// Returns a map of hash → list of occurrences for deduplication/counting.
pub fn harvest_hashes_from_mod(
    mod_path: &Path,
) -> Result<HashMap<String, Vec<HarvestedHash>>, String> {
    let ini_files = list_ini_files(mod_path)?;
    let mut hash_map: HashMap<String, Vec<HarvestedHash>> = HashMap::new();

    for ini_path in ini_files {
        match harvest_hashes_from_ini(&ini_path) {
            Ok(hashes) => {
                for h in hashes {
                    hash_map.entry(h.hash.clone()).or_default().push(h);
                }
            }
            Err(e) => {
                // Log but don't fail — a single bad INI shouldn't block the whole mod
                eprintln!(
                    "[keyviewer] Failed to harvest hashes from {}: {e}",
                    ini_path.display()
                );
            }
        }
    }

    Ok(hash_map)
}
