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
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, LazyLock, Mutex,
};

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

    pub(crate) fn cache_key(&self) -> String {
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
    content_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct HarvestCacheKey {
    mod_path: String,
    capability_slots: String,
}

#[derive(Debug, Clone)]
struct CachedModHarvest {
    files: Vec<IniFileSnapshot>,
    harvest: Arc<ModHarvest>,
}

type IniContentDigest = sha2::digest::Output<Sha256>;

struct IniFileContent {
    path: PathBuf,
    bytes: Vec<u8>,
    content_digest: IniContentDigest,
}

type IniFileContents = Vec<IniFileContent>;
type IniSnapshot = (Vec<IniFileSnapshot>, IniFileContents);

/// A sync already has to enumerate the effective INI files, but it should not
/// decode and parse unchanged files on every coalesced watcher event. This is
/// process-local by design: the existing manifest still remains the durable
/// source of published state.
static HARVEST_CACHE: LazyLock<Mutex<HashMap<HarvestCacheKey, CachedModHarvest>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static HARVEST_CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static HARVEST_CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
static HARVESTED_MODS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HarvestCacheMetrics {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub harvested_mods: u64,
    pub retained_entries: usize,
}

pub(crate) fn harvest_cache_metrics() -> HarvestCacheMetrics {
    HarvestCacheMetrics {
        cache_hits: HARVEST_CACHE_HITS.load(Ordering::Relaxed),
        cache_misses: HARVEST_CACHE_MISSES.load(Ordering::Relaxed),
        harvested_mods: HARVESTED_MODS.load(Ordering::Relaxed),
        retained_entries: HARVEST_CACHE
            .lock()
            .expect("KeyViewer harvest cache lock poisoned")
            .len(),
    }
}

#[cfg(test)]
#[derive(Default)]
struct HarvestIoCounts {
    reads: usize,
    digests: usize,
}

#[cfg(test)]
static HARVEST_READ_COUNTS: LazyLock<Mutex<HashMap<String, HarvestIoCounts>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn cache_path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

#[cfg(test)]
fn record_harvest_read(path: &Path) {
    HARVEST_READ_COUNTS
        .lock()
        .expect("KeyViewer harvest read counter lock poisoned")
        .entry(cache_path_key(path))
        .or_default()
        .reads += 1;
}

#[cfg(test)]
pub(crate) fn harvest_read_count(path: &Path) -> usize {
    HARVEST_READ_COUNTS
        .lock()
        .expect("KeyViewer harvest read counter lock poisoned")
        .get(&cache_path_key(path))
        .map(|counts| counts.reads)
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) fn harvest_digest_count(path: &Path) -> usize {
    HARVEST_READ_COUNTS
        .lock()
        .expect("KeyViewer harvest read counter lock poisoned")
        .get(&cache_path_key(path))
        .map(|counts| counts.digests)
        .unwrap_or_default()
}

fn ini_content_digest(path: &Path, bytes: &[u8]) -> IniContentDigest {
    #[cfg(test)]
    {
        HARVEST_READ_COUNTS
            .lock()
            .expect("KeyViewer harvest read counter lock poisoned")
            .entry(cache_path_key(path))
            .or_default()
            .digests += 1;
    }
    #[cfg(not(test))]
    let _ = path;
    Sha256::digest(bytes)
}

fn snapshot_ini_files(ini_files: &[PathBuf]) -> Result<IniSnapshot, std::io::Error> {
    let mut snapshots = Vec::with_capacity(ini_files.len());
    let mut contents = Vec::with_capacity(ini_files.len());
    for path in ini_files {
        let metadata = fs::metadata(path)?;
        #[cfg(test)]
        record_harvest_read(path);
        let bytes = fs::read(path)?;
        let content_digest = ini_content_digest(path, &bytes);
        snapshots.push(IniFileSnapshot {
            path: path.clone(),
            len: metadata.len(),
            modified: metadata.modified().ok(),
            content_digest: format!("{content_digest:x}"),
        });
        contents.push(IniFileContent {
            path: path.clone(),
            bytes,
            content_digest,
        });
    }
    Ok((snapshots, contents))
}

/// Keep one game's cache bounded to its currently enabled mods without
/// evicting warm entries belonging to another game's Mods root.
pub fn retain_cached_mods<'a>(
    mods_root: &Path,
    capabilities: &HarvestCapabilities,
    active_mod_paths: impl IntoIterator<Item = &'a Path>,
) {
    let active: HashSet<_> = active_mod_paths.into_iter().map(cache_path_key).collect();
    let mods_root = cache_path_key(mods_root);
    let mods_prefix = format!("{mods_root}/");
    let capability_slots = capabilities.cache_key();
    let mut cache = HARVEST_CACHE
        .lock()
        .expect("KeyViewer harvest cache lock poisoned");
    cache.retain(|key, _| {
        !key.mod_path.starts_with(&mods_prefix)
            || (active.contains(&key.mod_path) && key.capability_slots == capability_slots)
    });
}

/// Discard every capability-specific snapshot for one changed or disabled mod.
/// Scoped synchronization calls this before replacing its contribution so a
/// caller-provided change signal is authoritative even when file metadata was
/// preserved by a copy or restore operation.
pub fn invalidate_cached_mod(mod_path: &Path) {
    invalidate_cached_mods([mod_path]);
}

/// Discard all capability variants for the lexical old/new paths in one scope.
/// Normalize and deduplicate before taking the cache lock; other roots stay warm.
pub fn invalidate_cached_mods<'a>(mod_paths: impl IntoIterator<Item = &'a Path>) {
    let mod_paths: HashSet<_> = mod_paths.into_iter().map(cache_path_key).collect();
    if mod_paths.is_empty() {
        return;
    }
    let mut cache = HARVEST_CACHE
        .lock()
        .expect("KeyViewer harvest cache lock poisoned");
    if mod_paths.len() == 1 {
        let changed_path = mod_paths.iter().next().expect("one deduplicated path");
        cache.retain(|key, _| key.mod_path != *changed_path);
    } else {
        cache.retain(|key, _| !mod_paths.contains(&key.mod_path));
    }
}

/// Evict every parsed INI snapshot below a composition root that is no longer
/// retained. The caller supplies the same canonical root key used by the
/// composition cache, keeping multi-game memory bounded together.
pub fn evict_cached_root_by_key(mods_root_key: &str) {
    let raw_root = mods_root_key.replace('\\', "/").to_ascii_lowercase();
    let preserve_unc_prefix = raw_root.starts_with("//");
    let mut normalized_root = String::with_capacity(raw_root.len());
    for ch in raw_root.chars() {
        if ch == '/'
            && normalized_root.ends_with('/')
            && !(preserve_unc_prefix && normalized_root.len() == 1)
        {
            continue;
        }
        normalized_root.push(ch);
    }
    let root = normalized_root.trim_end_matches('/');
    let prefix = format!("{root}/");
    HARVEST_CACHE
        .lock()
        .expect("KeyViewer harvest cache lock poisoned")
        .retain(|key, _| key.mod_path != root && !key.mod_path.starts_with(&prefix));
}

#[cfg(test)]
pub(crate) fn cached_entry_count_for_root(mods_root: &Path) -> usize {
    let root = cache_path_key(mods_root);
    let prefix = format!("{root}/");
    HARVEST_CACHE
        .lock()
        .expect("KeyViewer harvest cache lock poisoned")
        .keys()
        .filter(|key| key.mod_path == root || key.mod_path.starts_with(&prefix))
        .count()
}

/// Harvest targets and key bindings from one mod with exactly one INI read per
/// file. This runs during artifact generation, never from the Present loop.
pub fn harvest_mod(
    mod_path: &Path,
    capabilities: &HarvestCapabilities,
) -> Result<Arc<ModHarvest>, AppError> {
    let ini_files = list_ini_files(mod_path)?;
    let key = HarvestCacheKey {
        mod_path: cache_path_key(mod_path),
        capability_slots: capabilities.cache_key(),
    };
    let (snapshots, contents) = match snapshot_ini_files(&ini_files) {
        Ok(snapshot) => snapshot,
        // A file can disappear between a watcher event and the sync. Preserve
        // the prior best-effort harvest behaviour, but do not cache uncertainty.
        Err(error) => {
            log::warn!(
                "[keyviewer] Could not snapshot INI files in {}: {error}",
                mod_path.display()
            );
            HARVEST_CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
            let harvest = Arc::new(harvest_ini_files(ini_files, capabilities)?);
            HARVESTED_MODS.fetch_add(1, Ordering::Relaxed);
            return Ok(harvest);
        }
    };

    let cached_harvest = {
        let cache = HARVEST_CACHE
            .lock()
            .expect("KeyViewer harvest cache lock poisoned");
        cache
            .get(&key)
            .filter(|cached| cached.files == snapshots)
            .map(|cached| Arc::clone(&cached.harvest))
    };
    if let Some(harvest) = cached_harvest {
        HARVEST_CACHE_HITS.fetch_add(1, Ordering::Relaxed);
        return Ok(harvest);
    }

    HARVEST_CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
    let harvest = Arc::new(harvest_ini_contents(contents, capabilities));
    HARVESTED_MODS.fetch_add(1, Ordering::Relaxed);
    HARVEST_CACHE
        .lock()
        .expect("KeyViewer harvest cache lock poisoned")
        .insert(
            key,
            CachedModHarvest {
                files: snapshots,
                harvest: Arc::clone(&harvest),
            },
        );
    Ok(harvest)
}

fn harvest_ini_files(
    ini_files: Vec<PathBuf>,
    capabilities: &HarvestCapabilities,
) -> Result<ModHarvest, AppError> {
    let contents = ini_files
        .into_iter()
        .filter_map(|ini_path| {
            #[cfg(test)]
            record_harvest_read(&ini_path);
            let bytes = match fs::read(&ini_path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    log::warn!("[keyviewer] Failed to read {}: {error}", ini_path.display());
                    return None;
                }
            };
            let content_digest = ini_content_digest(&ini_path, &bytes);
            Some(IniFileContent {
                path: ini_path,
                bytes,
                content_digest,
            })
        })
        .collect();
    Ok(harvest_ini_contents(contents, capabilities))
}

fn harvest_ini_contents(
    contents: IniFileContents,
    capabilities: &HarvestCapabilities,
) -> ModHarvest {
    use crate::modules::library::application::ini::document;

    let mut harvest = ModHarvest::default();
    for IniFileContent {
        path: ini_path,
        bytes,
        content_digest,
    } in contents
    {
        harvest.ini_fingerprints.push(format!("{content_digest:x}"));
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
    harvest
}

#[cfg(test)]
#[path = "harvest_dispatch_benchmark.rs"]
mod dispatch_benchmark;

#[cfg(test)]
mod cache_benchmark {
    use super::*;
    use std::time::Instant;
    use tempfile::TempDir;

    fn fixture_cache(roots: &[PathBuf]) -> HashMap<HarvestCacheKey, CachedModHarvest> {
        let prefixes: Vec<_> = roots
            .iter()
            .map(|root| format!("{}/", cache_path_key(root)))
            .collect();
        HARVEST_CACHE
            .lock()
            .unwrap()
            .iter()
            .filter(|(key, _)| {
                prefixes
                    .iter()
                    .any(|prefix| key.mod_path.starts_with(prefix))
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    fn assert_same_cache(
        left: &HashMap<HarvestCacheKey, CachedModHarvest>,
        right: &HashMap<HarvestCacheKey, CachedModHarvest>,
    ) {
        assert_eq!(left.len(), right.len());
        for (key, left_value) in left {
            let right_value = right.get(key).expect("identical cache key set");
            assert_eq!(left_value.files, right_value.files);
            assert!(Arc::ptr_eq(&left_value.harvest, &right_value.harvest));
        }
    }

    #[test]
    #[ignore = "manual actual-cache two-game invalidation/harvest benchmark"]
    fn benchmark_actual_cache_batch_invalidation() {
        const MODS_PER_GAME: usize = 2_048;
        const PAIRS: usize = 5;
        let fixture = TempDir::new().unwrap();
        let roots = [
            fixture.path().join("GameA/Mods"),
            fixture.path().join("GameB/Mods"),
        ];
        let capabilities = [
            HarvestCapabilities::from_callback_slots(["vb0"]),
            HarvestCapabilities::from_callback_slots(["ib"]),
        ];
        let mut paths = Vec::with_capacity(MODS_PER_GAME);
        let content = format!(
            "[TextureOverrideBody]\nhash = df65bb00\nvb0 = ResourceBody\nib = ResourceIndex\n[KeyBody]\nkey = CTRL+K\n{}",
            "; representative resource configuration padding\n".repeat(80)
        );
        for (game_index, root) in roots.iter().enumerate() {
            for mod_index in 0..MODS_PER_GAME {
                let path = root.join(format!(
                    "Group{:02}/Nested/Mod{mod_index:04}",
                    mod_index % 16
                ));
                fs::create_dir_all(&path).unwrap();
                fs::write(path.join("mod.ini"), &content).unwrap();
                for capability in &capabilities {
                    harvest_mod(&path, capability).unwrap();
                }
                if game_index == 0 {
                    paths.push(path);
                }
            }
        }
        let baseline = fixture_cache(&roots);
        assert_eq!(baseline.len(), MODS_PER_GAME * 4);
        let capacity = HARVEST_CACHE.lock().unwrap().capacity();
        for (case, scope) in [("one_mod", &paths[..1]), ("root", paths.as_slice())] {
            for pair in 0..PAIRS {
                let mut expected_cache = None;
                let mut expected_output: Option<Vec<Arc<ModHarvest>>> = None;
                let modes = if pair % 2 == 0 {
                    [false, true]
                } else {
                    [true, false]
                };
                for batch in modes {
                    {
                        let mut cache = HARVEST_CACHE.lock().unwrap();
                        cache.extend(baseline.clone());
                    }
                    let started = Instant::now();
                    if batch {
                        invalidate_cached_mods(
                            scope.iter().chain(scope.iter()).map(PathBuf::as_path),
                        );
                    } else {
                        // Preserve the pre-batch production algorithm exactly.
                        for path in scope.iter().chain(scope.iter()) {
                            let key = cache_path_key(path);
                            HARVEST_CACHE
                                .lock()
                                .unwrap()
                                .retain(|entry, _| entry.mod_path != key);
                        }
                    }
                    let invalidation_us = started.elapsed().as_secs_f64() * 1_000_000.0;
                    let remaining = fixture_cache(&roots);
                    assert_eq!(
                        remaining.len(),
                        baseline.len() - scope.len() * capabilities.len()
                    );
                    assert_eq!(cached_entry_count_for_root(&roots[1]), MODS_PER_GAME * 2);
                    for (key, value) in &remaining {
                        assert!(Arc::ptr_eq(&baseline[key].harvest, &value.harvest));
                    }
                    if let Some(expected) = &expected_cache {
                        assert_same_cache(expected, &remaining);
                    } else {
                        expected_cache = Some(remaining);
                    }
                    let started = Instant::now();
                    let output: Vec<_> = scope
                        .iter()
                        .map(|path| harvest_mod(path, &capabilities[0]).unwrap())
                        .collect();
                    let harvest_us = started.elapsed().as_secs_f64() * 1_000_000.0;
                    if let Some(expected) = &expected_output {
                        for (left, right) in expected.iter().zip(&output) {
                            assert_eq!(left.targets, right.targets);
                            assert_eq!(left.keybinds, right.keybinds);
                            assert_eq!(left.ini_fingerprints, right.ini_fingerprints);
                        }
                    } else {
                        expected_output = Some(output);
                    }
                    println!(
                        "{}",
                        serde_json::json!({
                            "case": case, "pair": pair + 1, "batch": batch,
                            "fixture_entries": baseline.len(), "cache_capacity": capacity,
                            "ini_bytes_per_mod": content.len(), "scope_mods": scope.len(),
                            "input_paths": scope.len() * 2,
                            "retain_passes": if batch { 1 } else { scope.len() * 2 },
                            "invalidation_us": invalidation_us, "harvest_us": harvest_us,
                            "invalidation_and_harvest_us": invalidation_us + harvest_us,
                            "allocation_measurement": "not instrumented",
                            "exact_cache_and_harvest_parity": true
                        })
                    );
                }
            }
        }
        evict_cached_root_by_key(&cache_path_key(fixture.path()));
    }
}
