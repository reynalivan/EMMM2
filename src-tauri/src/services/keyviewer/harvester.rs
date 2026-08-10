//! Hash harvester — extracts `hash = XXXXXXXX` from enabled mods' INI files.
//!
//! Scans 32-bit resource hashes from `TextureOverride*` sections. Shader hashes
//! are 64-bit identities and belong to the conflict scanner, not this matcher.

use crate::domain::errors::AppError;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::services::ini::document::list_ini_files;

/// Regex matching `hash = XXXXXXXX` (8 hex digits, case-insensitive).
static HASH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*hash\s*=\s*(?:0x)?([0-9a-f]{8})\s*(?:[;#].*)?$").expect("valid hash regex")
});

/// Regex matching resource override sections such as `[TextureOverrideFoo]`.
static TEXTURE_OVERRIDE_SECTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*\[(TextureOverride[^\]]*)\]\s*$")
        .expect("valid texture override section regex")
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

/// Sections to deny-list from hash harvesting (case-insensitive prefixes).
/// These are system/utility sections that should not contribute object hashes.
const DENYLIST_PREFIXES: &[&str] = &[
    "textureoverridenotification",
    "textureoverrideui",
    "textureoverridecursor",
];

/// Check if a section name is deny-listed.
fn is_denylisted(section_name: &str) -> bool {
    let lower = section_name.to_ascii_lowercase();
    DENYLIST_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

/// Harvest hashes from a single INI file.
///
/// Only extracts from `TextureOverride*` sections.
/// Deny-listed sections (UI, cursor, shadow, notification) are skipped.
pub fn harvest_hashes_from_ini(file_path: &Path) -> Result<Vec<HarvestedHash>, AppError> {
    let bytes = fs::read(file_path)?;

    // Hash lines are ASCII, so even the lossy fallback decode scans fine.
    let (text, _had_bom, _clean) = crate::services::ini::document::decode_ini_bytes(&bytes);

    Ok(harvest_hashes_from_text(&text, file_path))
}

/// Hash harvest over INI text already decoded by the caller.
fn harvest_hashes_from_text(text: &str, file_path: &Path) -> Vec<HarvestedHash> {
    let mut results = Vec::new();
    // `Some` exactly while inside a non-denylisted override section.
    let mut current_section: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();

        // Any section header resets tracking; an override one re-arms it.
        if trimmed.starts_with('[') {
            current_section = TEXTURE_OVERRIDE_SECTION_RE
                .captures(trimmed)
                .map(|caps| caps[1].to_string())
                .filter(|section_name| !is_denylisted(section_name));
            if trimmed.contains(']') {
                continue;
            }
        }

        let Some(section_name) = current_section.as_ref() else {
            continue;
        };

        if let Some(caps) = HASH_RE.captures(trimmed) {
            results.push(HarvestedHash {
                hash: caps[1].to_ascii_lowercase(),
                section_name: section_name.clone(),
                file_path: file_path.to_path_buf(),
            });
        }
    }

    results
}

/// Harvest hashes from all INI files in a mod folder.
///
/// Uses `list_ini_files` to discover INI files, then extracts hashes from each.
/// Returns a map of hash → list of occurrences for deduplication/counting.
pub fn harvest_hashes_from_mod(
    mod_path: &Path,
) -> Result<HashMap<String, Vec<HarvestedHash>>, AppError> {
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
                log::warn!(
                    "[keyviewer] Failed to harvest hashes from {}: {e}",
                    ini_path.display()
                );
            }
        }
    }

    Ok(hash_map)
}

/// Harvest key bindings from all INI files in a mod folder.
///
/// Uses `read_ini_document` to parse [Key*] sections.
pub fn harvest_keybinds_from_mod(
    mod_path: &Path,
) -> Result<Vec<crate::services::ini::document::KeyBinding>, AppError> {
    let ini_files = list_ini_files(mod_path)?;
    let mut all_keybinds = Vec::new();

    for ini_path in ini_files {
        if let Ok(doc) = crate::services::ini::document::read_ini_document(&ini_path) {
            all_keybinds.extend(doc.key_bindings);
        }
    }

    Ok(all_keybinds)
}

/// Everything one pass over a mod's INI files yields.
#[derive(Debug, Default)]
pub struct ModHarvest {
    /// Hash -> every occurrence of it, for deduplication and counting.
    pub hashes: HashMap<String, Vec<HarvestedHash>>,
    pub keybinds: Vec<crate::services::ini::document::KeyBinding>,
}

/// Harvest hashes and key bindings from a mod folder in a single pass.
///
/// Post-apply needs both. Asking for them separately listed the directory
/// twice and read and decoded every INI twice — per enabled mod, on every
/// toggle, apply and workspace switch.
pub fn harvest_mod(mod_path: &Path) -> Result<ModHarvest, AppError> {
    use crate::services::ini::document;

    let ini_files = list_ini_files(mod_path)?;
    let mut harvest = ModHarvest::default();

    for ini_path in ini_files {
        let bytes = match fs::read(&ini_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                // A single unreadable INI must not sink the whole mod.
                log::warn!("[keyviewer] Failed to read {}: {error}", ini_path.display());
                continue;
            }
        };

        let (text, _had_bom, _clean) = document::decode_ini_bytes(&bytes);
        for hash in harvest_hashes_from_text(&text, &ini_path) {
            harvest
                .hashes
                .entry(hash.hash.clone())
                .or_default()
                .push(hash);
        }

        // Matches `read_ini_document`, which refuses anything larger.
        if bytes.len() as u64 <= document::MAX_PARSEABLE_INI_BYTES {
            harvest
                .keybinds
                .extend(document::parse_ini_document(&ini_path, &bytes).key_bindings);
        }
    }

    Ok(harvest)
}
