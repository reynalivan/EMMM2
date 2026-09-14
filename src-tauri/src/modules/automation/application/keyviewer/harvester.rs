//! Typed KeyViewer target harvesting from enabled mod INI files.
//!
//! A resource hash alone is not a safe runtime observer: a character mod can
//! contain body, face, dress, and shared texture hashes. The harvester keeps
//! the resource slot and draw discriminator so the selector can prefer
//! geometry without rescanning INI files later.

use crate::modules::library::application::ini::document::list_ini_files;
use crate::modules::matching::application::deep_matcher::models::types::RuntimeResourceKind;
use crate::shared::errors::AppError;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

static HASH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*hash\s*=\s*(?:0x)?([0-9a-f]{8})\s*(?:[;#].*)?$").expect("valid hash regex")
});

static TEXTURE_OVERRIDE_SECTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*\[(TextureOverride[^\]]*)\]\s*$")
        .expect("valid texture override section regex")
});

const DENYLIST_PREFIXES: &[&str] = &[
    "textureoverridenotification",
    "textureoverrideui",
    "textureoverridecursor",
];

/// Installed `checktextureoverride` callback slots. Harvesting accepts a
/// target only when the package can observe that exact resource slot.
#[derive(Debug, Clone, Default)]
pub struct HarvestCapabilities {
    callback_slots: HashSet<String>,
}

impl HarvestCapabilities {
    pub fn from_callback_slots<I, S>(slots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            callback_slots: slots
                .into_iter()
                .map(|slot| slot.as_ref().trim().to_ascii_lowercase())
                .filter(|slot| !slot.is_empty())
                .collect(),
        }
    }

    fn supports(&self, slot: &str) -> bool {
        self.callback_slots.contains(slot)
    }

    fn cache_key(&self) -> String {
        let mut slots: Vec<_> = self.callback_slots.iter().cloned().collect();
        slots.sort_unstable();
        slots.join(",")
    }
}

/// Resource target from an enabled `TextureOverride` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarvestedTarget {
    pub hash: String,
    pub resource_kind: RuntimeResourceKind,
    pub callback_slot: String,
    pub match_first_index: Option<u32>,
    pub section_name: String,
    pub file_path: PathBuf,
}

/// Legacy raw-hash view retained for scanner callers that do not create
/// runtime observers. The KeyViewer pipeline uses [`HarvestedTarget`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarvestedHash {
    pub hash: String,
    pub section_name: String,
    pub file_path: PathBuf,
}

#[derive(Debug, Default)]
struct OverrideSection {
    name: String,
    hashes: Vec<String>,
    slots: HashSet<String>,
    match_first_index: Option<u32>,
}

impl OverrideSection {
    fn new(name: String) -> Self {
        Self {
            name,
            hashes: Vec::new(),
            slots: HashSet::new(),
            match_first_index: None,
        }
    }

    fn observe(&mut self, line: &str) {
        if let Some(captures) = HASH_RE.captures(line) {
            self.hashes.push(captures[1].to_ascii_lowercase());
            return;
        }

        let Some((key, _)) = assignment(line) else {
            return;
        };
        let key = key.to_ascii_lowercase();
        if resource_kind_for_slot(&key).is_some() {
            self.slots.insert(key);
            return;
        }
        if key == "match_first_index" {
            self.match_first_index = assignment(line)
                .and_then(|(_, value)| parse_u32(value))
                .or(self.match_first_index);
        }
    }

    fn finish(self, file_path: &Path, capabilities: &HarvestCapabilities) -> Vec<HarvestedTarget> {
        let Some((resource_kind, callback_slot)) = self.select_resource(capabilities) else {
            return Vec::new();
        };
        self.hashes
            .into_iter()
            .map(|hash| HarvestedTarget {
                hash,
                resource_kind,
                callback_slot: callback_slot.clone(),
                match_first_index: self.match_first_index,
                section_name: self.name.clone(),
                file_path: file_path.to_path_buf(),
            })
            .collect()
    }

    fn select_resource(
        &self,
        capabilities: &HarvestCapabilities,
    ) -> Option<(RuntimeResourceKind, String)> {
        let mut slots: Vec<_> = self.slots.iter().cloned().collect();
        slots.sort_by(|left, right| {
            slot_rank(left)
                .cmp(&slot_rank(right))
                .then_with(|| left.cmp(right))
        });
        for slot in slots {
            let resource_kind = resource_kind_for_slot(&slot)?;
            if capabilities.supports(&slot) {
                return Some((resource_kind, slot));
            }
        }

        // Older packages frequently name the section but do not state `vb0`.
        // This fallback is only valid when the installed profile confirms its
        // `vb0` callback, never just because a section happens to say Position.
        if section_is_position(&self.name) && capabilities.supports("vb0") {
            return Some((RuntimeResourceKind::PositionVb, "vb0".to_string()));
        }
        None
    }
}

fn is_denylisted(section_name: &str) -> bool {
    let lower = section_name.to_ascii_lowercase();
    DENYLIST_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn assignment(line: &str) -> Option<(&str, &str)> {
    let line = line.split([';', '#']).next()?.trim();
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    let value = value.trim();
    (!key.is_empty() && !value.is_empty()).then_some((key, value))
}

fn parse_u32(value: &str) -> Option<u32> {
    let value = value.trim();
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(
            || value.parse().ok(),
            |hex| u32::from_str_radix(hex, 16).ok(),
        )
}

fn resource_kind_for_slot(slot: &str) -> Option<RuntimeResourceKind> {
    match slot {
        "vb0" => Some(RuntimeResourceKind::PositionVb),
        "vb1" | "vb2" => Some(RuntimeResourceKind::DrawVb),
        "ib" => Some(RuntimeResourceKind::IndexBuffer),
        _ if slot.starts_with("ps-t")
            && !slot[4..].is_empty()
            && slot[4..]
                .chars()
                .all(|character| character.is_ascii_digit()) =>
        {
            Some(RuntimeResourceKind::Texture)
        }
        _ if slot.starts_with("vb")
            && !slot[2..].is_empty()
            && slot[2..]
                .chars()
                .all(|character| character.is_ascii_digit()) =>
        {
            Some(RuntimeResourceKind::VertexBuffer)
        }
        _ => None,
    }
}

fn slot_rank(slot: &str) -> u8 {
    match resource_kind_for_slot(slot) {
        Some(RuntimeResourceKind::PositionVb) => 0,
        Some(RuntimeResourceKind::DrawVb | RuntimeResourceKind::VertexBuffer) => 1,
        Some(RuntimeResourceKind::IndexBuffer) => 2,
        Some(RuntimeResourceKind::Texture) => 3,
        Some(RuntimeResourceKind::Shader) | None => 4,
    }
}

fn section_is_position(section_name: &str) -> bool {
    section_name
        .to_ascii_lowercase()
        .strip_prefix("textureoverride")
        .is_some_and(|name| name.ends_with("position"))
}

/// Harvest raw resource hashes from a single INI. This compatibility helper
/// intentionally does not infer a resource type and must not feed KeyViewer.
pub fn harvest_hashes_from_ini(file_path: &Path) -> Result<Vec<HarvestedHash>, AppError> {
    let bytes = fs::read(file_path)?;
    let (text, _, _) =
        crate::modules::library::application::ini::document::decode_ini_bytes(&bytes);
    Ok(harvest_hashes_from_text(&text, file_path))
}

fn harvest_hashes_from_text(text: &str, file_path: &Path) -> Vec<HarvestedHash> {
    let mut results = Vec::new();
    let mut current_section: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            current_section = TEXTURE_OVERRIDE_SECTION_RE
                .captures(trimmed)
                .map(|captures| captures[1].to_string())
                .filter(|name| !is_denylisted(name));
            continue;
        }
        let Some(section_name) = current_section.as_ref() else {
            continue;
        };
        if let Some(captures) = HASH_RE.captures(trimmed) {
            results.push(HarvestedHash {
                hash: captures[1].to_ascii_lowercase(),
                section_name: section_name.clone(),
                file_path: file_path.to_path_buf(),
            });
        }
    }
    results
}

/// Harvest observer-ready, typed targets from one INI file.
pub fn harvest_targets_from_ini(
    file_path: &Path,
    capabilities: &HarvestCapabilities,
) -> Result<Vec<HarvestedTarget>, AppError> {
    let bytes = fs::read(file_path)?;
    let (text, _, _) =
        crate::modules::library::application::ini::document::decode_ini_bytes(&bytes);
    Ok(harvest_targets_from_text(&text, file_path, capabilities))
}

fn harvest_targets_from_text(
    text: &str,
    file_path: &Path,
    capabilities: &HarvestCapabilities,
) -> Vec<HarvestedTarget> {
    let mut targets = Vec::new();
    let mut current: Option<OverrideSection> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if let Some(section) = current.take() {
                targets.extend(section.finish(file_path, capabilities));
            }
            current = TEXTURE_OVERRIDE_SECTION_RE
                .captures(trimmed)
                .map(|captures| captures[1].to_string())
                .filter(|name| !is_denylisted(name))
                .map(OverrideSection::new);
            continue;
        }
        if let Some(section) = current.as_mut() {
            section.observe(trimmed);
        }
    }
    if let Some(section) = current {
        targets.extend(section.finish(file_path, capabilities));
    }
    targets
}

/// Compatibility hash map for callers that need the raw scanner inventory.
pub fn harvest_hashes_from_mod(
    mod_path: &Path,
) -> Result<HashMap<String, Vec<HarvestedHash>>, AppError> {
    let mut hash_map: HashMap<String, Vec<HarvestedHash>> = HashMap::new();
    for ini_path in list_ini_files(mod_path)? {
        match harvest_hashes_from_ini(&ini_path) {
            Ok(hashes) => {
                for hash in hashes {
                    hash_map.entry(hash.hash.clone()).or_default().push(hash);
                }
            }
            Err(error) => log::warn!(
                "[keyviewer] Failed to harvest hashes from {}: {error}",
                ini_path.display()
            ),
        }
    }
    Ok(hash_map)
}

/// Everything one pass over a mod's INI files yields.
#[derive(Debug, Clone, Default)]
pub struct ModHarvest {
    pub targets: Vec<HarvestedTarget>,
    pub keybinds: Vec<crate::modules::library::application::ini::document::KeyBinding>,
    /// Digest of every effective INI read for this mod. The generator uses
    /// these to distinguish a real INI edit from an unchanged watcher burst.
    pub ini_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IniFileSnapshot {
    path: PathBuf,
    len: u64,
    modified: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct HarvestCacheKey {
    mod_path: String,
    capability_slots: String,
}

#[derive(Debug, Clone)]
struct CachedModHarvest {
    files: Vec<IniFileSnapshot>,
    harvest: ModHarvest,
}

/// A sync already has to enumerate the effective INI files, but it should not
/// decode and parse unchanged files on every coalesced watcher event. This is
/// process-local by design: the existing manifest still remains the durable
/// source of published state.
static HARVEST_CACHE: LazyLock<Mutex<HashMap<HarvestCacheKey, CachedModHarvest>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn cache_path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn snapshot_ini_files(ini_files: &[PathBuf]) -> Result<Vec<IniFileSnapshot>, std::io::Error> {
    ini_files
        .iter()
        .map(|path| {
            let metadata = fs::metadata(path)?;
            Ok(IniFileSnapshot {
                path: path.clone(),
                len: metadata.len(),
                modified: metadata.modified().ok(),
            })
        })
        .collect()
}

/// Keep the in-memory cache bounded to currently enabled mods. Deleted and
/// disabled mods cannot contribute to the next generation, so retaining their
/// parsed documents provides no value.
pub fn retain_cached_mods<'a>(active_mod_paths: impl IntoIterator<Item = &'a Path>) {
    let active: HashSet<_> = active_mod_paths.into_iter().map(cache_path_key).collect();
    let mut cache = HARVEST_CACHE
        .lock()
        .expect("KeyViewer harvest cache lock poisoned");
    cache.retain(|key, _| active.contains(&key.mod_path));
}

/// Harvest targets and key bindings from one mod with exactly one INI read per
/// file. This runs during artifact generation, never from the Present loop.
pub fn harvest_mod(
    mod_path: &Path,
    capabilities: &HarvestCapabilities,
) -> Result<ModHarvest, AppError> {
    let ini_files = list_ini_files(mod_path)?;
    let key = HarvestCacheKey {
        mod_path: cache_path_key(mod_path),
        capability_slots: capabilities.cache_key(),
    };
    let snapshots = match snapshot_ini_files(&ini_files) {
        Ok(snapshots) => snapshots,
        // A file can disappear between a watcher event and the sync. Preserve
        // the prior best-effort harvest behaviour, but do not cache uncertainty.
        Err(error) => {
            log::warn!(
                "[keyviewer] Could not snapshot INI files in {}: {error}",
                mod_path.display()
            );
            return harvest_ini_files(ini_files, capabilities);
        }
    };

    if let Some(cached) = HARVEST_CACHE
        .lock()
        .expect("KeyViewer harvest cache lock poisoned")
        .get(&key)
        .filter(|cached| cached.files == snapshots)
        .cloned()
    {
        return Ok(cached.harvest);
    }

    let harvest = harvest_ini_files(ini_files, capabilities)?;
    HARVEST_CACHE
        .lock()
        .expect("KeyViewer harvest cache lock poisoned")
        .insert(
            key,
            CachedModHarvest {
                files: snapshots,
                harvest: harvest.clone(),
            },
        );
    Ok(harvest)
}

fn harvest_ini_files(
    ini_files: Vec<PathBuf>,
    capabilities: &HarvestCapabilities,
) -> Result<ModHarvest, AppError> {
    use crate::modules::library::application::ini::document;

    let mut harvest = ModHarvest::default();
    for ini_path in ini_files {
        let bytes = match fs::read(&ini_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                log::warn!("[keyviewer] Failed to read {}: {error}", ini_path.display());
                continue;
            }
        };
        harvest
            .ini_fingerprints
            .push(format!("{:x}", Sha256::digest(&bytes)));
        let (text, _, _) = document::decode_ini_bytes(&bytes);
        harvest
            .targets
            .extend(harvest_targets_from_text(&text, &ini_path, capabilities));
        if bytes.len() as u64 <= document::MAX_PARSEABLE_INI_BYTES {
            harvest
                .keybinds
                .extend(document::parse_ini_document(&ini_path, &bytes).key_bindings);
        }
    }
    Ok(harvest)
}

/// Harvest key bindings only for compatibility callers.
pub fn harvest_keybinds_from_mod(
    mod_path: &Path,
) -> Result<Vec<crate::modules::library::application::ini::document::KeyBinding>, AppError> {
    let ini_files = list_ini_files(mod_path)?;
    let mut all_keybinds = Vec::new();
    for ini_path in ini_files {
        if let Ok(doc) =
            crate::modules::library::application::ini::document::read_ini_document(&ini_path)
        {
            all_keybinds.extend(doc.key_bindings);
        }
    }
    Ok(all_keybinds)
}
