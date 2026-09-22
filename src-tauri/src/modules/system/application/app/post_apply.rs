use crate::modules::automation::application::hotkeys::HotkeyConfig;
use crate::modules::automation::application::keyviewer::generator;
use crate::modules::automation::application::keyviewer::harvester;
use crate::modules::automation::application::keyviewer::matcher;
use crate::modules::matching::application::deep_matcher::models::types::{
    RuntimeResourceKind, RuntimeTarget,
};
use crate::modules::reconciliation::api::ActivationAuthority;
use crate::modules::system::domain::mod_path::ModFolderPath;
use crate::shared::errors::AppError;
use sha2::{Digest, Sha256};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use walkdir::WalkDir;

static KEYVIEWER_SYNC_LOCKS: OnceLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    OnceLock::new();
static KEYVIEWER_SYNC_REVISIONS: OnceLock<Mutex<HashMap<String, RuntimeSyncRevisionState>>> =
    OnceLock::new();
static KEYVIEWER_RUNTIME_SYNC_SNAPSHOTS: OnceLock<Mutex<HashMap<String, RuntimeSyncSnapshot>>> =
    OnceLock::new();
static RUNTIME_PREFLIGHT_CACHE: OnceLock<Mutex<HashMap<String, CachedRuntimePreflight>>> =
    OnceLock::new();
static RUNTIME_PREFLIGHT_CACHE_CLOCK: AtomicU64 = AtomicU64::new(0);
static KEYVIEWER_COMPOSITION_CACHE: OnceLock<Mutex<HashMap<String, CachedGameComposition>>> =
    OnceLock::new();
static KEYVIEWER_COMPOSITION_CACHE_CLOCK: AtomicU64 = AtomicU64::new(0);
const KNOWN_LEGACY_KEYVIEWER_SHA256: &str =
    "8741313bbaaae887483c7b3b6a5e7777d23145754f762efc5beaaa6a636c5579";
const LEGACY_KEYVIEWER_BACKUP_SUFFIX: &str = ".emmm-legacy-backup";

const KEYVIEWER_MANIFEST_FILE: &str = "manifest.json";
const KEYVIEWER_MIGRATION_JOURNAL_FILE: &str = "duplicate-migration.json";
const KEYVIEWER_RESOURCE_ROOT: &str = "generations";
const KEYVIEWER_MANIFEST_VERSION: u8 = 2;
// A complete per-game composition is required to publish an atomic KeyViewer
// artifact. Bound the number of warm games rather than the number of mods: a
// per-game mod cap makes every incremental update degrade to a full rebuild as
// soon as a large library crosses the threshold it needs the cache most.
const MAX_CACHED_GAMES: usize = 2;
const MAX_CACHED_RUNTIME_PREFLIGHTS: usize = 8;
const MAX_SCOPED_ROOTS: usize = 250;
// In-memory scans use a bounded cadence; filesystem harvests check every mod.
const RUNTIME_SUPERSESSION_CHECK_INTERVAL: usize = 64;

/// The reason an overlay snapshot was requested. Keeping this typed makes it
/// possible to force only authority-boundary refreshes while normal watcher
/// bursts remain idempotent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySyncCause {
    Startup,
    FirstIndex,
    GameActivated,
    ModsRootChanged,
    ImporterRootChanged,
    EffectiveModsChanged,
    EffectiveIniChanged,
    CollectionApplied,
    SafeModeChanged,
    SettingsChanged,
    CatalogChanged,
    Recovery,
}

impl OverlaySyncCause {
    fn forces_publish(self) -> bool {
        matches!(
            self,
            Self::FirstIndex | Self::ModsRootChanged | Self::ImporterRootChanged | Self::Recovery
        )
    }
}

/// Resulting runtime state for one changed mod. The stable database ID lets a
/// disable remove the old contribution even when the folder was renamed as
/// part of the same mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeModOutcome {
    Enabled,
    Disabled,
}

/// One committed mod mutation supplied to an incremental runtime sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeModChange {
    pub mod_id: String,
    pub folder_path: ModFolderPath,
    pub outcome: RuntimeModOutcome,
}

/// Input scope for a KeyViewer runtime generation. Existing callers use
/// `Full`; mutation callers can supply their committed per-mod outcomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSyncRequest {
    Full,
    Scoped { changes: Vec<RuntimeModChange> },
    ScopedRoots { roots: Vec<ModFolderPath> },
}

impl RuntimeSyncRequest {
    fn merge_pending(self, newer: Self) -> Self {
        match (self, newer) {
            (Self::Full, _) | (_, Self::Full) => Self::Full,
            (Self::Scoped { changes: older }, Self::Scoped { changes: newer }) => {
                let mut merged = BTreeMap::new();
                for change in older.into_iter().chain(newer) {
                    merged.insert(change.mod_id.clone(), change);
                }
                Self::Scoped {
                    changes: merged.into_values().collect(),
                }
            }
            (Self::ScopedRoots { roots: older }, Self::ScopedRoots { roots: newer }) => {
                let mut merged = BTreeMap::new();
                for root in older.into_iter().chain(newer) {
                    let key = root.as_stored().replace('\\', "/").to_ascii_lowercase();
                    merged.insert(key, root);
                }
                if merged.len() > MAX_SCOPED_ROOTS {
                    Self::Full
                } else {
                    Self::ScopedRoots {
                        roots: merged.into_values().collect(),
                    }
                }
            }
            // A direct-ID mutation and a subtree mutation can describe the
            // same rename from opposite sides. Without old/new root identity,
            // merging them incrementally is ambiguous; rebuild from DB+disk.
            (Self::Scoped { .. }, Self::ScopedRoots { .. })
            | (Self::ScopedRoots { .. }, Self::Scoped { .. }) => Self::Full,
        }
    }
}

#[derive(Debug, Clone)]
struct RuntimeSyncRevisionState {
    revision: u64,
    pending: Option<RuntimeSyncRequest>,
}

/// Publication is deliberately separate from input replay. A published
/// snapshot is not proof that 3DMigoto has already loaded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSyncPublication {
    Published,
    Unchanged,
    Skipped,
    FailedBeforePublish,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeReloadOutcome {
    NotRequired,
    ReloadSent {
        binding: String,
    },
    NeedsManualReload {
        binding: Option<String>,
        reason: String,
    },
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSyncPublicationStatus {
    Published,
    Unchanged,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeReloadStatus {
    NotRequired,
    Sent,
    Manual,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, PartialEq, Eq)]
pub struct KeyViewerRuntimeDiagnostics {
    pub last_sync_unix_ms: Option<i64>,
    pub publication: Option<RuntimeSyncPublicationStatus>,
    pub reload: Option<RuntimeReloadStatus>,
    pub reload_binding: Option<String>,
    pub cleanup_automatic_disabled: bool,
}

#[derive(Debug, Clone)]
struct RuntimeSyncSnapshot {
    last_sync_unix_ms: i64,
    publication: RuntimeSyncPublicationStatus,
    reload: RuntimeReloadStatus,
    reload_binding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSyncResult {
    pub cause: OverlaySyncCause,
    pub publication: RuntimeSyncPublication,
    pub reload: RuntimeReloadOutcome,
    pub failure: Option<String>,
}

impl RuntimeSyncResult {
    /// Preserve a failed sync as an explicit outcome for diagnostics while
    /// allowing mutation/reconcile callers to keep their retry semantics.
    pub fn ensure_success(self) -> Result<Self, AppError> {
        match self.failure.as_deref() {
            Some(message) => Err(AppError::Internal(message.to_string())),
            None => Ok(self),
        }
    }

    /// A failed publication needs another event-driven attempt. A manual reload
    /// request does not: its files are already published, but the configured
    /// reload binding could not be resolved or sent.
    pub fn requires_retry(&self) -> bool {
        self.failure.is_some()
    }

    pub fn needs_manual_reload(&self) -> bool {
        matches!(self.reload, RuntimeReloadOutcome::NeedsManualReload { .. })
    }

    /// Message suitable for a committed-mutation warning. This deliberately
    /// includes a manual reload outcome: persistence succeeded, but the user
    /// still needs to reload 3DMigoto before the game can consume the snapshot.
    pub fn diagnostic_message(&self) -> Option<String> {
        self.failure.clone().or_else(|| match &self.reload {
            RuntimeReloadOutcome::NeedsManualReload { binding, reason } => {
                let instruction = binding.as_deref().map_or_else(
                    || "reload the active game's 3DMigoto configuration manually".to_string(),
                    |key| format!("press {key} in the active game to reload its configuration"),
                );
                Some(format!("Manual reload required: {instruction}. {reason}"))
            }
            RuntimeReloadOutcome::NotRequired | RuntimeReloadOutcome::ReloadSent { .. } => None,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct KeyViewerManifest {
    version: u8,
    fingerprint: String,
    /// Kept for compatibility with older manifests; the active resources now
    /// always live directly under `.emmm_data/generations/`.
    generation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PostApplyPublication {
    Published,
    Unchanged,
    Skipped,
}

#[derive(Debug, Clone)]
struct CachedModContribution {
    folder_path: ModFolderPath,
    harvest: Arc<harvester::ModHarvest>,
}

#[derive(Debug, Clone)]
struct GameCompositionCache {
    mods_root: String,
    capability_slots: String,
    mods: BTreeMap<String, CachedModContribution>,
}

#[derive(Debug, Clone)]
struct CachedGameComposition {
    cache: Arc<GameCompositionCache>,
    last_used: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompositionMode {
    Full,
    Scoped,
    ScopedRoots,
    FullFallback,
}

#[derive(Debug)]
struct PreparedModComposition {
    cache: Arc<GameCompositionCache>,
    mode: CompositionMode,
    harvested_mods: usize,
}

/// Temporarily owns one warm composition while a scoped update is prepared.
/// If any fallible step exits early, Drop returns the coherent partial cache
/// so a transient INI or DB error does not force the next retry to rebuild all
/// enabled mods.
struct CompositionCacheLease {
    game_id: String,
    cache: Option<GameCompositionCache>,
}

impl CompositionCacheLease {
    fn new(game_id: &str, cache: GameCompositionCache) -> Self {
        Self {
            game_id: game_id.to_string(),
            cache: Some(cache),
        }
    }

    fn cache_mut(&mut self) -> &mut GameCompositionCache {
        self.cache
            .as_mut()
            .expect("composition cache lease is active")
    }

    fn into_prepared(
        mut self,
        mode: CompositionMode,
        harvested_mods: usize,
    ) -> PreparedModComposition {
        prepared_mod_composition(
            self.cache
                .take()
                .expect("composition cache lease is active"),
            mode,
            harvested_mods,
        )
    }
}

impl Drop for CompositionCacheLease {
    fn drop(&mut self) {
        let Some(cache) = self.cache.take() else {
            return;
        };
        let prepared = prepared_mod_composition(cache, CompositionMode::Scoped, 0);
        if let Err(error) = store_mod_composition(&self.game_id, &prepared) {
            log::error!(
                "Could not restore KeyViewer composition cache after a scoped failure for '{}': {error}",
                self.game_id
            );
        }
    }
}

fn prepared_mod_composition(
    cache: GameCompositionCache,
    mode: CompositionMode,
    harvested_mods: usize,
) -> PreparedModComposition {
    PreparedModComposition {
        cache: Arc::new(cache),
        mode,
        harvested_mods,
    }
}

type CurrentGenerationCheck<'a> = &'a (dyn Fn() -> Result<bool, AppError> + Sync);

fn composition_checkpoint(
    is_current: CurrentGenerationCheck<'_>,
    game_id: &str,
    cancellation_point: &'static str,
    completed_mods: usize,
) -> Result<bool, AppError> {
    let current = is_current()?;
    if !current {
        let metrics = harvester::harvest_cache_metrics();
        log::info!(
            "[post_apply] KeyViewer composition superseded game={game_id} cancellation_point={cancellation_point} completed_mods={completed_mods} cache_hits_total={} cache_misses_total={} harvested_count_total={} estimated_retained_entries={}",
            metrics.cache_hits,
            metrics.cache_misses,
            metrics.harvested_mods,
            metrics.retained_entries,
        );
    }
    Ok(current)
}

fn composition_checkpoint_due(index: usize) -> bool {
    index.is_multiple_of(RUNTIME_SUPERSESSION_CHECK_INTERVAL)
}

/// Runtime facts read from the installed 3DMigoto configuration once per
/// artifact generation. None of this is consulted by the Present loop.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimePreflight {
    runtime_mods_root: PathBuf,
    runtime_include_roots: Vec<PathBuf>,
    text_namespace: String,
    renderer_available: bool,
    callback_slots: HashSet<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimePreflightFileSnapshot {
    path: String,
    identity: Option<String>,
    len: u64,
    modified: Option<std::time::SystemTime>,
    content_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimePreflightSnapshot {
    importer_identity: Option<String>,
    mods_identity: Option<String>,
    files: Vec<RuntimePreflightFileSnapshot>,
}

#[derive(Debug, Clone)]
struct CachedRuntimePreflight {
    snapshot: RuntimePreflightSnapshot,
    preflight: RuntimePreflight,
    last_used: u64,
}

fn strip_ini_comment(line: &str) -> &str {
    line.split([';', '#']).next().unwrap_or_default().trim()
}

fn parse_include_recursive_roots(config: &str) -> Vec<String> {
    let mut in_include = false;
    let mut roots = Vec::new();
    for raw_line in config.lines() {
        let line = strip_ini_comment(raw_line);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_include = line[1..line.len() - 1]
                .trim()
                .eq_ignore_ascii_case("include");
            continue;
        }
        if !in_include {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("include_recursive") {
            roots.push(value.trim().trim_matches(['\"', '\'']).to_string());
        }
    }
    roots
}

fn has_ini_assignment(content: &str, expected_key: &str, expected_value: &str) -> bool {
    content.lines().any(|raw_line| {
        let line = strip_ini_comment(raw_line);
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        key.trim().eq_ignore_ascii_case(expected_key)
            && value.trim().eq_ignore_ascii_case(expected_value)
    })
}

fn canonical_or_original(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    canonical_or_original(path).starts_with(canonical_or_original(root))
}

fn runtime_preflight_snapshot(
    importer_root: &Path,
    configured_mods_root: &Path,
) -> Option<RuntimePreflightSnapshot> {
    let mut files = Vec::new();
    for entry in WalkDir::new(importer_root)
        .follow_links(false)
        .max_depth(5)
        .into_iter()
    {
        let entry = entry.ok()?;
        if !entry.file_type().is_file()
            || path_is_within(entry.path(), configured_mods_root)
            || !entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"))
        {
            continue;
        }
        if files.len() >= 1_000 {
            return None;
        }
        let metadata = entry.metadata().ok()?;
        let content = std::fs::read(entry.path()).ok()?;
        files.push(RuntimePreflightFileSnapshot {
            path: crate::shared::path_key::canonical_path_key_for_path(entry.path()),
            identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(entry.path()),
            len: metadata.len(),
            modified: metadata.modified().ok(),
            content_digest: format!("{:x}", Sha256::digest(content)),
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Some(RuntimePreflightSnapshot {
        importer_identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(importer_root),
        mods_identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(configured_mods_root),
        files,
    })
}

fn runtime_preflight_cache_key(
    importer_root: &Path,
    configured_mods_root: &Path,
    game_type: crate::modules::games::domain::models::GameType,
) -> String {
    format!(
        "{}|{}|{game_type:?}",
        crate::shared::path_key::canonical_path_key_for_path(importer_root),
        crate::shared::path_key::canonical_path_key_for_path(configured_mods_root),
    )
}

fn read_runtime_preflight(
    importer_root: &Path,
    configured_mods_root: &Path,
    game_type: crate::modules::games::domain::models::GameType,
) -> Result<RuntimePreflight, AppError> {
    let cache_key = runtime_preflight_cache_key(importer_root, configured_mods_root, game_type);
    let snapshot = runtime_preflight_snapshot(importer_root, configured_mods_root);
    if let Some(snapshot) = snapshot.as_ref() {
        let mut cache = RUNTIME_PREFLIGHT_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("runtime preflight cache lock poisoned");
        if let Some(cached) = cache.get_mut(&cache_key) {
            if &cached.snapshot == snapshot {
                cached.last_used = RUNTIME_PREFLIGHT_CACHE_CLOCK.fetch_add(1, Ordering::Relaxed);
                return Ok(cached.preflight.clone());
            }
        }
    }

    let preflight =
        read_runtime_preflight_uncached(importer_root, configured_mods_root, game_type)?;
    if let Some(snapshot) = snapshot {
        let verified_snapshot = runtime_preflight_snapshot(importer_root, configured_mods_root);
        if verified_snapshot.as_ref() == Some(&snapshot) {
            let mut cache = RUNTIME_PREFLIGHT_CACHE
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .expect("runtime preflight cache lock poisoned");
            if !cache.contains_key(&cache_key) && cache.len() >= MAX_CACHED_RUNTIME_PREFLIGHTS {
                if let Some(oldest_key) = cache
                    .iter()
                    .min_by_key(|(_, cached)| cached.last_used)
                    .map(|(key, _)| key.clone())
                {
                    cache.remove(&oldest_key);
                }
            }
            cache.insert(
                cache_key,
                CachedRuntimePreflight {
                    snapshot,
                    preflight: preflight.clone(),
                    last_used: RUNTIME_PREFLIGHT_CACHE_CLOCK.fetch_add(1, Ordering::Relaxed),
                },
            );
        }
    }
    Ok(preflight)
}

fn read_runtime_preflight_uncached(
    importer_root: &Path,
    configured_mods_root: &Path,
    game_type: crate::modules::games::domain::models::GameType,
) -> Result<RuntimePreflight, AppError> {
    let expected_namespace = match game_type {
        crate::modules::games::domain::models::GameType::GIMI => "GIMIv8",
        crate::modules::games::domain::models::GameType::SRMI => "SRMIv1",
        crate::modules::games::domain::models::GameType::ZZMI => "ZZMIv1",
        crate::modules::games::domain::models::GameType::WWMI => "WWMIv1",
        crate::modules::games::domain::models::GameType::EFMI => {
            return Err(AppError::Validation(
                "RuntimeCapabilitiesUnsupported: EFMI has no KeyViewer renderer profile"
                    .to_string(),
            ));
        }
    };

    let importer_root = canonical_or_original(importer_root);
    let configured_mods_root = canonical_or_original(configured_mods_root);
    if !path_is_within(&configured_mods_root, &importer_root) {
        return Err(AppError::Validation(format!(
            "RuntimeModsRootOutsideImporter: {} is not inside {}",
            configured_mods_root.display(),
            importer_root.display()
        )));
    }

    let config_path = importer_root.join("d3dx.ini");
    if !config_path.is_file() {
        return Ok(RuntimePreflight {
            runtime_mods_root: configured_mods_root.clone(),
            runtime_include_roots: vec![configured_mods_root],
            text_namespace: expected_namespace.to_string(),
            renderer_available: false,
            callback_slots: HashSet::new(),
            diagnostics: vec![format!(
                "RuntimeCapabilitiesUnverified: {} was not found",
                config_path.display()
            )],
        });
    }

    let config = std::fs::read_to_string(&config_path)?;
    let roots = parse_include_recursive_roots(&config);
    if roots.is_empty() {
        return Err(AppError::Validation(format!(
            "RuntimeModsRootMissing: {} has no [Include] include_recursive entry",
            config_path.display()
        )));
    }

    let mut resolved_roots = Vec::new();
    for root in roots {
        let relative = Path::new(&root);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(AppError::Validation(format!(
                "RuntimeModsRootInvalid: include_recursive={root} is not a safe importer-relative path"
            )));
        }
        let resolved = importer_root.join(relative);
        if !resolved.is_dir() || !path_is_within(&resolved, &importer_root) {
            return Err(AppError::Validation(format!(
                "RuntimeModsRootInvalid: include_recursive={root} does not resolve inside {}",
                importer_root.display()
            )));
        }
        resolved_roots.push(canonical_or_original(&resolved));
    }
    resolved_roots.sort();
    resolved_roots.dedup();
    if !resolved_roots
        .iter()
        .any(|root| root == &configured_mods_root)
    {
        return Err(AppError::Validation(format!(
            "RuntimeModsRootMismatch: configured {} is not an effective include_recursive root in {}",
            configured_mods_root.display(),
            config_path.display()
        )));
    }

    let mut renderer_available = false;
    let mut callback_slots = HashSet::new();
    let mut inspected_files = 0usize;
    for entry in WalkDir::new(&importer_root)
        .follow_links(false)
        .max_depth(5)
        .into_iter()
    {
        let entry = entry.map_err(|error| {
            AppError::Io(format!(
                "Could not inspect importer configuration under {}: {error}",
                importer_root.display()
            ))
        })?;
        if !entry.file_type().is_file()
            || path_is_within(entry.path(), &configured_mods_root)
            || !entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ini"))
        {
            continue;
        }
        inspected_files += 1;
        if inspected_files > 1_000 {
            return Err(AppError::Validation(
                "RuntimeCapabilitiesInvalid: importer configuration exceeds the INI scan limit"
                    .to_string(),
            ));
        }
        let content = std::fs::read_to_string(entry.path()).map_err(|error| {
            AppError::Io(format!(
                "Could not read importer configuration {}: {error}",
                entry.path().display()
            ))
        })?;
        let lower = content.to_ascii_lowercase();
        if has_ini_assignment(&content, "namespace", expected_namespace)
            && lower.contains("resourcetext")
            && lower.contains("resourcetextparams")
            && lower.contains("commandlistprinttext")
        {
            renderer_available = true;
        }
        for line in lower.lines() {
            let line = strip_ini_comment(line);
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim().eq_ignore_ascii_case("checktextureoverride") {
                let slot = value.trim().to_ascii_lowercase();
                if !slot.is_empty() {
                    callback_slots.insert(slot);
                }
            }
        }
    }

    let mut diagnostics = Vec::new();
    if !renderer_available {
        diagnostics.push(format!(
            "RendererUnsupported: {expected_namespace} ResourceText/ResourceTextParams/CommandListPrintText were not found"
        ));
    }
    if callback_slots.is_empty() {
        diagnostics.push("CallbackUnsupported: no checktextureoverride slot is active".to_string());
    }
    Ok(RuntimePreflight {
        runtime_mods_root: configured_mods_root,
        runtime_include_roots: resolved_roots,
        text_namespace: expected_namespace.to_string(),
        renderer_available,
        callback_slots,
        diagnostics,
    })
}

fn target_callback_slot(target: &RuntimeTarget) -> Option<String> {
    match target.resource_kind {
        RuntimeResourceKind::PositionVb => Some("vb0".to_string()),
        RuntimeResourceKind::IndexBuffer => Some("ib".to_string()),
        RuntimeResourceKind::DrawVb
        | RuntimeResourceKind::VertexBuffer
        | RuntimeResourceKind::Texture => {
            target.slot.as_ref().map(|slot| slot.to_ascii_lowercase())
        }
        RuntimeResourceKind::Shader => None,
    }
}

fn target_is_supported_by_runtime(target: &RuntimeTarget, preflight: &RuntimePreflight) -> bool {
    target.is_resource_target()
        && target_callback_slot(target).is_some_and(|slot| preflight.callback_slots.contains(&slot))
}

/// Context for post-mutation tasks.
#[derive(Clone)]
pub struct PostApplyContext {
    pub pool: SqlitePool,
    pub game_id: String,
    pub mods_path: PathBuf,
    /// Only the hotkey bindings, not the whole settings blob: post-apply reads
    /// `hotkeys` and nothing else, and this context is cloned on every
    /// mutation — a full `AppSettings` dragged every game, keyword and binding
    /// along with it.
    pub hotkeys: HotkeyConfig,
    /// Whether EMMM should publish its one owned KeyViewer entrypoint.
    pub keyviewer_enabled: bool,
    /// Per-game Safe Mode state rendered by the unified overlay banner.
    pub safe_mode: bool,
    /// Optional status overrides (e.g. preset name, folder name) from the mutation source.
    pub status_fields: Option<generator::StatusFields>,
}

#[derive(Debug)]
struct EnabledRuntimeMod {
    id: String,
    folder_path: ModFolderPath,
}

fn stored_mod_path_is_enabled(folder_path: &str) -> bool {
    !folder_path
        .split(['/', '\\'])
        .filter(|component| !component.is_empty())
        .any(crate::modules::workspace::domain::normalizer::is_disabled_folder)
}

async fn load_enabled_runtime_mods(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<EnabledRuntimeMod>, AppError> {
    let rows = sqlx::query(
        "SELECT id, folder_path FROM mods WHERE game_id = ? AND status = 1 ORDER BY id",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let id = row.get::<String, _>("id");
            let folder_path = row.get::<String, _>("folder_path");
            stored_mod_path_is_enabled(&folder_path).then_some(EnabledRuntimeMod {
                id,
                folder_path: ModFolderPath::from_stored(folder_path),
            })
        })
        .collect())
}

fn path_key_is_at_or_below(path_key: &str, root_key: &str) -> bool {
    path_key == root_key
        || path_key
            .strip_prefix(root_key)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn scoped_root_keys(mods_root: &Path, roots: &[ModFolderPath]) -> Option<Vec<String>> {
    if roots.len() > MAX_SCOPED_ROOTS {
        return None;
    }
    let mods_root = mods_root.to_string_lossy();
    let mut keys = Vec::with_capacity(roots.len());
    for root in roots {
        let stored = root.as_stored().trim();
        let path = Path::new(stored);
        if stored.is_empty()
            || path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return None;
        }
        keys.push(crate::shared::path_key::folder_path_key(
            stored,
            Some(&mods_root),
        ));
    }
    keys.sort();
    keys.dedup();
    let mut collapsed: Vec<String> = Vec::with_capacity(keys.len());
    for key in keys {
        if collapsed
            .iter()
            .any(|root| path_key_is_at_or_below(&key, root))
        {
            continue;
        }
        collapsed.push(key);
    }
    Some(collapsed)
}

async fn load_enabled_runtime_mods_for_roots(
    pool: &SqlitePool,
    game_id: &str,
    root_keys: &[String],
) -> Result<Vec<EnabledRuntimeMod>, AppError> {
    if root_keys.is_empty() {
        return Ok(Vec::new());
    }
    let mut query =
        QueryBuilder::<Sqlite>::new("SELECT id, folder_path FROM mods WHERE game_id = ");
    query.push_bind(game_id);
    query.push(" AND status = 1 AND (");
    for (position, root_key) in root_keys.iter().enumerate() {
        if position > 0 {
            query.push(" OR ");
        }
        let descendant_start = format!("{root_key}/");
        let descendant_end = format!("{root_key}0");
        query
            .push("(folder_path_key = ")
            .push_bind(root_key)
            .push(" OR (folder_path_key >= ")
            .push_bind(descendant_start)
            .push(" AND folder_path_key < ")
            .push_bind(descendant_end)
            .push("))");
    }
    query.push(") ORDER BY id");
    let rows = query.build().fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let id = row.get::<String, _>("id");
            let folder_path = row.get::<String, _>("folder_path");
            stored_mod_path_is_enabled(&folder_path).then_some(EnabledRuntimeMod {
                id,
                folder_path: ModFolderPath::from_stored(folder_path),
            })
        })
        .collect())
}

fn composition_cache_identity(
    mods_root: &Path,
    capabilities: &harvester::HarvestCapabilities,
) -> (String, String) {
    (
        crate::shared::path_key::canonical_path_key_for_path(mods_root),
        capabilities.cache_key(),
    )
}

fn take_cached_mod_composition(
    game_id: &str,
    mods_root: &Path,
    capabilities: &harvester::HarvestCapabilities,
) -> Result<Option<GameCompositionCache>, AppError> {
    let (mods_root, capability_slots) = composition_cache_identity(mods_root, capabilities);
    let caches = KEYVIEWER_COMPOSITION_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut caches = caches.lock().map_err(|_| {
        AppError::Internal(
            "KeyViewer composition cache was poisoned; restart the application".to_string(),
        )
    })?;
    let Some(cached) = caches.remove(game_id) else {
        return Ok(None);
    };
    if cached.cache.mods_root != mods_root || cached.cache.capability_slots != capability_slots {
        let stale_mods_root = cached.cache.mods_root.clone();
        drop(caches);
        harvester::evict_cached_root_by_key(&stale_mods_root);
        return Ok(None);
    }

    // The per-game runtime worker is single-flight, so after removing the map
    // entry this is normally the sole Arc owner. `try_unwrap` makes a one-mod
    // update O(log n) instead of cloning a 100k-entry composition map. The
    // clone is only a defensive fallback for transient diagnostic readers.
    Ok(Some(
        Arc::try_unwrap(cached.cache).unwrap_or_else(|cache| (*cache).clone()),
    ))
}

fn store_mod_composition(game_id: &str, prepared: &PreparedModComposition) -> Result<(), AppError> {
    let caches = KEYVIEWER_COMPOSITION_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut caches = caches.lock().map_err(|_| {
        AppError::Internal(
            "KeyViewer composition cache was poisoned; restart the application".to_string(),
        )
    })?;
    let mut evicted_mods_root = None;
    if !cfg!(test) && !caches.contains_key(game_id) && caches.len() >= MAX_CACHED_GAMES {
        if let Some(evicted_game) = caches
            .iter()
            .filter(|(cached_game_id, _)| cached_game_id.as_str() != game_id)
            .min_by_key(|(_, cached)| cached.last_used)
            .map(|(cached_game_id, _)| cached_game_id.clone())
        {
            evicted_mods_root = caches
                .remove(&evicted_game)
                .map(|cached| cached.cache.mods_root.clone());
        }
    }
    caches.insert(
        game_id.to_string(),
        CachedGameComposition {
            cache: Arc::clone(&prepared.cache),
            last_used: KEYVIEWER_COMPOSITION_CACHE_CLOCK.fetch_add(1, Ordering::Relaxed),
        },
    );
    let estimated_retained_composition_entries = caches
        .values()
        .map(|cached| cached.cache.mods.len())
        .sum::<usize>();
    drop(caches);
    if let Some(mods_root) = evicted_mods_root {
        harvester::evict_cached_root_by_key(&mods_root);
    }
    let harvest_metrics = harvester::harvest_cache_metrics();
    log::info!(
        "[post_apply] KeyViewer cache retention game={game_id} estimated_composition_entries={estimated_retained_composition_entries} estimated_harvest_entries={} cache_hits_total={} cache_misses_total={} harvested_count_total={}",
        harvest_metrics.retained_entries,
        harvest_metrics.cache_hits,
        harvest_metrics.cache_misses,
        harvest_metrics.harvested_mods,
    );
    Ok(())
}

fn remove_mod_composition(game_id: &str) -> Result<(), AppError> {
    let removed = KEYVIEWER_COMPOSITION_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| {
            AppError::Internal(
                "KeyViewer composition cache was poisoned; restart the application".to_string(),
            )
        })?
        .remove(game_id);
    if let Some(cached) = removed {
        harvester::evict_cached_root_by_key(&cached.cache.mods_root);
    }
    Ok(())
}

async fn harvest_runtime_mod(
    mods_root: &Path,
    folder_path: &ModFolderPath,
    capabilities: &harvester::HarvestCapabilities,
) -> Result<Arc<harvester::ModHarvest>, AppError> {
    let absolute_path = folder_path.resolve(mods_root);
    let capabilities = capabilities.clone();
    tokio::task::spawn_blocking(move || harvester::harvest_mod(&absolute_path, &capabilities))
        .await?
}

async fn prepare_full_mod_composition(
    pool: &SqlitePool,
    game_id: &str,
    mods_root: &Path,
    capabilities: &harvester::HarvestCapabilities,
    mode: CompositionMode,
    is_current: CurrentGenerationCheck<'_>,
) -> Result<Option<PreparedModComposition>, AppError> {
    let enabled_mods = load_enabled_runtime_mods(pool, game_id).await?;
    if !composition_checkpoint(is_current, game_id, "full_load", 0)? {
        return Ok(None);
    }
    let mut active_mod_paths = Vec::with_capacity(enabled_mods.len());
    for (index, entry) in enabled_mods.iter().enumerate() {
        if composition_checkpoint_due(index)
            && !composition_checkpoint(is_current, game_id, "full_active_paths", index)?
        {
            return Ok(None);
        }
        active_mod_paths.push(entry.folder_path.resolve(mods_root));
    }
    harvester::retain_cached_mods(
        mods_root,
        capabilities,
        active_mod_paths.iter().map(PathBuf::as_path),
    );
    if !composition_checkpoint(is_current, game_id, "full_cache_retention", 0)? {
        return Ok(None);
    }

    let (mods_root_key, capability_slots) = composition_cache_identity(mods_root, capabilities);
    let mut mods = BTreeMap::new();
    for (index, entry) in enabled_mods.into_iter().enumerate() {
        if !composition_checkpoint(is_current, game_id, "full_harvest", index)? {
            return Ok(None);
        }
        let harvest = harvest_runtime_mod(mods_root, &entry.folder_path, capabilities).await?;
        mods.insert(
            entry.id,
            CachedModContribution {
                folder_path: entry.folder_path,
                harvest,
            },
        );
    }
    let harvested_mods = mods.len();
    if !composition_checkpoint(is_current, game_id, "full_complete", harvested_mods)? {
        return Ok(None);
    }
    Ok(Some(PreparedModComposition {
        cache: Arc::new(GameCompositionCache {
            mods_root: mods_root_key,
            capability_slots,
            mods,
        }),
        mode,
        harvested_mods,
    }))
}

fn validate_scoped_changes(changes: &[RuntimeModChange]) -> Result<(), AppError> {
    if let Some(change) = changes.iter().find(|change| {
        change.mod_id.trim().is_empty() || change.folder_path.as_stored().trim().is_empty()
    }) {
        return Err(AppError::Validation(format!(
            "KeyViewer scoped sync contains an invalid mod identity: {:?}",
            change.mod_id
        )));
    }
    Ok(())
}

async fn prepare_scoped_mod_composition(
    game_id: &str,
    mods_root: &Path,
    capabilities: &harvester::HarvestCapabilities,
    changes: Vec<RuntimeModChange>,
    is_current: CurrentGenerationCheck<'_>,
) -> Result<Option<PreparedModComposition>, AppError> {
    validate_scoped_changes(&changes)?;
    if !composition_checkpoint(is_current, game_id, "scoped_start", 0)? {
        return Ok(None);
    }
    let Some(cache) = take_cached_mod_composition(game_id, mods_root, capabilities)? else {
        return Ok(None);
    };
    let mut cache_lease = CompositionCacheLease::new(game_id, cache);
    let cache = cache_lease.cache_mut();
    let mut latest_changes = BTreeMap::new();
    for (index, change) in changes.into_iter().enumerate() {
        if composition_checkpoint_due(index)
            && !composition_checkpoint(is_current, game_id, "scoped_deduplicate", index)?
        {
            return Ok(Some(cache_lease.into_prepared(CompositionMode::Scoped, 0)));
        }
        latest_changes.insert(change.mod_id.clone(), change);
    }
    let changed_mods = latest_changes.len();
    let mut harvested_mods = 0;
    for (index, (mod_id, change)) in latest_changes.into_iter().enumerate() {
        if !composition_checkpoint(is_current, game_id, "scoped_harvest", index)? {
            return Ok(Some(
                cache_lease.into_prepared(CompositionMode::Scoped, harvested_mods),
            ));
        }
        if let Some(previous) = cache.mods.remove(&mod_id) {
            harvester::invalidate_cached_mod(&previous.folder_path.resolve(mods_root));
        }
        let absolute_path = change.folder_path.resolve(mods_root);
        harvester::invalidate_cached_mod(&absolute_path);
        if change.outcome == RuntimeModOutcome::Enabled {
            let harvest = harvest_runtime_mod(mods_root, &change.folder_path, capabilities).await?;
            cache.mods.insert(
                mod_id,
                CachedModContribution {
                    folder_path: change.folder_path,
                    harvest,
                },
            );
            harvested_mods += 1;
        }
    }
    if !composition_checkpoint(is_current, game_id, "scoped_complete", harvested_mods)? {
        return Ok(Some(
            cache_lease.into_prepared(CompositionMode::Scoped, harvested_mods),
        ));
    }
    log::info!(
        "[post_apply] KeyViewer scoped composition game={game_id} changed_mods={changed_mods} harvested_mods={harvested_mods} reused_mods={}",
        cache.mods.len().saturating_sub(harvested_mods)
    );
    Ok(Some(
        cache_lease.into_prepared(CompositionMode::Scoped, harvested_mods),
    ))
}

async fn prepare_scoped_roots_composition(
    pool: &SqlitePool,
    game_id: &str,
    mods_root: &Path,
    capabilities: &harvester::HarvestCapabilities,
    roots: Vec<ModFolderPath>,
    is_current: CurrentGenerationCheck<'_>,
) -> Result<Option<PreparedModComposition>, AppError> {
    let Some(root_keys) = scoped_root_keys(mods_root, &roots) else {
        return Ok(None);
    };
    if !composition_checkpoint(is_current, game_id, "scoped_roots_start", 0)? {
        return Ok(None);
    }
    let Some(cache) = take_cached_mod_composition(game_id, mods_root, capabilities)? else {
        return Ok(None);
    };
    let mut cache_lease = CompositionCacheLease::new(game_id, cache);
    let cache = cache_lease.cache_mut();

    let mods_root_display = mods_root.to_string_lossy();
    let mut affected_ids = Vec::new();
    let mut scan_superseded = false;
    for (index, (mod_id, contribution)) in cache.mods.iter().enumerate() {
        if composition_checkpoint_due(index)
            && !composition_checkpoint(is_current, game_id, "scoped_roots_scan", index)?
        {
            scan_superseded = true;
            break;
        }
        let path_key = crate::shared::path_key::folder_path_key(
            contribution.folder_path.as_stored(),
            Some(&mods_root_display),
        );
        if root_keys
            .iter()
            .any(|root| path_key_is_at_or_below(&path_key, root))
        {
            affected_ids.push(mod_id.clone());
        }
    }
    if scan_superseded {
        return Ok(Some(
            cache_lease.into_prepared(CompositionMode::ScopedRoots, 0),
        ));
    }
    for (index, mod_id) in affected_ids.into_iter().enumerate() {
        if !composition_checkpoint(is_current, game_id, "scoped_roots_remove", index)? {
            return Ok(Some(
                cache_lease.into_prepared(CompositionMode::ScopedRoots, 0),
            ));
        }
        if let Some(previous) = cache.mods.remove(&mod_id) {
            harvester::invalidate_cached_mod(&previous.folder_path.resolve(mods_root));
        }
    }

    let enabled_mods = load_enabled_runtime_mods_for_roots(pool, game_id, &root_keys).await?;
    let harvested_mods = enabled_mods.len();
    if !composition_checkpoint(is_current, game_id, "scoped_roots_load", 0)? {
        return Ok(Some(
            cache_lease.into_prepared(CompositionMode::ScopedRoots, 0),
        ));
    }
    for (index, entry) in enabled_mods.into_iter().enumerate() {
        if !composition_checkpoint(is_current, game_id, "scoped_roots_harvest", index)? {
            return Ok(Some(
                cache_lease.into_prepared(CompositionMode::ScopedRoots, index),
            ));
        }
        if let Some(previous) = cache.mods.remove(&entry.id) {
            harvester::invalidate_cached_mod(&previous.folder_path.resolve(mods_root));
        }
        harvester::invalidate_cached_mod(&entry.folder_path.resolve(mods_root));
        let harvest = harvest_runtime_mod(mods_root, &entry.folder_path, capabilities).await?;
        cache.mods.insert(
            entry.id,
            CachedModContribution {
                folder_path: entry.folder_path,
                harvest,
            },
        );
    }
    if !composition_checkpoint(is_current, game_id, "scoped_roots_complete", harvested_mods)? {
        return Ok(Some(
            cache_lease.into_prepared(CompositionMode::ScopedRoots, harvested_mods),
        ));
    }
    log::info!(
        "[post_apply] KeyViewer root-scoped composition game={game_id} roots={} harvested_mods={harvested_mods} reused_mods={}",
        root_keys.len(),
        cache.mods.len().saturating_sub(harvested_mods)
    );
    Ok(Some(cache_lease.into_prepared(
        CompositionMode::ScopedRoots,
        harvested_mods,
    )))
}

async fn prepare_mod_composition(
    pool: &SqlitePool,
    game_id: &str,
    mods_root: &Path,
    capabilities: &harvester::HarvestCapabilities,
    request: RuntimeSyncRequest,
    is_current: CurrentGenerationCheck<'_>,
) -> Result<Option<PreparedModComposition>, AppError> {
    match request {
        RuntimeSyncRequest::Full => {
            prepare_full_mod_composition(
                pool,
                game_id,
                mods_root,
                capabilities,
                CompositionMode::Full,
                is_current,
            )
            .await
        }
        RuntimeSyncRequest::Scoped { changes } => {
            if let Some(prepared) = prepare_scoped_mod_composition(
                game_id,
                mods_root,
                capabilities,
                changes,
                is_current,
            )
            .await?
            {
                Ok(Some(prepared))
            } else if !composition_checkpoint(is_current, game_id, "scoped_fallback", 0)? {
                Ok(None)
            } else {
                log::info!(
                    "[post_apply] KeyViewer scoped composition cache miss game={game_id}; rebuilding full state"
                );
                prepare_full_mod_composition(
                    pool,
                    game_id,
                    mods_root,
                    capabilities,
                    CompositionMode::FullFallback,
                    is_current,
                )
                .await
            }
        }
        RuntimeSyncRequest::ScopedRoots { roots } => {
            if let Some(prepared) = prepare_scoped_roots_composition(
                pool,
                game_id,
                mods_root,
                capabilities,
                roots,
                is_current,
            )
            .await?
            {
                Ok(Some(prepared))
            } else if !composition_checkpoint(is_current, game_id, "scoped_roots_fallback", 0)? {
                Ok(None)
            } else {
                log::info!(
                    "[post_apply] KeyViewer root-scoped composition cache miss or ambiguous scope game={game_id}; rebuilding full state"
                );
                prepare_full_mod_composition(
                    pool,
                    game_id,
                    mods_root,
                    capabilities,
                    CompositionMode::FullFallback,
                    is_current,
                )
                .await
            }
        }
    }
}

async fn catalog_app_data_dir(pool: &SqlitePool) -> Result<PathBuf, AppError> {
    let rows = sqlx::query("PRAGMA database_list").fetch_all(pool).await?;
    let file = rows
        .iter()
        .find(|row| row.try_get::<String, _>("name").ok().as_deref() == Some("main"))
        .and_then(|row| row.try_get::<String, _>("file").ok())
        .filter(|file| !file.trim().is_empty())
        .ok_or_else(|| {
            AppError::Internal("Could not resolve the app data directory".to_string())
        })?;
    PathBuf::from(file)
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| AppError::Internal("App database has no parent directory".to_string()))
}

fn kv_entries_from_catalog(
    entries: Vec<crate::modules::workspace::application::scanner::master_db::CatalogKeyviewerEntry>,
    preflight: &RuntimePreflight,
) -> Vec<matcher::KvObjectEntry> {
    entries
        .into_iter()
        .filter(|entry| entry.object_type.eq_ignore_ascii_case("character"))
        .filter_map(|entry| {
            let mut skin_hashes: HashMap<String, Vec<String>> = HashMap::new();
            let runtime_targets: Vec<RuntimeTarget> = entry
                .runtime_targets
                .into_iter()
                .filter_map(|mut target| {
                    let callback_slot = target_callback_slot(&target)?;
                    if target_is_supported_by_runtime(&target, preflight) {
                        target.hash = target.hash.to_ascii_lowercase();
                        target.slot = Some(callback_slot);
                        Some(target)
                    } else {
                        None
                    }
                })
                .collect();
            for target in &runtime_targets {
                skin_hashes
                    .entry(target.variant.clone())
                    .or_default()
                    .push(target.hash.to_ascii_lowercase());
            }
            for hashes in skin_hashes.values_mut() {
                hashes.sort_unstable();
                hashes.dedup();
            }
            let code_hashes: Vec<String> = skin_hashes
                .values()
                .flatten()
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            (!code_hashes.is_empty()).then_some(matcher::KvObjectEntry {
                name: entry.name,
                object_type: entry.object_type,
                code_hashes,
                skin_hashes,
                runtime_targets,
                tags: entry.aliases,
                thumbnail_path: None,
            })
        })
        .collect()
}

fn section_mentions_object(section_name: &str, object_name: &str) -> bool {
    let normalized_section: String = section_name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    let normalized_object: String = object_name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    !normalized_object.is_empty() && normalized_section.contains(&normalized_object)
}

fn cleanup_staging_after_error(staging: &std::path::Path, error: AppError) -> AppError {
    match std::fs::remove_dir_all(staging) {
        Ok(()) => error,
        Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup_error) => AppError::Io(format!(
            "KeyViewer artifact error ({error}); staging cleanup also failed ({cleanup_error}): {}",
            staging.display()
        )),
    }
}

fn sync_lock_for_game(game_id: &str) -> Result<Arc<tokio::sync::Mutex<()>>, AppError> {
    let locks = KEYVIEWER_SYNC_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = locks.lock().map_err(|_| {
        AppError::Internal("KeyViewer sync lock was poisoned; restart the application".to_string())
    })?;
    Ok(locks
        .entry(game_id.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone())
}

fn request_sync_revision_with_request(
    game_id: &str,
    request: RuntimeSyncRequest,
) -> Result<u64, AppError> {
    if let RuntimeSyncRequest::Scoped { changes } = &request {
        validate_scoped_changes(changes)?;
    }
    let revisions = KEYVIEWER_SYNC_REVISIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut revisions = revisions.lock().map_err(|_| {
        AppError::Internal(
            "KeyViewer sync revision state was poisoned; restart the application".to_string(),
        )
    })?;
    let state = revisions
        .entry(game_id.to_string())
        .or_insert_with(|| RuntimeSyncRevisionState {
            revision: 0,
            pending: None,
        });
    state.revision = state.revision.saturating_add(1);
    state.pending = Some(match state.pending.take() {
        Some(pending) => pending.merge_pending(request),
        None => request,
    });
    Ok(state.revision)
}

fn request_sync_revision(game_id: &str) -> Result<u64, AppError> {
    request_sync_revision_with_request(game_id, RuntimeSyncRequest::Full)
}

/// Reserve an explicit full-recovery generation for backward-compatible
/// callers. Full requests dominate any unprocessed incremental request.
pub fn reserve_overlay_sync_revision(game_id: &str) -> Result<u64, AppError> {
    request_sync_revision(game_id)
}

/// Reserve a generation carrying its incremental mod scope. Queue adapters can
/// store only the returned revision; the request remains process-local and is
/// merged with any older scope superseded by the latest-wins queue.
pub fn reserve_overlay_sync_revision_for_request(
    game_id: &str,
    request: RuntimeSyncRequest,
) -> Result<u64, AppError> {
    request_sync_revision_with_request(game_id, request)
}

fn is_current_sync_revision(game_id: &str, revision: u64) -> Result<bool, AppError> {
    let revisions = KEYVIEWER_SYNC_REVISIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let revisions = revisions.lock().map_err(|_| {
        AppError::Internal(
            "KeyViewer sync revision state was poisoned; restart the application".to_string(),
        )
    })?;
    Ok(revisions
        .get(game_id)
        .is_some_and(|current| current.revision == revision))
}

fn sync_request_for_revision(
    game_id: &str,
    revision: u64,
) -> Result<Option<RuntimeSyncRequest>, AppError> {
    let revisions = KEYVIEWER_SYNC_REVISIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let revisions = revisions.lock().map_err(|_| {
        AppError::Internal(
            "KeyViewer sync revision state was poisoned; restart the application".to_string(),
        )
    })?;
    Ok(revisions.get(game_id).and_then(|state| {
        (state.revision == revision)
            .then(|| state.pending.clone())
            .flatten()
    }))
}

fn settle_sync_request(game_id: &str, revision: u64) -> Result<(), AppError> {
    let revisions = KEYVIEWER_SYNC_REVISIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut revisions = revisions.lock().map_err(|_| {
        AppError::Internal(
            "KeyViewer sync revision state was poisoned; restart the application".to_string(),
        )
    })?;
    if let Some(state) = revisions.get_mut(game_id) {
        if state.revision == revision {
            state.pending = None;
        }
    }
    Ok(())
}

fn runtime_sync_request_is_current(
    game_id: &str,
    expected_sync_revision: Option<u64>,
    activation_authority: Option<&ActivationAuthority>,
) -> Result<bool, AppError> {
    if let Some(revision) = expected_sync_revision {
        if !is_current_sync_revision(game_id, revision)? {
            return Ok(false);
        }
    }
    Ok(activation_authority.is_none_or(|authority| authority.with_current(|| ()).is_some()))
}

fn commit_if_current_sync_revision<T>(
    game_id: &str,
    revision: u64,
    activation_authority: Option<&ActivationAuthority>,
    commit: impl FnOnce() -> Result<T, AppError>,
) -> Result<Option<T>, AppError> {
    let revisions = KEYVIEWER_SYNC_REVISIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let revisions = revisions.lock().map_err(|_| {
        AppError::Internal(
            "KeyViewer sync revision state was poisoned; restart the application".to_string(),
        )
    })?;
    if revisions
        .get(game_id)
        .is_none_or(|current| current.revision != revision)
    {
        return Ok(None);
    }
    match activation_authority {
        Some(authority) => authority.with_current(commit).transpose(),
        None => commit().map(Some),
    }
}

fn superseded_sync_result(cause: OverlaySyncCause) -> RuntimeSyncResult {
    RuntimeSyncResult {
        cause,
        publication: RuntimeSyncPublication::Skipped,
        reload: RuntimeReloadOutcome::NotRequired,
        failure: None,
    }
}

fn publication_status(result: &RuntimeSyncResult) -> RuntimeSyncPublicationStatus {
    match result.publication {
        RuntimeSyncPublication::Published => RuntimeSyncPublicationStatus::Published,
        RuntimeSyncPublication::Unchanged => RuntimeSyncPublicationStatus::Unchanged,
        RuntimeSyncPublication::Skipped => RuntimeSyncPublicationStatus::Skipped,
        RuntimeSyncPublication::FailedBeforePublish => RuntimeSyncPublicationStatus::Failed,
    }
}

fn reload_status_and_binding(result: &RuntimeSyncResult) -> (RuntimeReloadStatus, Option<String>) {
    match &result.reload {
        RuntimeReloadOutcome::NotRequired => (RuntimeReloadStatus::NotRequired, None),
        RuntimeReloadOutcome::ReloadSent { binding } => {
            (RuntimeReloadStatus::Sent, Some(binding.clone()))
        }
        RuntimeReloadOutcome::NeedsManualReload { binding, .. } => {
            (RuntimeReloadStatus::Manual, binding.clone())
        }
    }
}

fn record_runtime_sync_result(game_id: &str, result: &RuntimeSyncResult) {
    let snapshots = KEYVIEWER_RUNTIME_SYNC_SNAPSHOTS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut snapshots) = snapshots.lock() else {
        log::warn!("KeyViewer runtime diagnostics could not record a sync result");
        return;
    };
    let (reload, reload_binding) = reload_status_and_binding(result);
    snapshots.insert(
        game_id.to_string(),
        RuntimeSyncSnapshot {
            last_sync_unix_ms: chrono::Utc::now().timestamp_millis(),
            publication: publication_status(result),
            reload,
            reload_binding,
        },
    );
}

fn record_runtime_sync_result_if_current(
    game_id: &str,
    result: &RuntimeSyncResult,
    expected_sync_revision: Option<u64>,
    activation_authority: Option<&ActivationAuthority>,
) -> Result<bool, AppError> {
    match expected_sync_revision {
        Some(revision) => {
            Ok(
                commit_if_current_sync_revision(game_id, revision, activation_authority, || {
                    record_runtime_sync_result(game_id, result);
                    Ok(())
                })?
                .is_some(),
            )
        }
        None => {
            record_runtime_sync_result(game_id, result);
            Ok(true)
        }
    }
}

pub fn keyviewer_runtime_diagnostics(
    settings: &crate::modules::settings::application::config::AppSettings,
    game_id: &str,
) -> Result<KeyViewerRuntimeDiagnostics, AppError> {
    let game = settings
        .games
        .iter()
        .find(|game| game.id == game_id)
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} was not found")))?;
    let snapshots = KEYVIEWER_RUNTIME_SYNC_SNAPSHOTS.get_or_init(|| Mutex::new(HashMap::new()));
    let snapshot = snapshots
        .lock()
        .map_err(|_| {
            AppError::Internal(
                "KeyViewer runtime diagnostics state was poisoned; restart the application"
                    .to_string(),
            )
        })?
        .get(game_id)
        .cloned();
    let discovered_reload_binding = (settings.active_game_id.as_deref() == Some(game_id))
        .then(|| {
            crate::modules::automation::application::hotkeys::reload::configured_reload_config_binding(
                settings,
            )
        })
        .and_then(Result::ok);

    Ok(KeyViewerRuntimeDiagnostics {
        last_sync_unix_ms: snapshot.as_ref().map(|entry| entry.last_sync_unix_ms),
        publication: snapshot.as_ref().map(|entry| entry.publication.clone()),
        reload: snapshot.as_ref().map(|entry| entry.reload.clone()),
        reload_binding: snapshot
            .and_then(|entry| entry.reload_binding)
            .or(discovered_reload_binding),
        cleanup_automatic_disabled: game.game_exe.is_none(),
    })
}

fn manifest_path(emmm_data_dir: &Path) -> PathBuf {
    emmm_data_dir.join(KEYVIEWER_MANIFEST_FILE)
}

fn read_manifest(emmm_data_dir: &Path) -> Option<KeyViewerManifest> {
    let content = std::fs::read_to_string(manifest_path(emmm_data_dir)).ok()?;
    serde_json::from_str(&content).ok()
}

fn entrypoint_uses_generation(content: &[u8], generation_id: &str) -> bool {
    let content = String::from_utf8_lossy(content);
    let prefix = format!("filename = {KEYVIEWER_RESOURCE_ROOT}/{generation_id}/");
    content.contains(&format!("{prefix}status/")) || content.contains(&format!("{prefix}keybinds/"))
}

fn published_artifact_matches(emmm_data_dir: &Path, entrypoint: &Path, fingerprint: &str) -> bool {
    let Some(manifest) = read_manifest(emmm_data_dir) else {
        return false;
    };
    if manifest.version != KEYVIEWER_MANIFEST_VERSION || manifest.fingerprint != fingerprint {
        return false;
    }
    let Ok(content) = std::fs::read(entrypoint) else {
        return false;
    };
    if !is_current_emmm_entrypoint(&content) {
        return false;
    }
    emmm_data_dir
        .join(KEYVIEWER_RESOURCE_ROOT)
        .join(&manifest.generation_id)
        .is_dir()
        && entrypoint_uses_generation(&content, &manifest.generation_id)
}

#[allow(clippy::too_many_arguments)] // These are independent immutable fingerprint inputs.
fn runtime_input_fingerprint(
    ctx: &PostApplyContext,
    importer_root: &Path,
    preflight: &RuntimePreflight,
    matches: &[matcher::MatchResult],
    sources_per_object: &HashMap<String, Vec<generator::SourceKeyBinding>>,
    status: &generator::StatusFields,
    ini_fingerprints: &[String],
    catalog_manifest_checksum: &str,
) -> String {
    let mut inputs = vec![
        format!("importer={}", normalized_windows_path(importer_root)),
        format!(
            "mods={}",
            normalized_windows_path(&preflight.runtime_mods_root)
        ),
        format!("namespace={}", preflight.text_namespace),
        format!("renderer={}", preflight.renderer_available),
        format!("layout={}", generator::KEYVIEWER_LAYOUT_REVISION),
        format!("safe={}", ctx.safe_mode),
        format!("keyviewer={}", ctx.keyviewer_enabled),
        format!("hotkeys={:?}", ctx.hotkeys),
        format!("status={status:?}"),
        format!("catalog_manifest={catalog_manifest_checksum}"),
    ];
    let mut include_roots = preflight
        .runtime_include_roots
        .iter()
        .map(|path| normalized_windows_path(path))
        .collect::<Vec<_>>();
    include_roots.sort_unstable();
    inputs.extend(
        include_roots
            .into_iter()
            .map(|path| format!("include={path}")),
    );

    let mut callback_slots = preflight.callback_slots.iter().cloned().collect::<Vec<_>>();
    callback_slots.sort_unstable();
    inputs.extend(
        callback_slots
            .into_iter()
            .map(|slot| format!("callback={slot}")),
    );

    let mut rendered_matches = matches
        .iter()
        .map(|entry| format!("match={entry:?}"))
        .collect::<Vec<_>>();
    rendered_matches.sort_unstable();
    inputs.extend(rendered_matches);

    let mut rendered_sources = sources_per_object
        .iter()
        .map(|(name, sources)| format!("sources={name}:{sources:?}"))
        .collect::<Vec<_>>();
    rendered_sources.sort_unstable();
    inputs.extend(rendered_sources);

    let mut effective_ini_fingerprints = ini_fingerprints.to_vec();
    effective_ini_fingerprints.sort_unstable();
    inputs.extend(
        effective_ini_fingerprints
            .into_iter()
            .map(|fingerprint| format!("ini={fingerprint}")),
    );

    let mut hasher = Sha256::new();
    for input in inputs {
        hasher.update(input.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

fn catalog_manifest_checksum(app_data_dir: &Path) -> String {
    let manifest = app_data_dir
        .join(
            crate::modules::workspace::application::scanner::master_db::asset_pack::PACK_DIRECTORY,
        )
        .join(
            crate::modules::workspace::application::scanner::master_db::asset_pack::MANIFEST_FILE,
        );
    match std::fs::read(&manifest) {
        Ok(content) => format!("{:x}", Sha256::digest(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "missing".to_string(),
        Err(error) => format!("unreadable:{:?}", error.kind()),
    }
}

/// Read only the persisted active preset label for the status banner. Building
/// the live collection projection here would scan the library during every
/// overlay sync, despite the banner not needing that information.
async fn active_preset_name(pool: &SqlitePool, game_id: &str) -> Result<Option<String>, AppError> {
    let active_id = crate::modules::collections::adapters::sqlite::runtime::get(pool, game_id)
        .await?
        .and_then(|state| state.active_collection_id);
    let Some(active_id) = active_id else {
        return Ok(None);
    };
    Ok(
        crate::modules::collections::adapters::sqlite::get_by_id(pool, &active_id)
            .await?
            .filter(|collection| collection.game_id == game_id && !collection.is_draft)
            .map(|collection| collection.name),
    )
}

fn game_is_confirmed_stopped(
    settings: &crate::modules::settings::application::config::AppSettings,
    game_id: &str,
) -> bool {
    let game_exe = settings
        .games
        .iter()
        .find(|game| game.id == game_id)
        .and_then(|game| game.game_exe.as_deref());
    game_exe.is_some()
        && !crate::modules::system::application::game_detector::is_game_running(game_exe)
}

fn cleanup_generation_siblings(emmm_data_dir: &Path) -> Result<(), AppError> {
    let entries = std::fs::read_dir(emmm_data_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            (name.starts_with("generations.staging.")
                || name.starts_with("generations.recover.")
                || matches!(name.as_str(), "keybinds" | "status"))
            .then_some(entry.path())
        })
        .collect::<Vec<_>>();
    for path in entries {
        std::fs::remove_dir_all(&path).map_err(|error| {
            AppError::Io(format!(
                "Could not remove stale KeyViewer artifact {}: {error}",
                path.display()
            ))
        })?;
    }
    cleanup_inactive_generation_children(emmm_data_dir)?;
    Ok(())
}

/// Remove managed generations except the one named by the durable manifest.
/// Callers only run this after reload or after confirming the game is stopped.
fn cleanup_inactive_generation_children(emmm_data_dir: &Path) -> Result<(), AppError> {
    let generations_dir = emmm_data_dir.join(KEYVIEWER_RESOURCE_ROOT);
    if !generations_dir.is_dir() {
        return Ok(());
    }
    let active_generation = read_manifest(emmm_data_dir)
        .filter(|manifest| manifest.version == KEYVIEWER_MANIFEST_VERSION)
        .map(|manifest| manifest.generation_id);
    let entries = std::fs::read_dir(&generations_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            active_generation.as_deref() != Some(name.as_str())
                && (is_legacy_generation_id(&name)
                    || is_content_generation_id(&name)
                    || matches!(name.as_str(), "status" | "keybinds")
                    || name.contains(".staging.")
                    || name.contains(".recover."))
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    for path in entries {
        std::fs::remove_dir_all(&path).map_err(|error| {
            AppError::Io(format!(
                "Could not remove legacy KeyViewer generation {}: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

fn is_content_generation_id(name: &str) -> bool {
    name.len() == 65
        && name.starts_with('g')
        && name[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

fn is_legacy_generation_id(name: &str) -> bool {
    let Some(value) = name.strip_prefix('g') else {
        return false;
    };
    let mut parts = value.split('-');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(milliseconds), Some(process_id), Some(sequence), None)
            if milliseconds.parse::<u128>().is_ok()
                && process_id.parse::<u32>().is_ok()
                && sequence.parse::<u64>().is_ok()
    )
}

/// Keep one published generation on disk. Cleanup is only called after the
/// game has stopped or the newly published snapshot was successfully reloaded;
/// a manual-reload result keeps the previous folder until it is safe to remove.
fn should_cleanup_generations(reload: &RuntimeReloadOutcome, game_stopped: bool) -> bool {
    game_stopped || matches!(reload, RuntimeReloadOutcome::ReloadSent { .. })
}

fn is_current_emmm_entrypoint(content: &[u8]) -> bool {
    let content = String::from_utf8_lossy(content).to_ascii_lowercase();
    content.contains("namespace = emmmv1") && content.contains("emmm-artifact: keyviewer v1")
}

fn is_known_legacy_entrypoint(content: &[u8]) -> bool {
    format!("{:x}", Sha256::digest(content)) == KNOWN_LEGACY_KEYVIEWER_SHA256
}

fn is_migratable_legacy_entrypoint(content: &[u8]) -> bool {
    let content = String::from_utf8_lossy(content).to_ascii_lowercase();
    content.contains("keyviewer.ini")
        && content.contains("auto-generated by emmm")
        && content.contains("[keyemmm_toggle]")
        && content.contains("[commandlist_emm_render]")
}

fn is_verified_emmm_entrypoint(content: &[u8]) -> bool {
    is_current_emmm_entrypoint(content)
        || is_known_legacy_entrypoint(content)
        || is_migratable_legacy_entrypoint(content)
}

fn legacy_entrypoint_backup_path(entrypoint: &Path) -> Result<PathBuf, AppError> {
    let name = entrypoint.file_name().ok_or_else(|| {
        AppError::Validation(format!(
            "Invalid KeyViewer entrypoint path: {}",
            entrypoint.display()
        ))
    })?;
    Ok(entrypoint.with_file_name(format!(
        "{}{}",
        name.to_string_lossy(),
        LEGACY_KEYVIEWER_BACKUP_SUFFIX
    )))
}

/// Preserve an EMMM-generated v0 artifact before replacing it with the
/// content-addressed entrypoint. Unknown entrypoints still fail closed.
fn preserve_legacy_entrypoint(entrypoint: &Path, expected_content: &str) -> Result<(), AppError> {
    let current = std::fs::read_to_string(entrypoint)?;
    if current != expected_content {
        return Err(AppError::Validation(format!(
            "LegacyOverlayChanged: {} changed while EMMM was preparing its migration; retry the runtime update",
            entrypoint.display()
        )));
    }

    let backup = legacy_entrypoint_backup_path(entrypoint)?;
    match std::fs::read_to_string(&backup) {
        Ok(existing) if existing == current => Ok(()),
        Ok(_) => Err(AppError::Validation(format!(
            "LegacyOverlayBackupConflict: {} already exists with different content; resolve it before regenerating KeyViewer",
            backup.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            generator::atomic_write(&backup, &current)
        }
        Err(error) => Err(error.into()),
    }
}

fn disable_owned_entrypoint(emmm_data_dir: &std::path::Path) -> Result<(), AppError> {
    let entrypoint = emmm_data_dir.join("KeyViewer.ini");
    if !entrypoint.exists() {
        return Ok(());
    }
    let content = std::fs::read(&entrypoint)?;
    if !is_current_emmm_entrypoint(&content) {
        return Err(AppError::Validation(format!(
            "AmbiguousOverlayOwnership: {} is not a current EMMM artifact; it was left unchanged",
            entrypoint.display()
        )));
    }
    let disabled = emmm_data_dir.join("KeyViewer.ini.emmm-disabled");
    if disabled.exists() {
        return Err(AppError::Validation(format!(
            "OverlayDisablePending: backup already exists at {}; resolve it before disabling the overlay",
            disabled.display()
        )));
    }
    std::fs::rename(entrypoint, disabled)?;
    Ok(())
}

fn normalized_windows_path(path: &std::path::Path) -> String {
    let normalized = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let rendered = normalized.to_string_lossy();
    rendered
        .strip_prefix(r"\\?\")
        .unwrap_or(rendered.as_ref())
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn is_disabled_include_path(path: &std::path::Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("DISABLED")
    })
}

fn duplicate_keyviewer_entrypoints(
    runtime_include_roots: &[PathBuf],
    primary_mods_root: &Path,
) -> Vec<PathBuf> {
    let expected =
        normalized_windows_path(&primary_mods_root.join(".emmm_data").join("KeyViewer.ini"));
    let mut seen_paths = HashSet::new();
    let mut duplicates = Vec::new();
    for root in runtime_include_roots {
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| entry.file_name().eq_ignore_ascii_case("KeyViewer.ini"))
            .filter(|entry| !is_disabled_include_path(entry.path()))
        {
            let normalized = normalized_windows_path(entry.path());
            if normalized == expected || !seen_paths.insert(normalized) {
                continue;
            }
            let Ok(content) = std::fs::read(entry.path()) else {
                continue;
            };
            if is_verified_emmm_entrypoint(&content) {
                duplicates.push(entry.into_path());
            }
        }
    }
    duplicates.sort();
    duplicates
}

#[derive(Debug)]
struct MigratedEntrypoint {
    original: PathBuf,
    backup: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DuplicateMigrationJournal {
    version: u8,
    generation_id: String,
    migrations: Vec<MigrationJournalEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MigrationJournalEntry {
    original: PathBuf,
    backup: PathBuf,
}

fn duplicate_migration_journal_path(emmm_data_dir: &Path) -> PathBuf {
    emmm_data_dir.join(KEYVIEWER_MIGRATION_JOURNAL_FILE)
}

fn write_duplicate_migration_journal(
    emmm_data_dir: &Path,
    generation_id: &str,
    migrations: &[MigratedEntrypoint],
) -> Result<(), AppError> {
    let journal = DuplicateMigrationJournal {
        version: 1,
        generation_id: generation_id.to_string(),
        migrations: migrations
            .iter()
            .map(|entry| MigrationJournalEntry {
                original: entry.original.clone(),
                backup: entry.backup.clone(),
            })
            .collect(),
    };
    let content = serde_json::to_string_pretty(&journal).map_err(|error| {
        AppError::Internal(format!(
            "Could not serialize KeyViewer migration journal: {error}"
        ))
    })?;
    generator::atomic_write(&duplicate_migration_journal_path(emmm_data_dir), &content)
}

fn clear_duplicate_migration_journal(emmm_data_dir: &Path) -> Result<(), AppError> {
    let path = duplicate_migration_journal_path(emmm_data_dir);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn recover_duplicate_migration(emmm_data_dir: &Path, entrypoint: &Path) -> Result<(), AppError> {
    let journal_path = duplicate_migration_journal_path(emmm_data_dir);
    let content = match std::fs::read_to_string(&journal_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let journal: DuplicateMigrationJournal = serde_json::from_str(&content).map_err(|error| {
        AppError::Validation(format!(
            "DuplicateOverlayMigrationRecovery: journal {} is invalid: {error}",
            journal_path.display()
        ))
    })?;
    if journal.version != 1 {
        return Err(AppError::Validation(format!(
            "DuplicateOverlayMigrationRecovery: unsupported journal version {}",
            journal.version
        )));
    }

    let published_generation = std::fs::read_to_string(entrypoint)
        .ok()
        .is_some_and(|content| {
            entrypoint_uses_generation(content.as_bytes(), &journal.generation_id)
        });
    if !published_generation {
        let migrations = journal
            .migrations
            .iter()
            .map(|entry| MigratedEntrypoint {
                original: entry.original.clone(),
                backup: entry.backup.clone(),
            })
            .collect::<Vec<_>>();
        rollback_duplicate_keyviewer_migrations(&migrations);
    }
    clear_duplicate_migration_journal(emmm_data_dir)
}

fn backup_path_for_entrypoint(path: &std::path::Path) -> Result<PathBuf, AppError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::Validation(format!("Invalid overlay path: {}", path.display())))?
        .to_string_lossy();
    let backup = path.with_file_name(format!("{file_name}.emmm-backup"));
    if backup.exists() {
        return Err(AppError::Validation(format!(
            "DuplicateOverlay: backup already exists at {}; resolve it before regeneration",
            backup.display()
        )));
    }
    Ok(backup)
}

fn migrate_duplicate_keyviewer_entrypoints(
    runtime_include_roots: &[PathBuf],
    primary_mods_root: &Path,
    emmm_data_dir: &Path,
    generation_id: &str,
) -> Result<Vec<MigratedEntrypoint>, AppError> {
    let migrations = duplicate_keyviewer_entrypoints(runtime_include_roots, primary_mods_root)
        .into_iter()
        .map(|original| {
            Ok(MigratedEntrypoint {
                backup: backup_path_for_entrypoint(&original)?,
                original,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    if migrations.is_empty() {
        return Ok(Vec::new());
    }
    write_duplicate_migration_journal(emmm_data_dir, generation_id, &migrations)?;

    let mut migrated = Vec::new();
    for migration in migrations {
        let MigratedEntrypoint { original, backup } = migration;
        if let Err(error) = std::fs::rename(&original, &backup) {
            rollback_duplicate_keyviewer_migrations(&migrated);
            let _ = clear_duplicate_migration_journal(emmm_data_dir);
            return Err(AppError::Io(format!(
                "DuplicateOverlay: could not back up {}: {error}",
                original.display()
            )));
        }
        migrated.push(MigratedEntrypoint { original, backup });
    }
    Ok(migrated)
}

fn rollback_duplicate_keyviewer_migrations(migrated: &[MigratedEntrypoint]) {
    for entrypoint in migrated.iter().rev() {
        if entrypoint.backup.exists() && !entrypoint.original.exists() {
            if let Err(error) = std::fs::rename(&entrypoint.backup, &entrypoint.original) {
                log::error!(
                    "Could not restore legacy EMMM overlay {} from {}: {error}",
                    entrypoint.original.display(),
                    entrypoint.backup.display()
                );
            }
        }
    }
}

#[derive(Debug, Clone)]
struct FallbackCandidate {
    mod_path: ModFolderPath,
    sentinels: Vec<matcher::RuntimeSentinel>,
    keybinds: Vec<crate::modules::library::application::ini::document::KeyBinding>,
}

#[derive(Debug)]
struct FallbackPanel {
    name: String,
    sentinels: Vec<matcher::RuntimeSentinel>,
    sources: Vec<generator::SourceKeyBinding>,
}

fn fallback_sentinels(targets: &[harvester::HarvestedTarget]) -> Vec<matcher::RuntimeSentinel> {
    matcher::select_geometry_first_sentinels(targets.iter().map(|target| {
        matcher::RuntimeSentinel {
            hash: target.hash.clone(),
            resource_kind: target.resource_kind,
            callback_slot: target.callback_slot.clone(),
            match_first_index: target.match_first_index,
            source: matcher::RuntimeSentinelSource::Harvest {
                section_name: target.section_name.clone(),
                file_path: target.file_path.clone(),
            },
        }
    }))
}

fn fallback_is_geometry(candidate: &FallbackCandidate) -> bool {
    candidate.sentinels.first().is_some_and(|sentinel| {
        matches!(
            sentinel.resource_kind,
            RuntimeResourceKind::PositionVb
                | RuntimeResourceKind::DrawVb
                | RuntimeResourceKind::VertexBuffer
        )
    })
}

fn same_sentinel(left: &matcher::RuntimeSentinel, right: &matcher::RuntimeSentinel) -> bool {
    left.stable_key() == right.stable_key()
}

fn fallback_name(paths: &[ModFolderPath]) -> String {
    let mut names: Vec<_> = paths
        .iter()
        .map(|path| path.folder_name().to_string())
        .collect();
    names.sort_by_key(|name| name.to_ascii_lowercase());
    let first = names.first().cloned().unwrap_or_else(|| "Mod".to_string());
    if paths.len() > 1 {
        format!("{first} + {}", paths.len() - 1)
    } else {
        first
    }
}

/// Group fallback mods only when their selected geometry observer is shared.
/// Index/texture collisions remain excluded instead of inventing ownership.
#[derive(Debug, Default, PartialEq, Eq)]
struct FallbackDiagnostics {
    missing_sentinels: usize,
    missing_keybinds: usize,
    ambiguous_hashes: bool,
}

type FallbackPanelGrouping = Option<(Vec<FallbackPanel>, FallbackDiagnostics)>;

fn group_fallback_panels(
    mut candidates: Vec<FallbackCandidate>,
    is_current: CurrentGenerationCheck<'_>,
    game_id: &str,
) -> Result<FallbackPanelGrouping, AppError> {
    candidates.sort_by(|left, right| left.mod_path.cmp(&right.mod_path));
    let mut diagnostics = FallbackDiagnostics::default();
    candidates.retain(|candidate| {
        if candidate.sentinels.is_empty() {
            diagnostics.missing_sentinels += 1;
        }
        if candidate.keybinds.is_empty() {
            diagnostics.missing_keybinds += 1;
        }
        let keep = !candidate.sentinels.is_empty() && !candidate.keybinds.is_empty();
        keep
    });

    let mut parent: Vec<usize> = (0..candidates.len()).collect();
    let mut first_sentinel_owner = BTreeMap::new();
    let mut first_geometry_owner = BTreeMap::new();
    let mut ambiguous = HashSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if composition_checkpoint_due(index)
            && !composition_checkpoint(is_current, game_id, "fallback_grouping", index)?
        {
            return Ok(None);
        }
        let is_geometry = fallback_is_geometry(candidate);
        let sentinel_keys = candidate
            .sentinels
            .iter()
            .map(matcher::RuntimeSentinel::stable_key)
            .collect::<BTreeSet<_>>();
        for key in sentinel_keys {
            if let Some(&other_index) = first_sentinel_owner.get(&key) {
                if !is_geometry {
                    ambiguous.insert(index);
                }
                if !fallback_is_geometry(&candidates[other_index]) {
                    ambiguous.insert(other_index);
                }
            } else {
                first_sentinel_owner.insert(key.clone(), index);
            }

            if is_geometry {
                if let Some(&other_index) = first_geometry_owner.get(&key) {
                    union_find_union(&mut parent, index, other_index);
                } else {
                    first_geometry_owner.insert(key, index);
                }
            }
        }
    }
    if !ambiguous.is_empty() {
        diagnostics.ambiguous_hashes = true;
    }

    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..candidates.len() {
        if composition_checkpoint_due(index)
            && !composition_checkpoint(is_current, game_id, "fallback_materialize", index)?
        {
            return Ok(None);
        }
        if ambiguous.contains(&index) {
            continue;
        }
        let root = union_find_root(&mut parent, index);
        groups.entry(root).or_default().push(index);
    }

    let mut panels = Vec::new();
    for indices in groups.into_values() {
        let mut paths = Vec::new();
        let mut sentinels = Vec::new();
        let mut sources = Vec::new();
        for index in indices {
            let candidate = &candidates[index];
            paths.push(candidate.mod_path.clone());
            sentinels.extend(candidate.sentinels.clone());
            sources.push(generator::SourceKeyBinding {
                mod_name: candidate.mod_path.folder_name().to_string(),
                keybinds: candidate.keybinds.clone(),
            });
        }
        sentinels.sort_by_key(|sentinel| sentinel.stable_key());
        sentinels.dedup_by(|left, right| same_sentinel(left, right));
        sources.sort_by(|left, right| {
            left.mod_name
                .to_ascii_lowercase()
                .cmp(&right.mod_name.to_ascii_lowercase())
        });
        panels.push(FallbackPanel {
            name: fallback_name(&paths),
            sentinels,
            sources,
        });
    }
    panels.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(Some((panels, diagnostics)))
}

fn union_find_root(parent: &mut [usize], index: usize) -> usize {
    if parent[index] != index {
        let root = union_find_root(parent, parent[index]);
        parent[index] = root;
    }
    parent[index]
}

fn union_find_union(parent: &mut [usize], left: usize, right: usize) {
    let left_root = union_find_root(parent, left);
    let right_root = union_find_root(parent, right);
    if left_root != right_root {
        parent[right_root] = left_root;
    }
}

/// Run tasks that should execute after any mod state change (Toggle, Apply, Switch).
///
/// Tasks include:
/// 1. Recomputing runtime signature (DB)
/// 2. Harvesting hashes from enabled mods
/// 3. Matching characters & generating KeyViewer.ini + keybind texts
/// 4. Refreshing conflict cache
/// 5. Updating runtime status banner
pub async fn run_post_apply_tasks(ctx: PostApplyContext) -> Result<(), AppError> {
    run_post_apply_tasks_with_options(ctx, false, None, None, RuntimeSyncRequest::Full)
        .await
        .map(|_| ())
}

async fn run_post_apply_tasks_with_options(
    ctx: PostApplyContext,
    force_publish: bool,
    expected_sync_revision: Option<u64>,
    activation_authority: Option<ActivationAuthority>,
    request: RuntimeSyncRequest,
) -> Result<PostApplyPublication, AppError> {
    let total_started = std::time::Instant::now();
    let pool = &ctx.pool;
    let game_id = &ctx.game_id;
    log::info!(
        "[post_apply] Starting post-apply tasks for game={} request={request:?}",
        game_id,
    );
    if !runtime_sync_request_is_current(
        game_id,
        expected_sync_revision,
        activation_authority.as_ref(),
    )? {
        return Ok(PostApplyPublication::Skipped);
    }

    let game_type = crate::modules::games::adapters::sqlite::game::get_game_type(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} not found")))?;
    let importer_root = sqlx::query("SELECT path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(pool)
        .await?
        .map(|row| PathBuf::from(row.get::<String, _>("path")))
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} not found")))?;
    if !runtime_sync_request_is_current(
        game_id,
        expected_sync_revision,
        activation_authority.as_ref(),
    )? {
        return Ok(PostApplyPublication::Skipped);
    }
    let preflight_started = std::time::Instant::now();
    let preflight_importer_root = importer_root.clone();
    let preflight_mods_path = ctx.mods_path.clone();
    let runtime_preflight = tokio::task::spawn_blocking(move || {
        read_runtime_preflight(&preflight_importer_root, &preflight_mods_path, game_type)
    })
    .await??;
    let preflight_ms = preflight_started.elapsed().as_millis();
    if !runtime_sync_request_is_current(
        game_id,
        expected_sync_revision,
        activation_authority.as_ref(),
    )? {
        return Ok(PostApplyPublication::Skipped);
    }
    for diagnostic in &runtime_preflight.diagnostics {
        log::warn!("[post_apply] {diagnostic}");
    }
    if importer_root.join("d3dx.ini").is_file() && !runtime_preflight.renderer_available {
        return Err(AppError::Validation(
            "RendererUnsupported: package text renderer is unavailable; KeyViewer was not published"
                .to_string(),
        ));
    }
    let mods_path = &runtime_preflight.runtime_mods_root;
    if !mods_path.is_dir() {
        return Err(AppError::NotFound(format!(
            "Mods folder not found: {}",
            mods_path.display()
        )));
    }

    // 3. KeyViewer Pipeline (Req-43)
    let emmm_data_dir = mods_path.join(".emmm_data");
    let entrypoint = emmm_data_dir.join("KeyViewer.ini");
    if generator::recover_atomic_write(&entrypoint)? {
        log::warn!(
            "Recovered KeyViewer entrypoint after an interrupted atomic replacement: {}",
            entrypoint.display()
        );
    }
    let manifest_file = manifest_path(&emmm_data_dir);
    let _ = generator::recover_atomic_write(&manifest_file)?;
    if !ctx.keyviewer_enabled {
        let duplicates =
            duplicate_keyviewer_entrypoints(&runtime_preflight.runtime_include_roots, mods_path);
        if !duplicates.is_empty() {
            return Err(AppError::Validation(format!(
                "DuplicateOverlay: {} additional verified EMMM entrypoint(s) remain in the effective include tree",
                duplicates.len()
            )));
        }
        let disabled = match expected_sync_revision {
            Some(revision) => commit_if_current_sync_revision(
                game_id,
                revision,
                activation_authority.as_ref(),
                || disable_owned_entrypoint(&emmm_data_dir),
            )?,
            None => Some(disable_owned_entrypoint(&emmm_data_dir)?),
        };
        if disabled.is_none() {
            return Ok(PostApplyPublication::Skipped);
        }
        remove_mod_composition(game_id)?;
        log::info!(
            "[post_apply] KeyViewer entrypoint disabled game={game_id} preflight_ms={preflight_ms} harvest_ms=0 publish_ms=0 total_ms={}",
            total_started.elapsed().as_millis()
        );
        return Ok(PostApplyPublication::Skipped);
    }

    // Harvest: one pass per mod for both hashes and keybinds.
    let harvest_started = std::time::Instant::now();
    let mut occurrence_counts = HashMap::new();
    let mut hash_to_mod_path = HashMap::new();
    let mut mod_harvests = HashMap::new();
    let mut effective_ini_fingerprints = Vec::new();
    let harvest_capabilities = harvester::HarvestCapabilities::from_callback_slots(
        runtime_preflight.callback_slots.iter(),
    );
    let harvest_metrics_before = harvester::harvest_cache_metrics();
    let is_current = || {
        runtime_sync_request_is_current(
            game_id,
            expected_sync_revision,
            activation_authority.as_ref(),
        )
    };
    let Some(prepared) = prepare_mod_composition(
        pool,
        game_id,
        mods_path,
        &harvest_capabilities,
        request,
        &is_current,
    )
    .await?
    else {
        return Ok(PostApplyPublication::Skipped);
    };
    if !runtime_sync_request_is_current(
        game_id,
        expected_sync_revision,
        activation_authority.as_ref(),
    )? {
        // A newer generation owns publication, but this disk-derived partial
        // composition is still a valid warm base for its merged request. A
        // burst must not turn an otherwise incremental follow-up into a full
        // rebuild merely because it superseded this generation mid-harvest.
        store_mod_composition(game_id, &prepared)?;
        return Ok(PostApplyPublication::Skipped);
    }
    let cache_committed = match expected_sync_revision {
        Some(revision) => commit_if_current_sync_revision(
            game_id,
            revision,
            activation_authority.as_ref(),
            || store_mod_composition(game_id, &prepared),
        )?,
        None => Some(store_mod_composition(game_id, &prepared)?),
    };
    if cache_committed.is_none() {
        store_mod_composition(game_id, &prepared)?;
        return Ok(PostApplyPublication::Skipped);
    }
    let harvest_metrics_after = harvester::harvest_cache_metrics();
    log::info!(
        "[post_apply] KeyViewer composition game={game_id} mode={:?} visited_mods={} total_mods={} cache_hits={} cache_misses={} harvested_count={} estimated_retained_entries={}",
        prepared.mode,
        prepared.harvested_mods,
        prepared.cache.mods.len(),
        harvest_metrics_after
            .cache_hits
            .saturating_sub(harvest_metrics_before.cache_hits),
        harvest_metrics_after
            .cache_misses
            .saturating_sub(harvest_metrics_before.cache_misses),
        harvest_metrics_after
            .harvested_mods
            .saturating_sub(harvest_metrics_before.harvested_mods),
        harvest_metrics_after.retained_entries,
    );

    for (index, contribution) in prepared.cache.mods.values().enumerate() {
        if composition_checkpoint_due(index)
            && !composition_checkpoint(&is_current, game_id, "composition_aggregate", index)?
        {
            return Ok(PostApplyPublication::Skipped);
        }
        let stored_path = &contribution.folder_path;
        let harvest = contribution.harvest.as_ref();

        effective_ini_fingerprints.extend(
            harvest
                .ini_fingerprints
                .iter()
                .map(|fingerprint| format!("{}:{fingerprint}", stored_path.as_stored())),
        );

        for target in &harvest.targets {
            *occurrence_counts.entry(target.hash.clone()).or_insert(0) += 1;
            hash_to_mod_path
                .entry(target.hash.clone())
                .or_insert_with(Vec::new)
                .push(stored_path.clone());
        }
        mod_harvests.insert(stored_path.clone(), Arc::clone(&contribution.harvest));
    }
    let harvest_ms = harvest_started.elapsed().as_millis();

    let (entries, catalog_diagnostic, catalog_checksum) = match catalog_app_data_dir(pool).await {
        Ok(catalog_dir) => {
            let checksum = catalog_manifest_checksum(&catalog_dir);
            match crate::modules::workspace::application::scanner::master_db::load_catalog_keyviewer_entries(
            &catalog_dir,
            game_type as i32,
        ) {
            Ok(catalog_entries) => {
                let has_runtime_targets = !catalog_entries.is_empty();
                let entries = kv_entries_from_catalog(catalog_entries, &runtime_preflight);
                let diagnostic = entries.is_empty().then_some(
                    if has_runtime_targets {
                        "CallbackUnsupported: catalog targets do not match active checktextureoverride slots"
                            .to_string()
                    } else {
                        "CatalogUpgradeRequired: no validated runtime targets are installed for this game"
                            .to_string()
                    },
                );
                (entries, diagnostic, checksum)
            }
            Err(error) => {
                let kind = if error.to_string().contains("not installed") {
                    "CatalogMissing"
                } else {
                    "CatalogInvalid"
                };
                (Vec::new(), Some(format!("{kind}: {error}")), checksum)
            }
        }
        }
        Err(error) => (
            Vec::new(),
            Some(format!("CatalogMissing: {error}")),
            "unavailable".to_string(),
        ),
    };
    if let Some(diagnostic) = catalog_diagnostic.as_deref() {
        log::warn!(
            "[post_apply] {diagnostic}; catalog character labels are unavailable, but eligible mods can use the hash-based fallback"
        );
    }

    // Match
    let config = matcher::MatchConfig;
    let active_hashes: HashSet<String> = occurrence_counts.keys().cloned().collect();
    let mut matches = matcher::match_objects(&entries, &active_hashes, &occurrence_counts, &config);

    // Catalog matches have a verified character identity. Keep their mod paths
    // out of the fallback so one mod cannot render both a character panel and
    // an unverified mod panel for the same harvested evidence.
    let mut catalog_matched_mods = HashSet::new();
    for matched in &matches {
        for hash in &matched.matched_hashes {
            if let Some(mod_paths) = hash_to_mod_path.get(hash) {
                catalog_matched_mods.extend(mod_paths.iter().cloned());
            }
        }
    }

    // Map keybinds back to objects, grouped by mod source (Req-43). A mod
    // that matches several characters has insufficient ownership evidence for
    // its generic [Key*] sections, so only explicitly character-named sections
    // are rendered for it instead of showing unrelated controls.
    let mut matched_objects_per_mod = HashMap::new();
    for matched in &matches {
        for hash in &matched.matched_hashes {
            if let Some(mod_paths) = hash_to_mod_path.get(hash) {
                for mod_path in mod_paths {
                    matched_objects_per_mod
                        .entry(mod_path.clone())
                        .or_insert_with(HashSet::new)
                        .insert(matched.object_name.clone());
                }
            }
        }
    }
    let mut sources_per_object = HashMap::new();
    for m in &matches {
        let mut object_sources = Vec::new();
        let mut seen_mod_paths = HashSet::new();

        for matched_hash in &m.matched_hashes {
            if let Some(mod_paths) = hash_to_mod_path.get(matched_hash) {
                for mp in mod_paths {
                    if seen_mod_paths.insert(mp) {
                        if let Some(harvest) = mod_harvests.get(mp) {
                            let kbs = &harvest.keybinds;
                            let matched_objects = matched_objects_per_mod
                                .get(mp)
                                .expect("mod path came from the matching ownership index");
                            let keybinds = if matched_objects.len() == 1 {
                                kbs.clone()
                            } else {
                                kbs.iter()
                                    .filter(|binding| {
                                        section_mentions_object(
                                            &binding.section_name,
                                            &m.object_name,
                                        )
                                    })
                                    .cloned()
                                    .collect()
                            };
                            if keybinds.is_empty() {
                                continue;
                            }
                            // Use the folder name as the mod name
                            let mod_name = mp.folder_name().to_string();

                            object_sources.push(generator::SourceKeyBinding { mod_name, keybinds });
                        }
                    }
                }
            }
        }
        sources_per_object.insert(m.object_name.clone(), object_sources);
    }

    // A catalog is enrichment, not a prerequisite. Fallback keeps only
    // observer-ready INI targets, uses terminal folder names, and combines
    // folders that share their selected geometry observer.
    let fallback_candidates = mod_harvests
        .iter()
        .filter(|(mod_path, _)| !catalog_matched_mods.contains(*mod_path))
        .map(|(mod_path, harvest)| FallbackCandidate {
            mod_path: mod_path.clone(),
            sentinels: fallback_sentinels(&harvest.targets),
            keybinds: harvest.keybinds.clone(),
        })
        .collect();
    let Some((fallback_panels, fallback_diagnostics)) =
        group_fallback_panels(fallback_candidates, &is_current, game_id)?
    else {
        return Ok(PostApplyPublication::Skipped);
    };
    if fallback_diagnostics.missing_sentinels > 0 || fallback_diagnostics.missing_keybinds > 0 {
        log::info!(
            "[post_apply] KeyViewer fallback excluded mods game={game_id} missing_sentinels={} missing_keybinds={}",
            fallback_diagnostics.missing_sentinels,
            fallback_diagnostics.missing_keybinds,
        );
    }
    if fallback_diagnostics.ambiguous_hashes {
        log::warn!("[post_apply] AmbiguousFallbackHash");
    }
    for panel in fallback_panels {
        let matched_hashes = panel
            .sentinels
            .iter()
            .map(|sentinel| sentinel.hash.clone())
            .collect();
        sources_per_object.insert(panel.name.clone(), panel.sources);
        matches.push(matcher::MatchResult {
            object_name: panel.name,
            object_type: "Mod".to_string(),
            score: 0.0,
            matched_hashes,
            sentinels: panel.sentinels,
            confidence: matcher::MatchConfidence::Low,
        });
    }

    let publish_started = std::time::Instant::now();
    if !runtime_sync_request_is_current(
        game_id,
        expected_sync_revision,
        activation_authority.as_ref(),
    )? {
        return Ok(PostApplyPublication::Skipped);
    }
    recover_duplicate_migration(&emmm_data_dir, &entrypoint)?;
    let legacy_entrypoint = if entrypoint.exists() {
        let content = std::fs::read(&entrypoint)?;
        if is_current_emmm_entrypoint(&content) {
            None
        } else if is_verified_emmm_entrypoint(&content) {
            Some(String::from_utf8(content).map_err(|_| {
                AppError::Validation(format!(
                    "LegacyOverlayEncoding: {} is not valid UTF-8 and was left unchanged",
                    entrypoint.display()
                ))
            })?)
        } else {
            return Err(AppError::Validation(format!(
                "AmbiguousOverlayOwnership: {} was left unchanged",
                entrypoint.display()
            )));
        }
    } else {
        None
    };

    // 4. Update Runtime Status (Req-42)
    //
    // A caller that already settled the applied collection supplies the answer
    // directly. Otherwise read the persisted label only, not a live projection.
    let caller_knows_preset = ctx
        .status_fields
        .as_ref()
        .is_some_and(|fields| fields.preset_name.is_some());

    let mut preset_name = None;
    if !caller_knows_preset {
        match active_preset_name(pool, game_id).await {
            Ok(name) => preset_name = name,
            Err(error) => log::warn!("[post_apply] Could not derive runtime status: {error}"),
        }
    }

    let mut status = generator::StatusFields {
        safe_mode: ctx.safe_mode,
        preset_name,
    };

    // Override with fields from the mutation source if provided
    if let Some(overrides) = ctx.status_fields.as_ref() {
        if overrides.preset_name.is_some() {
            status.preset_name = overrides.preset_name.clone();
        }
    }

    let fingerprint = runtime_input_fingerprint(
        &ctx,
        &importer_root,
        &runtime_preflight,
        &matches,
        &sources_per_object,
        &status,
        &effective_ini_fingerprints,
        &catalog_checksum,
    );
    if !force_publish && published_artifact_matches(&emmm_data_dir, &entrypoint, &fingerprint) {
        if !runtime_sync_request_is_current(
            game_id,
            expected_sync_revision,
            activation_authority.as_ref(),
        )? {
            return Ok(PostApplyPublication::Skipped);
        }
        log::info!(
            "[post_apply] KeyViewer inputs unchanged game={game_id} preflight_ms={preflight_ms} harvest_ms={harvest_ms} publish_ms={} total_ms={}",
            publish_started.elapsed().as_millis(),
            total_started.elapsed().as_millis(),
        );
        return Ok(PostApplyPublication::Unchanged);
    }
    if !runtime_sync_request_is_current(
        game_id,
        expected_sync_revision,
        activation_authority.as_ref(),
    )? {
        return Ok(PostApplyPublication::Skipped);
    }

    // Stage one immutable content-addressed generation. The entrypoint is the
    // only runtime pointer and is replaced last, so a crash cannot expose a
    // half-written resource tree or create a no-artifact window.
    let generation_id = format!("g{fingerprint}");
    let generations_dir = emmm_data_dir.join(KEYVIEWER_RESOURCE_ROOT);
    let generation_dir = generations_dir.join(&generation_id);
    let staging_artifacts = generator::create_staging_directory(&generation_dir)?;
    let resource_root = format!("{KEYVIEWER_RESOURCE_ROOT}/{generation_id}");
    let kv_ini_content = generator::generate_keyviewer_ini_for_resources(
        &matches,
        &ctx.hotkeys.toggle_overlay,
        game_type,
        &resource_root,
    )?;
    let staging_keybinds = staging_artifacts.join("keybinds").join("active");
    let staging_status = staging_artifacts.join("status");
    if let Err(write_error) = generator::write_keybind_files(
        &staging_keybinds,
        &matches,
        &sources_per_object,
        &ctx.hotkeys.toggle_overlay,
    ) {
        return Err(cleanup_staging_after_error(&staging_artifacts, write_error));
    }

    if let Err(error) = generator::write_status_file(&staging_status, &status, &ctx.hotkeys) {
        return Err(cleanup_staging_after_error(&staging_artifacts, error));
    }
    let manifest = KeyViewerManifest {
        version: KEYVIEWER_MANIFEST_VERSION,
        fingerprint,
        generation_id: generation_id.clone(),
    };
    let manifest_content = serde_json::to_string_pretty(&manifest).map_err(|error| {
        AppError::Internal(format!("Could not serialize KeyViewer manifest: {error}"))
    })?;
    let previous_manifest = std::fs::read_to_string(manifest_path(&emmm_data_dir)).ok();
    let publish = || -> Result<(), AppError> {
        if generation_dir.is_dir() {
            std::fs::remove_dir_all(&staging_artifacts)?;
        } else if let Err(error) = std::fs::rename(&staging_artifacts, &generation_dir) {
            return Err(error.into());
        }
        let migrated_entrypoints = migrate_duplicate_keyviewer_entrypoints(
            &runtime_preflight.runtime_include_roots,
            mods_path,
            &emmm_data_dir,
            &generation_id,
        )?;
        if let Some(content) = legacy_entrypoint.as_deref() {
            if let Err(error) = preserve_legacy_entrypoint(&entrypoint, content) {
                rollback_duplicate_keyviewer_migrations(&migrated_entrypoints);
                let _ = clear_duplicate_migration_journal(&emmm_data_dir);
                return Err(error);
            }
        }
        let manifest_file = manifest_path(&emmm_data_dir);
        if let Err(error) = generator::atomic_write(&manifest_file, &manifest_content) {
            rollback_duplicate_keyviewer_migrations(&migrated_entrypoints);
            let _ = clear_duplicate_migration_journal(&emmm_data_dir);
            return Err(error);
        }
        if let Err(error) = generator::atomic_write(&entrypoint, &kv_ini_content) {
            match previous_manifest.as_deref() {
                Some(previous) => {
                    let _ = generator::atomic_write(&manifest_file, previous);
                }
                None => {
                    let _ = std::fs::remove_file(&manifest_file);
                }
            }
            rollback_duplicate_keyviewer_migrations(&migrated_entrypoints);
            let _ = clear_duplicate_migration_journal(&emmm_data_dir);
            return Err(error);
        }
        if let Err(error) = clear_duplicate_migration_journal(&emmm_data_dir) {
            // The entrypoint is already the durable publication point. A
            // journal left behind is recovered against this generation.
            log::warn!("KeyViewer duplicate migration journal cleanup was deferred: {error}");
        }
        Ok(())
    };
    let committed = match expected_sync_revision {
        Some(revision) => commit_if_current_sync_revision(
            game_id,
            revision,
            activation_authority.as_ref(),
            publish,
        )?,
        None => Some(publish()?),
    };
    if committed.is_none() {
        let _ = std::fs::remove_dir_all(&staging_artifacts);
        return Ok(PostApplyPublication::Skipped);
    }

    log::info!(
        "[post_apply] Completed game={game_id} preflight_ms={preflight_ms} harvest_ms={harvest_ms} publish_ms={} total_ms={}",
        publish_started.elapsed().as_millis(),
        total_started.elapsed().as_millis(),
    );
    Ok(PostApplyPublication::Published)
}

fn post_apply_context_for_game(
    pool: &SqlitePool,
    settings: &crate::modules::settings::application::config::AppSettings,
    game_id: &str,
) -> Result<PostApplyContext, AppError> {
    let game = settings
        .games
        .iter()
        .find(|entry| entry.id == game_id)
        .ok_or_else(|| AppError::Internal(format!("Game {} not found", game_id)))?;
    Ok(PostApplyContext {
        pool: pool.clone(),
        game_id: game_id.to_string(),
        mods_path: game.mod_path.clone(),
        hotkeys: settings.hotkeys.clone(),
        keyviewer_enabled: settings.keyviewer.enabled,
        safe_mode: settings.safety.runtime_safe_mode_for(game_id),
        status_fields: None,
    })
}

fn refresh_context_from_persisted_settings(
    mut context: PostApplyContext,
    settings: &crate::modules::settings::application::config::AppSettings,
) -> Result<PostApplyContext, AppError> {
    let game = settings
        .games
        .iter()
        .find(|entry| entry.id == context.game_id)
        .ok_or_else(|| AppError::NotFound(format!("Game {} not found", context.game_id)))?;
    context.mods_path = game.mod_path.clone();
    context.hotkeys = settings.hotkeys.clone();
    context.keyviewer_enabled = settings.keyviewer.enabled;
    context.safe_mode = settings.safety.runtime_safe_mode_for(&context.game_id);
    Ok(context)
}

/// Backward-compatible full-recovery entrypoint for externally-triggered
/// KeyViewer synchronization.
pub async fn request_overlay_sync_for_game(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
) -> Result<RuntimeSyncResult, AppError> {
    request_overlay_sync_for_game_with_request(
        pool,
        config,
        game_id,
        cause,
        RuntimeSyncRequest::Full,
    )
    .await
}

/// Incremental entrypoint for committed mod mutations. The database and disk
/// mutation must be settled before enqueueing these outcomes.
pub async fn request_overlay_sync_for_game_scoped(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
    changes: Vec<RuntimeModChange>,
) -> Result<RuntimeSyncResult, AppError> {
    request_overlay_sync_for_game_with_request(
        pool,
        config,
        game_id,
        cause,
        RuntimeSyncRequest::Scoped { changes },
    )
    .await
}

/// Incremental entrypoint for parent/object toggles and watcher bursts. Roots
/// are stored paths relative to the game's Mods directory. Disable/rename
/// callers must include the pre-mutation root so cached descendants can be
/// evicted; include the post-mutation root too when it differs.
pub async fn request_overlay_sync_for_game_scoped_roots(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
    roots: Vec<ModFolderPath>,
) -> Result<RuntimeSyncResult, AppError> {
    request_overlay_sync_for_game_with_request(
        pool,
        config,
        game_id,
        cause,
        RuntimeSyncRequest::ScopedRoots { roots },
    )
    .await
}

async fn request_overlay_sync_for_game_with_request(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
    request: RuntimeSyncRequest,
) -> Result<RuntimeSyncResult, AppError> {
    let revision = request_sync_revision_with_request(game_id, request)?;
    let lock = sync_lock_for_game(game_id)?;
    let _guard = lock.lock().await;
    if !is_current_sync_revision(game_id, revision)? {
        return Ok(superseded_sync_result(cause));
    }
    let settings = config.get_settings();
    if settings.active_game_id.as_deref() != Some(game_id) {
        settle_sync_request(game_id, revision)?;
        return Ok(superseded_sync_result(cause));
    }
    let Some(activation_authority) = ActivationAuthority::current_for(game_id) else {
        settle_sync_request(game_id, revision)?;
        return Ok(superseded_sync_result(cause));
    };
    let context = post_apply_context_for_game(pool, &settings, game_id)?;
    if !is_current_sync_revision(game_id, revision)? {
        return Ok(superseded_sync_result(cause));
    }
    let Some(request) = sync_request_for_revision(game_id, revision)? else {
        return Ok(superseded_sync_result(cause));
    };
    synchronize_overlay_context(
        context,
        cause,
        Some(settings),
        Some(revision),
        Some(activation_authority),
        request,
    )
    .await
}

/// Variant used by committed mutation pipelines that already prepared the
/// status fields for the generation. It still uses the same game-scoped gate;
/// callers cannot bypass publication ordering by carrying a custom context.
pub async fn request_overlay_sync_with_context(
    context: PostApplyContext,
    cause: OverlaySyncCause,
) -> Result<RuntimeSyncResult, AppError> {
    let game_id = context.game_id.clone();
    let revision = request_sync_revision(&game_id)?;
    let lock = sync_lock_for_game(&game_id)?;
    let _guard = lock.lock().await;
    if !is_current_sync_revision(&game_id, revision)? {
        return Ok(superseded_sync_result(cause));
    }
    // A collection operation may have captured settings before it waited for
    // this per-game lock. Refresh only the persisted settings-derived fields
    // here, preserving its post-mutation status override, so it cannot publish
    // a stale shortcut/root over a newer Settings request.
    let settings =
        crate::modules::settings::application::config::ConfigService::load_from_db(&context.pool)
            .await?;
    if settings.active_game_id.as_deref() != Some(game_id.as_str()) {
        return Ok(superseded_sync_result(cause));
    }
    let Some(activation_authority) = ActivationAuthority::current_for(&game_id) else {
        return Ok(superseded_sync_result(cause));
    };
    let context = refresh_context_from_persisted_settings(context, &settings)?;
    if !is_current_sync_revision(&game_id, revision)? {
        return Ok(superseded_sync_result(cause));
    }
    synchronize_overlay_context(
        context,
        cause,
        Some(settings),
        Some(revision),
        Some(activation_authority),
        RuntimeSyncRequest::Full,
    )
    .await
}

async fn request_overlay_sync_for_game_at_revision(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
    revision: u64,
    activation_authority: Option<ActivationAuthority>,
) -> Result<RuntimeSyncResult, AppError> {
    let Some(activation_authority) = activation_authority else {
        if is_current_sync_revision(game_id, revision)? {
            settle_sync_request(game_id, revision)?;
        }
        return Ok(superseded_sync_result(cause));
    };
    let lock = sync_lock_for_game(game_id)?;
    let _guard = lock.lock().await;
    if !is_current_sync_revision(game_id, revision)? {
        return Ok(superseded_sync_result(cause));
    }
    let settings = config.get_settings();
    let context = post_apply_context_for_game(pool, &settings, game_id)?;
    if !is_current_sync_revision(game_id, revision)? {
        return Ok(superseded_sync_result(cause));
    }
    let Some(request) = sync_request_for_revision(game_id, revision)? else {
        return Ok(superseded_sync_result(cause));
    };
    synchronize_overlay_context(
        context,
        cause,
        Some(settings),
        Some(revision),
        Some(activation_authority),
        request,
    )
    .await
}

/// Retries only a failed pre-publication attempt. Input replay is never
/// retried: `NeedsManualReload` is a user-visible result, not a failed write.
pub async fn request_overlay_sync_with_retry_for_game(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
) -> Result<RuntimeSyncResult, AppError> {
    let first = request_overlay_sync_for_game(pool, config, game_id, cause).await?;
    if first.requires_retry() {
        request_overlay_sync_for_game(pool, config, game_id, cause).await
    } else {
        Ok(first)
    }
}

pub async fn request_overlay_sync_with_retry_for_game_scoped(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
    changes: Vec<RuntimeModChange>,
) -> Result<RuntimeSyncResult, AppError> {
    let first =
        request_overlay_sync_for_game_scoped(pool, config, game_id, cause, changes.clone()).await?;
    if first.requires_retry() {
        request_overlay_sync_for_game_scoped(pool, config, game_id, cause, changes).await
    } else {
        Ok(first)
    }
}

pub async fn request_overlay_sync_with_retry_for_game_scoped_roots(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
    roots: Vec<ModFolderPath>,
) -> Result<RuntimeSyncResult, AppError> {
    let first =
        request_overlay_sync_for_game_scoped_roots(pool, config, game_id, cause, roots.clone())
            .await?;
    if first.requires_retry() {
        request_overlay_sync_for_game_scoped_roots(pool, config, game_id, cause, roots).await
    } else {
        Ok(first)
    }
}

pub(crate) async fn request_overlay_sync_with_retry_for_game_at_revision(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
    revision: u64,
    activation_authority: Option<ActivationAuthority>,
) -> Result<RuntimeSyncResult, AppError> {
    let first = request_overlay_sync_for_game_at_revision(
        pool,
        config,
        game_id,
        cause,
        revision,
        activation_authority.clone(),
    )
    .await?;
    if first.requires_retry() && is_current_sync_revision(game_id, revision)? {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        if !is_current_sync_revision(game_id, revision)?
            || activation_authority
                .as_ref()
                .is_some_and(|authority| authority.with_current(|| ()).is_none())
        {
            return Ok(superseded_sync_result(cause));
        }
        request_overlay_sync_for_game_at_revision(
            pool,
            config,
            game_id,
            cause,
            revision,
            activation_authority,
        )
        .await
    } else {
        Ok(first)
    }
}

async fn synchronize_overlay_context(
    context: PostApplyContext,
    cause: OverlaySyncCause,
    reload_settings: Option<crate::modules::settings::application::config::AppSettings>,
    expected_sync_revision: Option<u64>,
    activation_authority: Option<ActivationAuthority>,
    request: RuntimeSyncRequest,
) -> Result<RuntimeSyncResult, AppError> {
    let game_id = context.game_id.clone();
    let emmm_data_dir = context.mods_path.join(".emmm_data");
    let publication_started = std::time::Instant::now();
    let publication = match run_post_apply_tasks_with_options(
        context,
        cause.forces_publish(),
        expected_sync_revision,
        activation_authority.clone(),
        request,
    )
    .await
    {
        Ok(publication) => match publication {
            PostApplyPublication::Published => RuntimeSyncPublication::Published,
            PostApplyPublication::Unchanged => RuntimeSyncPublication::Unchanged,
            PostApplyPublication::Skipped => RuntimeSyncPublication::Skipped,
        },
        Err(error) => {
            log::error!(
                "Runtime publication failed game_id={game_id} cause={cause:?} revision={expected_sync_revision:?}: {error}"
            );
            let result = RuntimeSyncResult {
                cause,
                publication: RuntimeSyncPublication::FailedBeforePublish,
                reload: RuntimeReloadOutcome::NotRequired,
                failure: Some(error.to_string()),
            };
            if !record_runtime_sync_result_if_current(
                &game_id,
                &result,
                expected_sync_revision,
                activation_authority.as_ref(),
            )? {
                return Ok(superseded_sync_result(cause));
            }
            return Ok(result);
        }
    };
    let publication_ms = publication_started.elapsed().as_millis();

    let reload_started = std::time::Instant::now();
    let reload = if matches!(
        publication,
        RuntimeSyncPublication::Published | RuntimeSyncPublication::Skipped
    ) && reload_settings
        .as_ref()
        .is_some_and(|settings| settings.active_game_id.as_deref() == Some(game_id.as_str()))
    {
        let settings = reload_settings
            .as_ref()
            .expect("reload settings were checked above");
        let send_reload = || {
            Ok(match crate::modules::automation::application::hotkeys::reload::trigger_reload_config(
                settings,
            ) {
                Ok(binding) => RuntimeReloadOutcome::ReloadSent { binding },
                Err(error) => RuntimeReloadOutcome::NeedsManualReload {
                    binding: crate::modules::automation::application::hotkeys::reload::configured_reload_config_binding(settings)
                        .ok(),
                    reason: error.to_string(),
                },
            })
        };
        match expected_sync_revision {
            Some(revision) => commit_if_current_sync_revision(
                &game_id,
                revision,
                activation_authority.as_ref(),
                send_reload,
            )?
            .unwrap_or(RuntimeReloadOutcome::NotRequired),
            None => send_reload()?,
        }
    } else {
        RuntimeReloadOutcome::NotRequired
    };
    let reload_ms = reload_started.elapsed().as_millis();

    let game_stopped = reload_settings
        .as_ref()
        .is_some_and(|settings| game_is_confirmed_stopped(settings, &game_id));
    if should_cleanup_generations(&reload, game_stopped) {
        if let Err(error) = cleanup_generation_siblings(&emmm_data_dir) {
            // Cleanup is intentionally deferred maintenance. It must never
            // turn a valid current snapshot into a failed synchronization.
            log::warn!("KeyViewer generation cleanup was deferred: {error}");
        }
    }

    let result = RuntimeSyncResult {
        cause,
        publication,
        reload,
        failure: None,
    };
    let authoritative = record_runtime_sync_result_if_current(
        &game_id,
        &result,
        expected_sync_revision,
        activation_authority.as_ref(),
    )?;
    if let Some(revision) = expected_sync_revision {
        settle_sync_request(&game_id, revision)?;
    }
    if !authoritative {
        return Ok(superseded_sync_result(cause));
    }
    log::info!(
        "runtime sync timing game_id={game_id} cause={cause:?} publication={:?} publish_ms={publication_ms} reload_ms={reload_ms}",
        result.publication,
    );
    Ok(result)
}

/// Compatibility wrapper for callers that only need the publication attempt.
pub async fn trigger_overlay_refresh_for_game(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
) -> Result<(), AppError> {
    request_overlay_sync_for_game(pool, config, game_id, OverlaySyncCause::SettingsChanged)
        .await
        .and_then(RuntimeSyncResult::ensure_success)
        .map(|_| ())
}

/// Convenience function to trigger a full overlay artifact regeneration for the active game.
/// Useful when settings (hotkeys or classification keywords) change without a mod mutation.
pub async fn trigger_overlay_refresh(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
) -> Result<(), AppError> {
    let game_id = config
        .with_settings(|settings| settings.active_game_id.clone())
        .ok_or_else(|| AppError::Internal("No active game".to_string()))?;
    request_overlay_sync_for_game(pool, config, &game_id, OverlaySyncCause::SettingsChanged)
        .await
        .and_then(RuntimeSyncResult::ensure_success)
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tempfile::TempDir;

    async fn file_backed_test_pool(database_path: &Path) -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(database_path)
                    .create_if_missing(true),
            )
            .await
            .expect("create file-backed test database");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrate file-backed test database");
        crate::modules::system::adapters::sqlite::utils::unicode_keys::ensure_unicode_keys(&pool)
            .await
            .expect("backfill unicode keys");
        pool
    }

    fn published_generation_dir(mods_path: &Path) -> PathBuf {
        let emmm_data = mods_path.join(".emmm_data");
        let manifest = read_manifest(&emmm_data).expect("published manifest");
        emmm_data
            .join(KEYVIEWER_RESOURCE_ROOT)
            .join(manifest.generation_id)
    }

    fn install_gimi_runtime_catalog(app_data_dir: &Path) {
        use sha2::Digest;

        let catalog = r#"{
            "entries": [{
                "name": "Arlecchino",
                "object_type": "Character",
                "runtime_targets": [{
                    "variant": "Default",
                    "component": "Body",
                    "resource_kind": "position_vb",
                    "hash": "A1B2C3D4",
                    "provenance": {
                        "source_repo": "github.com/example/catalog",
                        "commit": "fixture",
                        "path": "gimi/arlecchino/hash.json"
                    }
                }]
            }]
        }"#;
        let pack_dir = app_data_dir.join("asset-pack");
        std::fs::create_dir_all(&pack_dir).expect("create fixture catalog directory");
        std::fs::write(pack_dir.join("gimi.json"), catalog).expect("write fixture catalog");
        let checksum = format!("{:x}", sha2::Sha256::digest(catalog.as_bytes()));
        let manifest = serde_json::json!({
            "id": "fixture-catalog",
            "version": "2026.09.14",
            "author": "EMMM test",
            "source": "https://example.invalid/catalog",
            "catalogs": { "gimi": { "path": "gimi.json", "sha256": checksum } }
        });
        std::fs::write(
            pack_dir.join("manifest.json"),
            serde_json::to_vec(&manifest).expect("serialize fixture manifest"),
        )
        .expect("write fixture manifest");
    }

    struct CompositionFixture {
        _temp: TempDir,
        pool: SqlitePool,
        game_id: String,
        mods_root: PathBuf,
        first_mod_id: String,
        second_mod_id: String,
        third_mod_id: String,
        first_ini: PathBuf,
        second_ini: PathBuf,
        third_ini: PathBuf,
        capabilities: harvester::HarvestCapabilities,
    }

    impl CompositionFixture {
        fn change(
            &self,
            mod_id: &str,
            folder_path: &str,
            outcome: RuntimeModOutcome,
        ) -> RuntimeModChange {
            RuntimeModChange {
                mod_id: mod_id.to_string(),
                folder_path: ModFolderPath::from_stored(folder_path),
                outcome,
            }
        }
    }

    fn current_generation() -> Result<bool, AppError> {
        Ok(true)
    }

    async fn composition_fixture(game_id: &str) -> CompositionFixture {
        let temp = TempDir::new().unwrap();
        let pool = file_backed_test_pool(&temp.path().join("app.db")).await;
        let importer = temp.path().join("importer");
        let mods_root = importer.join("Mods");
        let first_dir = mods_root.join("Parent").join("Mod A");
        let second_dir = mods_root.join("Parent").join("Mod B");
        let third_dir = mods_root.join("Other").join("Mod C");
        std::fs::create_dir_all(&first_dir).unwrap();
        std::fs::create_dir_all(&second_dir).unwrap();
        std::fs::create_dir_all(&third_dir).unwrap();
        let first_ini = first_dir.join("mod.ini");
        let second_ini = second_dir.join("mod.ini");
        let third_ini = third_dir.join("mod.ini");
        std::fs::write(
            &first_ini,
            "[TextureOverrideFirstPosition]\nhash = 11111111\nvb0 = ResourceFirst\n",
        )
        .unwrap();
        std::fs::write(
            &second_ini,
            "[TextureOverrideSecondPosition]\nhash = 22222222\nvb0 = ResourceSecond\n",
        )
        .unwrap();
        std::fs::write(
            &third_ini,
            "[TextureOverrideThirdPosition]\nhash = 33333333\nvb0 = ResourceThird\n",
        )
        .unwrap();
        let importer_path = importer.to_string_lossy().into_owned();
        let mods_path = mods_root.to_string_lossy().into_owned();
        crate::test_utils::insert_test_game(
            &pool,
            &crate::test_utils::TestGameFixture {
                id: game_id,
                name: "GIMI",
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                path: &importer_path,
                mods_path: Some(&mods_path),
            },
        )
        .await
        .unwrap();
        let first_mod_id = format!("{game_id}-a");
        let second_mod_id = format!("{game_id}-b");
        let third_mod_id = format!("{game_id}-c");
        for (id, name, folder_path) in [
            (&first_mod_id, "Mod A", "Parent/Mod A"),
            (&second_mod_id, "Mod B", "Parent/Mod B"),
            (&third_mod_id, "Mod C", "Other/Mod C"),
        ] {
            crate::test_utils::insert_test_mod(
                &pool,
                &crate::test_utils::TestModFixture {
                    id,
                    game_id,
                    object_id: None,
                    actual_name: name,
                    folder_path,
                    status: crate::modules::games::domain::models::ItemStatus::Enabled,
                    is_safe: true,
                    object_type: Some("Other"),
                    mods_path: Some(&mods_path),
                },
            )
            .await
            .unwrap();
        }
        CompositionFixture {
            _temp: temp,
            pool,
            game_id: game_id.to_string(),
            mods_root,
            first_mod_id,
            second_mod_id,
            third_mod_id,
            first_ini,
            second_ini,
            third_ini,
            capabilities: harvester::HarvestCapabilities::from_callback_slots(["vb0"]),
        }
    }

    async fn seed_composition_cache(fixture: &CompositionFixture) -> PreparedModComposition {
        let prepared = prepare_mod_composition(
            &fixture.pool,
            &fixture.game_id,
            &fixture.mods_root,
            &fixture.capabilities,
            RuntimeSyncRequest::Full,
            &current_generation,
        )
        .await
        .unwrap()
        .expect("full composition should remain current");
        store_mod_composition(&fixture.game_id, &prepared).unwrap();
        prepared
    }

    #[test]
    fn composition_identity_change_evicts_the_old_harvest_root() {
        let temp = tempfile::tempdir().unwrap();
        let old_root = temp.path().join("OldMods");
        let new_root = temp.path().join("NewMods");
        let mod_path = old_root.join("Example");
        std::fs::create_dir_all(&mod_path).unwrap();
        std::fs::create_dir_all(&new_root).unwrap();
        std::fs::write(
            mod_path.join("mod.ini"),
            "[TextureOverrideBody]\nhash = df65bb00\nvb0 = ResourceBody\n",
        )
        .unwrap();
        let capabilities = harvester::HarvestCapabilities::from_callback_slots(["vb0"]);
        harvester::harvest_mod(&mod_path, &capabilities).unwrap();
        assert_eq!(harvester::cached_entry_count_for_root(&old_root), 1);

        let (mods_root, capability_slots) = composition_cache_identity(&old_root, &capabilities);
        let prepared = PreparedModComposition {
            cache: Arc::new(GameCompositionCache {
                mods_root,
                capability_slots,
                mods: BTreeMap::new(),
            }),
            mode: CompositionMode::Full,
            harvested_mods: 0,
        };
        let game_id = "composition-root-change";
        store_mod_composition(game_id, &prepared).unwrap();

        assert!(
            take_cached_mod_composition(game_id, &new_root, &capabilities)
                .unwrap()
                .is_none()
        );
        assert_eq!(harvester::cached_entry_count_for_root(&old_root), 0);
    }

    #[test]
    fn large_game_composition_remains_incrementally_cacheable() {
        let temp = tempfile::tempdir().unwrap();
        let capabilities = harvester::HarvestCapabilities::default();
        let (mods_root, capability_slots) = composition_cache_identity(temp.path(), &capabilities);
        let mods = (0..5_000)
            .map(|index| {
                (
                    format!("mod-{index}"),
                    CachedModContribution {
                        folder_path: ModFolderPath::from_stored(format!("Object/Mod {index}")),
                        harvest: Arc::new(harvester::ModHarvest::default()),
                    },
                )
            })
            .collect();
        let prepared = PreparedModComposition {
            cache: Arc::new(GameCompositionCache {
                mods_root,
                capability_slots,
                mods,
            }),
            mode: CompositionMode::Full,
            harvested_mods: 5_000,
        };

        store_mod_composition("large-cache-game", &prepared).unwrap();
        drop(prepared);
        let cached = take_cached_mod_composition("large-cache-game", temp.path(), &capabilities)
            .unwrap()
            .expect("large game cache should remain available");

        assert_eq!(cached.mods.len(), 5_000);
    }

    #[test]
    fn scoped_cache_lease_restores_warm_state_on_early_exit() {
        let temp = tempfile::tempdir().unwrap();
        let capabilities = harvester::HarvestCapabilities::default();
        let (mods_root, capability_slots) = composition_cache_identity(temp.path(), &capabilities);
        let game_id = "scoped-cache-lease-restore";
        let cache = GameCompositionCache {
            mods_root,
            capability_slots,
            mods: BTreeMap::from([(
                "mod-a".to_string(),
                CachedModContribution {
                    folder_path: ModFolderPath::from_stored("Object/Mod A"),
                    harvest: Arc::new(harvester::ModHarvest::default()),
                },
            )]),
        };

        drop(CompositionCacheLease::new(game_id, cache));

        let restored = take_cached_mod_composition(game_id, temp.path(), &capabilities)
            .unwrap()
            .expect("lease drop should restore the warm cache");
        assert!(restored.mods.contains_key("mod-a"));
    }

    #[tokio::test]
    async fn scoped_composition_stops_at_a_bounded_supersession_checkpoint() {
        let temp = tempfile::tempdir().unwrap();
        let capabilities = harvester::HarvestCapabilities::default();
        let (mods_root, capability_slots) = composition_cache_identity(temp.path(), &capabilities);
        let harvest = Arc::new(harvester::ModHarvest::default());
        let mods = (0..256)
            .map(|index| {
                (
                    format!("mod-{index}"),
                    CachedModContribution {
                        folder_path: ModFolderPath::from_stored(format!("Object/Mod {index}")),
                        harvest: Arc::clone(&harvest),
                    },
                )
            })
            .collect();
        let prepared = PreparedModComposition {
            cache: Arc::new(GameCompositionCache {
                mods_root,
                capability_slots,
                mods,
            }),
            mode: CompositionMode::Full,
            harvested_mods: 256,
        };
        let game_id = "superseded-scoped-composition";
        store_mod_composition(game_id, &prepared).unwrap();
        drop(prepared);

        let checks = std::sync::atomic::AtomicUsize::new(0);
        let is_current = || Ok(checks.fetch_add(1, Ordering::Relaxed) < 2);
        let changes = (0..256)
            .map(|index| RuntimeModChange {
                mod_id: format!("mod-{index}"),
                folder_path: ModFolderPath::from_stored(format!("Object/Mod {index}")),
                outcome: RuntimeModOutcome::Disabled,
            })
            .collect();

        let prepared = prepare_scoped_mod_composition(
            game_id,
            temp.path(),
            &capabilities,
            changes,
            &is_current,
        )
        .await
        .unwrap()
        .expect("superseded scoped work should retain a coherent warm cache");

        assert_eq!(prepared.cache.mods.len(), 256);
        assert_eq!(prepared.harvested_mods, 0);
        assert_eq!(checks.load(Ordering::Relaxed), 3);
    }

    #[test]
    #[ignore = "manual 100k-entry composition cache benchmark"]
    fn benchmark_100k_entry_composition_cache_update() {
        const ENTRY_COUNT: usize = 100_000;
        const SAMPLE_COUNT: usize = 31;

        let temp = tempfile::tempdir().unwrap();
        let capabilities = harvester::HarvestCapabilities::default();
        let (mods_root, capability_slots) = composition_cache_identity(temp.path(), &capabilities);
        let harvest = Arc::new(harvester::ModHarvest {
            targets: vec![harvester::HarvestedTarget {
                hash: "df65bb00".to_string(),
                resource_kind: RuntimeResourceKind::PositionVb,
                callback_slot: "vb0".to_string(),
                match_first_index: Some(0),
                section_name: "TextureOverrideBody".to_string(),
                file_path: PathBuf::from("mod.ini"),
            }],
            keybinds: vec![
                crate::modules::library::application::ini::document::KeyBinding {
                    section_name: "KeyToggle".to_string(),
                    key: Some("F1".to_string()),
                    back: None,
                    binding_type: Some("toggle".to_string()),
                    condition: None,
                    key_line_idx: Some(1),
                    back_line_idx: None,
                },
            ],
            ini_fingerprints: vec!["fixture-fingerprint".to_string()],
        });
        let mods = (0..ENTRY_COUNT)
            .map(|index| {
                (
                    format!("mod-{index:06}"),
                    CachedModContribution {
                        folder_path: ModFolderPath::from_stored(format!("Object/Mod {index}")),
                        harvest: Arc::clone(&harvest),
                    },
                )
            })
            .collect();
        let game_id = "benchmark-100k-composition";
        let prepared = PreparedModComposition {
            cache: Arc::new(GameCompositionCache {
                mods_root,
                capability_slots,
                mods,
            }),
            mode: CompositionMode::Full,
            harvested_mods: ENTRY_COUNT,
        };
        let estimated_bytes = prepared.cache.mods.iter().fold(
            std::mem::size_of::<(String, CachedModContribution)>() * ENTRY_COUNT,
            |total, (mod_id, contribution)| {
                total
                    .saturating_add(mod_id.capacity())
                    .saturating_add(contribution.folder_path.as_stored().len())
            },
        );
        store_mod_composition(game_id, &prepared).unwrap();
        drop(prepared);

        let mut samples = Vec::with_capacity(SAMPLE_COUNT);
        for sample in 0..SAMPLE_COUNT {
            let started = std::time::Instant::now();
            let mut cache = take_cached_mod_composition(game_id, temp.path(), &capabilities)
                .unwrap()
                .expect("benchmark composition should remain cached");
            let contribution = cache
                .mods
                .get_mut("mod-050000")
                .expect("benchmark update target should exist");
            contribution.folder_path =
                ModFolderPath::from_stored(format!("Object/Updated {sample}"));
            assert_eq!(cache.mods.len(), ENTRY_COUNT);
            let prepared = PreparedModComposition {
                cache: Arc::new(cache),
                mode: CompositionMode::Scoped,
                harvested_mods: 1,
            };
            store_mod_composition(game_id, &prepared).unwrap();
            drop(prepared);
            samples.push(started.elapsed());
        }
        let cache = take_cached_mod_composition(game_id, temp.path(), &capabilities)
            .unwrap()
            .expect("benchmark composition should remain cached for aggregation");
        let mut aggregation_samples = Vec::with_capacity(7);
        for _ in 0..7 {
            let started = std::time::Instant::now();
            let mut occurrence_counts = HashMap::new();
            let mut hash_to_mod_path = HashMap::new();
            let mut mod_harvests = HashMap::new();
            let mut fingerprints = Vec::new();
            for contribution in cache.mods.values() {
                let path = &contribution.folder_path;
                for fingerprint in &contribution.harvest.ini_fingerprints {
                    fingerprints.push(format!("{}:{fingerprint}", path.as_stored()));
                }
                for target in &contribution.harvest.targets {
                    *occurrence_counts.entry(target.hash.clone()).or_insert(0) += 1;
                    hash_to_mod_path
                        .entry(target.hash.clone())
                        .or_insert_with(Vec::new)
                        .push(path.clone());
                }
                mod_harvests.insert(path.clone(), Arc::clone(&contribution.harvest));
            }
            std::hint::black_box((
                occurrence_counts,
                hash_to_mod_path,
                mod_harvests,
                fingerprints,
            ));
            aggregation_samples.push(started.elapsed());
        }

        samples.sort_unstable();
        aggregation_samples.sort_unstable();
        let p50 = samples[(SAMPLE_COUNT * 50 + 99) / 100 - 1];
        let p95 = samples[(SAMPLE_COUNT * 95 + 99) / 100 - 1];
        let aggregation_p50 = aggregation_samples[3];
        let aggregation_p95 = aggregation_samples[6];
        eprintln!(
            "KeyViewer 100k-entry composition: samples={SAMPLE_COUNT} estimated_bytes={estimated_bytes} update_p50_us={} update_p95_us={} aggregate_p50_ms={} aggregate_p95_ms={}",
            p50.as_micros(),
            p95.as_micros(),
            aggregation_p50.as_millis(),
            aggregation_p95.as_millis(),
        );
    }

    #[tokio::test]
    async fn scoped_sync_does_not_read_unchanged_mods() {
        let fixture = composition_fixture("keyviewer-scoped-unchanged").await;
        seed_composition_cache(&fixture).await;
        let first_reads = harvester::harvest_read_count(&fixture.first_ini);
        let second_reads = harvester::harvest_read_count(&fixture.second_ini);
        let third_reads = harvester::harvest_read_count(&fixture.third_ini);

        let prepared = prepare_mod_composition(
            &fixture.pool,
            &fixture.game_id,
            &fixture.mods_root,
            &fixture.capabilities,
            RuntimeSyncRequest::Scoped { changes: vec![] },
            &current_generation,
        )
        .await
        .unwrap()
        .expect("scoped composition should remain current");

        assert_eq!(prepared.mode, CompositionMode::Scoped);
        assert_eq!(prepared.harvested_mods, 0);
        assert_eq!(
            harvester::harvest_read_count(&fixture.first_ini),
            first_reads
        );
        assert_eq!(
            harvester::harvest_read_count(&fixture.second_ini),
            second_reads
        );
        assert_eq!(
            harvester::harvest_read_count(&fixture.third_ini),
            third_reads
        );
        assert_eq!(prepared.cache.mods.len(), 3);
    }

    #[tokio::test]
    async fn composition_and_harvest_cache_share_the_same_allocation() {
        let fixture = composition_fixture("keyviewer-shared-harvest-allocation").await;
        let prepared = seed_composition_cache(&fixture).await;
        let first_mod = fixture
            .first_ini
            .parent()
            .expect("fixture INI should have a mod parent");
        let cached = harvester::harvest_mod(first_mod, &fixture.capabilities).unwrap();

        assert!(Arc::ptr_eq(
            &prepared.cache.mods[&fixture.first_mod_id].harvest,
            &cached,
        ));
    }

    #[tokio::test]
    async fn scoped_disable_removes_only_that_mod_contribution() {
        let fixture = composition_fixture("keyviewer-scoped-disable").await;
        seed_composition_cache(&fixture).await;
        let first_reads = harvester::harvest_read_count(&fixture.first_ini);
        let second_reads = harvester::harvest_read_count(&fixture.second_ini);

        let prepared = prepare_mod_composition(
            &fixture.pool,
            &fixture.game_id,
            &fixture.mods_root,
            &fixture.capabilities,
            RuntimeSyncRequest::Scoped {
                changes: vec![fixture.change(
                    &fixture.first_mod_id,
                    "Parent/DISABLED Mod A",
                    RuntimeModOutcome::Disabled,
                )],
            },
            &current_generation,
        )
        .await
        .unwrap()
        .expect("scoped composition should remain current");

        assert!(!prepared.cache.mods.contains_key(&fixture.first_mod_id));
        assert!(prepared.cache.mods.contains_key(&fixture.second_mod_id));
        assert!(prepared.cache.mods.contains_key(&fixture.third_mod_id));
        assert_eq!(
            harvester::harvest_read_count(&fixture.first_ini),
            first_reads
        );
        assert_eq!(
            harvester::harvest_read_count(&fixture.second_ini),
            second_reads
        );
    }

    #[tokio::test]
    async fn scoped_enabled_change_reharvests_only_that_mod() {
        let fixture = composition_fixture("keyviewer-scoped-change").await;
        seed_composition_cache(&fixture).await;
        let first_reads = harvester::harvest_read_count(&fixture.first_ini);
        let second_reads = harvester::harvest_read_count(&fixture.second_ini);
        std::fs::write(
            &fixture.first_ini,
            "[TextureOverrideFirstPosition]\nhash = a1b2c3d4\nvb0 = ResourceFirst\n",
        )
        .unwrap();

        let prepared = prepare_mod_composition(
            &fixture.pool,
            &fixture.game_id,
            &fixture.mods_root,
            &fixture.capabilities,
            RuntimeSyncRequest::Scoped {
                changes: vec![fixture.change(
                    &fixture.first_mod_id,
                    "Parent/Mod A",
                    RuntimeModOutcome::Enabled,
                )],
            },
            &current_generation,
        )
        .await
        .unwrap()
        .expect("scoped composition should remain current");

        assert_eq!(prepared.harvested_mods, 1);
        assert_eq!(
            prepared.cache.mods[&fixture.first_mod_id].harvest.targets[0].hash,
            "a1b2c3d4"
        );
        assert_eq!(
            prepared.cache.mods[&fixture.second_mod_id].harvest.targets[0].hash,
            "22222222"
        );
        assert_eq!(
            harvester::harvest_read_count(&fixture.first_ini),
            first_reads + 1
        );
        assert_eq!(
            harvester::harvest_read_count(&fixture.second_ini),
            second_reads
        );
    }

    #[tokio::test]
    async fn scoped_root_replaces_only_currently_enabled_descendants() {
        let fixture = composition_fixture("keyviewer-scoped-root").await;
        seed_composition_cache(&fixture).await;
        let first_reads = harvester::harvest_read_count(&fixture.first_ini);
        let second_reads = harvester::harvest_read_count(&fixture.second_ini);
        let third_reads = harvester::harvest_read_count(&fixture.third_ini);
        sqlx::query("UPDATE mods SET status = 0 WHERE id = ?")
            .bind(&fixture.first_mod_id)
            .execute(&fixture.pool)
            .await
            .unwrap();
        std::fs::write(
            &fixture.second_ini,
            "[TextureOverrideSecondPosition]\nhash = b1c2d3e4\nvb0 = ResourceSecond\n",
        )
        .unwrap();

        let prepared = prepare_mod_composition(
            &fixture.pool,
            &fixture.game_id,
            &fixture.mods_root,
            &fixture.capabilities,
            RuntimeSyncRequest::ScopedRoots {
                roots: vec![ModFolderPath::from_stored("Parent")],
            },
            &current_generation,
        )
        .await
        .unwrap()
        .expect("root-scoped composition should remain current");

        assert_eq!(prepared.mode, CompositionMode::ScopedRoots);
        assert_eq!(prepared.harvested_mods, 1);
        assert!(!prepared.cache.mods.contains_key(&fixture.first_mod_id));
        assert_eq!(
            prepared.cache.mods[&fixture.second_mod_id].harvest.targets[0].hash,
            "b1c2d3e4"
        );
        assert_eq!(
            prepared.cache.mods[&fixture.third_mod_id].harvest.targets[0].hash,
            "33333333"
        );
        assert_eq!(
            harvester::harvest_read_count(&fixture.first_ini),
            first_reads
        );
        assert_eq!(
            harvester::harvest_read_count(&fixture.second_ini),
            second_reads + 1
        );
        assert_eq!(
            harvester::harvest_read_count(&fixture.third_ini),
            third_reads
        );
    }

    #[tokio::test]
    async fn cold_scoped_root_sync_falls_back_to_a_complete_rebuild() {
        let fixture = composition_fixture("keyviewer-scoped-cold-fallback").await;

        let prepared = prepare_mod_composition(
            &fixture.pool,
            &fixture.game_id,
            &fixture.mods_root,
            &fixture.capabilities,
            RuntimeSyncRequest::ScopedRoots {
                roots: vec![ModFolderPath::from_stored("Parent")],
            },
            &current_generation,
        )
        .await
        .unwrap()
        .expect("fallback composition should remain current");

        assert_eq!(prepared.mode, CompositionMode::FullFallback);
        assert_eq!(prepared.harvested_mods, 3);
        assert!(prepared.cache.mods.contains_key(&fixture.first_mod_id));
        assert!(prepared.cache.mods.contains_key(&fixture.second_mod_id));
        assert!(prepared.cache.mods.contains_key(&fixture.third_mod_id));
        assert_eq!(harvester::harvest_read_count(&fixture.first_ini), 1);
        assert_eq!(harvester::harvest_read_count(&fixture.second_ini), 1);
        assert_eq!(harvester::harvest_read_count(&fixture.third_ini), 1);
    }

    fn fallback_candidate(
        stored_path: &str,
        hash: &str,
        resource_kind: RuntimeResourceKind,
        match_first_index: Option<u32>,
    ) -> FallbackCandidate {
        FallbackCandidate {
            mod_path: ModFolderPath::from_stored(stored_path),
            sentinels: vec![matcher::RuntimeSentinel {
                hash: hash.to_string(),
                resource_kind,
                callback_slot: match resource_kind {
                    RuntimeResourceKind::PositionVb => "vb0",
                    RuntimeResourceKind::DrawVb | RuntimeResourceKind::VertexBuffer => "vb1",
                    RuntimeResourceKind::IndexBuffer => "ib",
                    RuntimeResourceKind::Texture => "ps-t0",
                    RuntimeResourceKind::Shader => "",
                }
                .to_string(),
                match_first_index,
                source: matcher::RuntimeSentinelSource::Harvest {
                    section_name: "TextureOverrideFixture".to_string(),
                    file_path: PathBuf::from("fixture.ini"),
                },
            }],
            keybinds: vec![
                crate::modules::library::application::ini::document::KeyBinding {
                    section_name: "KeyFixture".to_string(),
                    key: Some("CTRL+[".to_string()),
                    back: None,
                    binding_type: None,
                    condition: None,
                    key_line_idx: None,
                    back_line_idx: None,
                },
            ],
        }
    }

    #[test]
    fn fallback_groups_shared_geometry_into_one_folder_named_panel() {
        let (panels, diagnostics) = group_fallback_panels(
            vec![
                fallback_candidate(
                    "Character/Mod B",
                    "6895f405",
                    RuntimeResourceKind::PositionVb,
                    None,
                ),
                fallback_candidate(
                    "Character/Mod A",
                    "6895f405",
                    RuntimeResourceKind::PositionVb,
                    None,
                ),
            ],
            &|| Ok(true),
            "test-game",
        )
        .unwrap()
        .unwrap();

        assert_eq!(diagnostics, FallbackDiagnostics::default());
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].name, "Mod A + 1");
        assert_eq!(panels[0].sentinels.len(), 1);
        assert_eq!(
            panels[0]
                .sources
                .iter()
                .map(|source| source.mod_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Mod A", "Mod B"]
        );
    }

    #[test]
    fn fallback_drops_shared_texture_without_inventing_a_panel_owner() {
        let (panels, diagnostics) = group_fallback_panels(
            vec![
                fallback_candidate(
                    "Character/Face A",
                    "a44625da",
                    RuntimeResourceKind::Texture,
                    None,
                ),
                fallback_candidate(
                    "Character/Face B",
                    "a44625da",
                    RuntimeResourceKind::Texture,
                    None,
                ),
            ],
            &|| Ok(true),
            "test-game",
        )
        .unwrap()
        .unwrap();

        assert!(panels.is_empty());
        assert_eq!(
            diagnostics,
            FallbackDiagnostics {
                ambiguous_hashes: true,
                ..FallbackDiagnostics::default()
            }
        );
    }

    #[test]
    fn fallback_reports_exclusion_counts_without_one_warning_per_mod() {
        let mut missing_sentinel = fallback_candidate(
            "Character/No Sentinel",
            "a44625da",
            RuntimeResourceKind::PositionVb,
            None,
        );
        missing_sentinel.sentinels.clear();
        let mut missing_keybind = fallback_candidate(
            "Character/No Keybind",
            "b44625da",
            RuntimeResourceKind::PositionVb,
            None,
        );
        missing_keybind.keybinds.clear();

        let (panels, diagnostics) = group_fallback_panels(
            vec![missing_sentinel, missing_keybind],
            &|| Ok(true),
            "test-game",
        )
        .unwrap()
        .unwrap();

        assert!(panels.is_empty());
        assert_eq!(
            diagnostics,
            FallbackDiagnostics {
                missing_sentinels: 1,
                missing_keybinds: 1,
                ..FallbackDiagnostics::default()
            }
        );
    }

    #[test]
    fn fallback_grouping_stops_when_a_newer_generation_supersedes_it() {
        let checks = std::sync::atomic::AtomicUsize::new(0);
        let candidates = (0..128)
            .map(|index| {
                fallback_candidate(
                    &format!("Character/Mod {index}"),
                    &format!("{index:08x}"),
                    RuntimeResourceKind::PositionVb,
                    None,
                )
            })
            .collect();

        let grouped = group_fallback_panels(
            candidates,
            &|| Ok(checks.fetch_add(1, Ordering::Relaxed) == 0),
            "test-game",
        )
        .unwrap();

        assert!(grouped.is_none());
        assert!(checks.load(Ordering::Relaxed) >= 2);
    }

    /// Manual end-to-end fallback grouping benchmark. This includes sentinel
    /// indexing, union-find grouping, panel materialization, and sorting.
    #[test]
    #[ignore = "manual 100k fallback grouping benchmark"]
    fn benchmark_100k_fallback_grouping() {
        const COUNT: usize = 100_000;
        let candidates = (0..COUNT)
            .map(|index| {
                fallback_candidate(
                    &format!("Character/Mod {index:06}"),
                    &format!("{index:08x}"),
                    RuntimeResourceKind::PositionVb,
                    None,
                )
            })
            .collect();
        let started = std::time::Instant::now();
        let (panels, diagnostics) =
            group_fallback_panels(candidates, &|| Ok(true), "benchmark-game")
                .unwrap()
                .unwrap();
        eprintln!(
            "KeyViewer 100k fallback grouping: candidates={COUNT} panels={} missing_sentinels={} missing_keybinds={} ambiguous_hashes={} elapsed_ms={}",
            panels.len(),
            diagnostics.missing_sentinels,
            diagnostics.missing_keybinds,
            diagnostics.ambiguous_hashes,
            started.elapsed().as_millis(),
        );
        assert_eq!(panels.len(), COUNT);
    }

    #[test]
    fn authority_boundary_causes_force_a_fresh_snapshot() {
        assert!(OverlaySyncCause::FirstIndex.forces_publish());
        assert!(OverlaySyncCause::ModsRootChanged.forces_publish());
        assert!(OverlaySyncCause::ImporterRootChanged.forces_publish());
        assert!(OverlaySyncCause::Recovery.forces_publish());
        assert!(!OverlaySyncCause::EffectiveIniChanged.forces_publish());
    }

    #[test]
    fn newer_sync_revision_supersedes_queued_request() {
        let game_id = "keyviewer-revision-test";
        let older = request_sync_revision(game_id).unwrap();
        let newer = request_sync_revision(game_id).unwrap();

        assert!(!is_current_sync_revision(game_id, older).unwrap());
        assert!(is_current_sync_revision(game_id, newer).unwrap());
        assert!(!runtime_sync_request_is_current(game_id, Some(older), None).unwrap());
        assert!(runtime_sync_request_is_current(game_id, Some(newer), None).unwrap());
        assert!(matches!(
            superseded_sync_result(OverlaySyncCause::EffectiveIniChanged).publication,
            RuntimeSyncPublication::Skipped
        ));
    }

    #[test]
    fn latest_scoped_revision_keeps_changes_from_superseded_generations() {
        let game_id = "keyviewer-scoped-revision-merge";
        let first = reserve_overlay_sync_revision_for_request(
            game_id,
            RuntimeSyncRequest::Scoped {
                changes: vec![RuntimeModChange {
                    mod_id: "mod-a".to_string(),
                    folder_path: ModFolderPath::from_stored("Mod A"),
                    outcome: RuntimeModOutcome::Enabled,
                }],
            },
        )
        .unwrap();
        let latest = reserve_overlay_sync_revision_for_request(
            game_id,
            RuntimeSyncRequest::Scoped {
                changes: vec![RuntimeModChange {
                    mod_id: "mod-b".to_string(),
                    folder_path: ModFolderPath::from_stored("DISABLED Mod B"),
                    outcome: RuntimeModOutcome::Disabled,
                }],
            },
        )
        .unwrap();

        assert!(!is_current_sync_revision(game_id, first).unwrap());
        let RuntimeSyncRequest::Scoped { changes } =
            sync_request_for_revision(game_id, latest).unwrap().unwrap()
        else {
            panic!("latest request should remain scoped");
        };
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].mod_id, "mod-a");
        assert_eq!(changes[1].mod_id, "mod-b");
        settle_sync_request(game_id, latest).unwrap();
    }

    #[test]
    fn ambiguous_or_oversized_root_scope_requires_full_recovery() {
        let mixed = RuntimeSyncRequest::Scoped {
            changes: vec![RuntimeModChange {
                mod_id: "mod-a".to_string(),
                folder_path: ModFolderPath::from_stored("Parent/Mod A"),
                outcome: RuntimeModOutcome::Enabled,
            }],
        }
        .merge_pending(RuntimeSyncRequest::ScopedRoots {
            roots: vec![ModFolderPath::from_stored("Parent")],
        });
        assert_eq!(mixed, RuntimeSyncRequest::Full);

        let oversized = (0..=MAX_SCOPED_ROOTS)
            .map(|index| ModFolderPath::from_stored(format!("Root {index}")))
            .collect::<Vec<_>>();
        assert!(scoped_root_keys(Path::new("C:/Game/Mods"), &oversized).is_none());
        assert!(scoped_root_keys(
            Path::new("C:/Game/Mods"),
            &[ModFolderPath::from_stored("../Outside")]
        )
        .is_none());
    }

    #[test]
    fn superseded_sync_cannot_enter_the_publication_commit() {
        let game_id = "keyviewer-publication-authority-test";
        let older = request_sync_revision(game_id).unwrap();
        let newer = request_sync_revision(game_id).unwrap();
        let committed = std::sync::atomic::AtomicBool::new(false);

        let stale = commit_if_current_sync_revision(game_id, older, None, || {
            committed.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .unwrap();
        assert!(stale.is_none());
        assert!(!committed.load(std::sync::atomic::Ordering::SeqCst));

        let current = commit_if_current_sync_revision(game_id, newer, None, || {
            committed.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .unwrap();
        assert_eq!(current, Some(()));
        assert!(committed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn superseded_unchanged_result_cannot_replace_runtime_diagnostics() {
        let game_id = "keyviewer-stale-unchanged-diagnostics";
        let older = request_sync_revision(game_id).unwrap();
        let newer = request_sync_revision(game_id).unwrap();
        let stale_result = RuntimeSyncResult {
            cause: OverlaySyncCause::EffectiveModsChanged,
            publication: RuntimeSyncPublication::Unchanged,
            reload: RuntimeReloadOutcome::NotRequired,
            failure: None,
        };

        let recorded = commit_if_current_sync_revision(game_id, older, None, || {
            record_runtime_sync_result(game_id, &stale_result);
            Ok(())
        })
        .unwrap();

        assert!(recorded.is_none());
        let snapshots = KEYVIEWER_RUNTIME_SYNC_SNAPSHOTS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap();
        assert!(!snapshots.contains_key(game_id));
        drop(snapshots);
        settle_sync_request(game_id, newer).unwrap();
    }

    #[test]
    fn superseded_failure_cannot_replace_runtime_diagnostics_or_settle_current_work() {
        let game_id = "keyviewer-stale-failure-diagnostics";
        let older = request_sync_revision(game_id).unwrap();
        let newer = request_sync_revision(game_id).unwrap();
        let failed_result = RuntimeSyncResult {
            cause: OverlaySyncCause::EffectiveModsChanged,
            publication: RuntimeSyncPublication::FailedBeforePublish,
            reload: RuntimeReloadOutcome::NotRequired,
            failure: Some("stale failure".to_string()),
        };

        let recorded =
            record_runtime_sync_result_if_current(game_id, &failed_result, Some(older), None)
                .unwrap();

        assert!(!recorded);
        assert!(sync_request_for_revision(game_id, newer).unwrap().is_some());
        let snapshots = KEYVIEWER_RUNTIME_SYNC_SNAPSHOTS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap();
        assert!(!snapshots.contains_key(game_id));
        drop(snapshots);
        settle_sync_request(game_id, newer).unwrap();
    }

    #[test]
    fn current_failure_is_recorded_without_settling_retry_work() {
        let game_id = "keyviewer-current-failure-diagnostics";
        let revision = request_sync_revision(game_id).unwrap();
        let failed_result = RuntimeSyncResult {
            cause: OverlaySyncCause::EffectiveModsChanged,
            publication: RuntimeSyncPublication::FailedBeforePublish,
            reload: RuntimeReloadOutcome::NotRequired,
            failure: Some("current failure".to_string()),
        };

        let recorded =
            record_runtime_sync_result_if_current(game_id, &failed_result, Some(revision), None)
                .unwrap();

        assert!(recorded);
        assert!(sync_request_for_revision(game_id, revision)
            .unwrap()
            .is_some());
        settle_sync_request(game_id, revision).unwrap();
    }

    #[test]
    fn game_switch_revokes_an_older_games_publication_commit() {
        let _test_guard = crate::modules::reconciliation::application::disk_reconcile::orchestrator::activation_epoch_test_guard();
        let game_id = "keyviewer-activation-authority-test";
        let state = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState::new();
        state.begin_activation(Some(game_id.to_string()));
        let authority = state
            .activation_authority_for(game_id)
            .expect("active game authority");
        let revision = request_sync_revision(game_id).unwrap();
        state.begin_activation(Some("new-game".to_string()));
        let committed = std::sync::atomic::AtomicBool::new(false);

        let result = commit_if_current_sync_revision(game_id, revision, Some(&authority), || {
            committed.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .unwrap();

        assert!(result.is_none());
        assert!(!committed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn runtime_sync_only_retries_failed_publication_and_reports_manual_reload() {
        let failed = RuntimeSyncResult {
            cause: OverlaySyncCause::SettingsChanged,
            publication: RuntimeSyncPublication::FailedBeforePublish,
            reload: RuntimeReloadOutcome::NotRequired,
            failure: Some("generation failed".to_string()),
        };
        assert!(failed.requires_retry());
        assert_eq!(
            failed.diagnostic_message().as_deref(),
            Some("generation failed")
        );

        let manual = RuntimeSyncResult {
            cause: OverlaySyncCause::SettingsChanged,
            publication: RuntimeSyncPublication::Published,
            reload: RuntimeReloadOutcome::NeedsManualReload {
                binding: Some("F10".to_string()),
                reason: "active game is not focused".to_string(),
            },
            failure: None,
        };
        assert!(!manual.requires_retry());
        assert!(manual.needs_manual_reload());
        assert_eq!(
            manual.diagnostic_message().as_deref(),
            Some(
                "Manual reload required: focus the active game and press F10 to reload its configuration. active game is not focused"
            )
        );
    }

    #[test]
    fn runtime_diagnostics_explain_when_generation_cleanup_is_disabled() {
        let game_id = "runtime-diagnostics-without-executable";
        let settings = crate::modules::settings::application::config::AppSettings {
            games: vec![crate::modules::settings::application::config::GameConfig {
                id: game_id.to_string(),
                name: "GIMI".to_string(),
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                instance_path: PathBuf::from("C:/Game"),
                mod_path: PathBuf::from("C:/Game/Mods"),
                ready_to_move_path: None,
                launch_mode: Default::default(),
                game_exe: None,
                loader_exe: None,
                xxmi_launcher_exe: None,
                launch_args: None,
                warnings: Vec::new(),
            }],
            ..Default::default()
        };
        let result = RuntimeSyncResult {
            cause: OverlaySyncCause::Startup,
            publication: RuntimeSyncPublication::Unchanged,
            reload: RuntimeReloadOutcome::NotRequired,
            failure: None,
        };
        record_runtime_sync_result(game_id, &result);

        let diagnostics = keyviewer_runtime_diagnostics(&settings, game_id).unwrap();

        assert!(diagnostics.last_sync_unix_ms.is_some());
        assert_eq!(
            diagnostics.publication,
            Some(RuntimeSyncPublicationStatus::Unchanged)
        );
        assert_eq!(diagnostics.reload, Some(RuntimeReloadStatus::NotRequired));
        assert!(diagnostics.cleanup_automatic_disabled);
    }

    #[tokio::test]
    async fn collection_context_uses_the_newest_persisted_runtime_settings() {
        let old_mods = PathBuf::from("C:/Old/Mods");
        let new_mods = PathBuf::from("C:/New/Mods");
        let context = PostApplyContext {
            pool: SqlitePool::connect_lazy("sqlite::memory:").expect("test pool"),
            game_id: "game".to_string(),
            mods_path: old_mods,
            hotkeys: HotkeyConfig::default(),
            keyviewer_enabled: true,
            safe_mode: false,
            status_fields: Some(generator::StatusFields {
                preset_name: Some("Committed preset".to_string()),
                ..Default::default()
            }),
        };
        let settings = crate::modules::settings::application::config::AppSettings {
            games: vec![crate::modules::settings::application::config::GameConfig {
                id: "game".to_string(),
                name: "GIMI".to_string(),
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                instance_path: PathBuf::from("C:/New"),
                mod_path: new_mods.clone(),
                ready_to_move_path: None,
                launch_mode: Default::default(),
                game_exe: None,
                loader_exe: None,
                xxmi_launcher_exe: None,
                launch_args: None,
                warnings: Vec::new(),
            }],
            hotkeys: HotkeyConfig {
                toggle_overlay: "F8".to_string(),
                ..HotkeyConfig::default()
            },
            keyviewer: crate::modules::automation::application::hotkeys::KeyViewerConfig {
                enabled: false,
            },
            safety: crate::modules::settings::application::config::SafetyConfig {
                runtime_safe_mode_by_game: std::collections::BTreeMap::from([(
                    "game".to_string(),
                    true,
                )]),
                ..Default::default()
            },
            ..Default::default()
        };

        let refreshed = refresh_context_from_persisted_settings(context, &settings)
            .expect("persisted game should refresh the context");

        assert_eq!(refreshed.mods_path, new_mods);
        assert_eq!(refreshed.hotkeys.toggle_overlay, "F8");
        assert!(!refreshed.keyviewer_enabled);
        assert!(refreshed.safe_mode);
        assert_eq!(
            refreshed
                .status_fields
                .and_then(|fields| fields.preset_name)
                .as_deref(),
            Some("Committed preset")
        );
    }

    #[test]
    fn manifest_requires_the_entrypoint_to_reference_its_generation() {
        let temp = TempDir::new().unwrap();
        let emmm_data = temp.path().join(".emmm_data");
        let generation_id = format!("g{}", "a".repeat(64));
        let generation = emmm_data.join(KEYVIEWER_RESOURCE_ROOT).join(&generation_id);
        std::fs::create_dir_all(generation.join("status")).unwrap();
        let entrypoint = emmm_data.join("KeyViewer.ini");
        std::fs::write(
            &entrypoint,
            format!("; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1\nfilename = generations/{generation_id}/status/runtime_status.txt\n"),
        )
        .unwrap();
        let manifest = KeyViewerManifest {
            version: KEYVIEWER_MANIFEST_VERSION,
            fingerprint: "abc".to_string(),
            generation_id,
        };
        std::fs::write(
            manifest_path(&emmm_data),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        assert!(published_artifact_matches(&emmm_data, &entrypoint, "abc"));
        assert!(!published_artifact_matches(
            &emmm_data,
            &entrypoint,
            "different"
        ));
    }

    #[test]
    fn generation_cleanup_keeps_only_the_manifest_generation() {
        let temp = TempDir::new().unwrap();
        let emmm_data = temp.path().join(".emmm_data");
        let generations = emmm_data.join("generations");
        let active_generation = format!("g{}", "a".repeat(64));
        std::fs::create_dir_all(generations.join(&active_generation).join("status")).unwrap();
        std::fs::create_dir_all(generations.join("g1726262400000-1234-0")).unwrap();
        std::fs::create_dir_all(emmm_data.join("generations.recover.123")).unwrap();
        std::fs::create_dir_all(emmm_data.join("generations.staging.456")).unwrap();
        std::fs::create_dir_all(emmm_data.join("keybinds")).unwrap();
        std::fs::create_dir_all(emmm_data.join("status")).unwrap();
        std::fs::write(
            manifest_path(&emmm_data),
            serde_json::to_string(&KeyViewerManifest {
                version: KEYVIEWER_MANIFEST_VERSION,
                fingerprint: "abc".to_string(),
                generation_id: active_generation.clone(),
            })
            .unwrap(),
        )
        .unwrap();

        cleanup_generation_siblings(&emmm_data).unwrap();

        assert!(generations.join(active_generation).join("status").is_dir());
        assert!(!generations.join("g1726262400000-1234-0").exists());
        assert!(!emmm_data.join("generations.recover.123").exists());
        assert!(!emmm_data.join("generations.staging.456").exists());
        assert!(!emmm_data.join("keybinds").exists());
        assert!(!emmm_data.join("status").exists());
    }

    #[test]
    fn generation_cleanup_is_safe_after_a_confirmed_reload() {
        assert!(should_cleanup_generations(
            &RuntimeReloadOutcome::ReloadSent {
                binding: "F10".to_string()
            },
            false
        ));
        assert!(should_cleanup_generations(
            &RuntimeReloadOutcome::NotRequired,
            true
        ));
        assert!(!should_cleanup_generations(
            &RuntimeReloadOutcome::NeedsManualReload {
                binding: Some("F10".to_string()),
                reason: "game is not focused".to_string(),
            },
            false
        ));
    }

    #[tokio::test]
    async fn missing_mods_root_is_not_created_by_post_apply() {
        let ctx = crate::test_utils::init_test_db().await;
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing-mods");

        let result = run_post_apply_tasks(PostApplyContext {
            pool: ctx.pool,
            game_id: "missing-game".to_string(),
            mods_path: missing.clone(),
            hotkeys: HotkeyConfig::default(),
            keyviewer_enabled: true,
            safe_mode: false,
            status_fields: None,
        })
        .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
        assert!(!missing.exists());
    }

    #[tokio::test]
    async fn missing_catalog_publishes_a_status_only_overlay() {
        let ctx = crate::test_utils::init_test_db().await;
        let pool = ctx.pool;
        let temp = TempDir::new().unwrap();
        let mods = temp.path().join("Mods");
        std::fs::create_dir_all(&mods).unwrap();
        let legacy_entrypoint = mods.join(".emmm_data").join("KeyViewer.ini");
        let legacy_content = "; =================================================================\n;   KeyViewer.ini — Auto-generated by EMMM\n;   Do not edit manually.\n; =================================================================\n\n[KeyEMMM_Toggle]\nkey = F7\ntype = cycle\n$kv_active = 0, 1\n\n[CommandList_EMM_Render]\nif $kv_active == 1\nendif\n";
        std::fs::create_dir_all(legacy_entrypoint.parent().unwrap()).unwrap();
        std::fs::write(&legacy_entrypoint, legacy_content).unwrap();
        let game_path = temp.path().to_string_lossy().into_owned();
        let mods_path = mods.to_string_lossy().into_owned();
        crate::test_utils::insert_test_game(
            &pool,
            &crate::test_utils::TestGameFixture {
                id: "game-status-only",
                name: "GIMI",
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                path: &game_path,
                mods_path: Some(&mods_path),
            },
        )
        .await
        .unwrap();

        run_post_apply_tasks(PostApplyContext {
            pool: pool.clone(),
            game_id: "game-status-only".to_string(),
            mods_path: mods.clone(),
            hotkeys: HotkeyConfig::default(),
            keyviewer_enabled: true,
            safe_mode: false,
            status_fields: None,
        })
        .await
        .unwrap();

        let entrypoint =
            std::fs::read_to_string(mods.join(".emmm_data").join("KeyViewer.ini")).unwrap();
        assert!(entrypoint.contains("ResourceEMMM_Status"));
        assert!(!entrypoint.contains("[TextureOverride_EMMMv1_"));
        assert!(entrypoint.contains("generations/"));
        assert_eq!(
            std::fs::read_to_string(
                mods.join(".emmm_data")
                    .join("KeyViewer.ini.emmm-legacy-backup"),
            )
            .unwrap(),
            legacy_content
        );

        run_post_apply_tasks(PostApplyContext {
            pool,
            game_id: "game-status-only".to_string(),
            mods_path: mods.clone(),
            hotkeys: HotkeyConfig::default(),
            keyviewer_enabled: true,
            safe_mode: false,
            status_fields: None,
        })
        .await
        .unwrap();
        let generations = mods.join(".emmm_data").join("generations");
        let published_generation = published_generation_dir(&mods);
        assert!(published_generation.join("status").is_dir());
        let nested_generation_dirs = std::fs::read_dir(&generations)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .filter(|entry| entry.file_name().to_string_lossy().starts_with('g'))
            .count();
        assert_eq!(nested_generation_dirs, 1);
    }

    #[tokio::test]
    async fn missing_catalog_generates_a_hash_based_mod_panel() {
        let ctx = crate::test_utils::init_test_db().await;
        let temp = TempDir::new().unwrap();
        let mods = temp.path().join("Mods");
        let core = temp.path().join("Core").join("GIMI");
        let mod_dir = mods.join("Unclassified Arlecchino");
        std::fs::create_dir_all(&core).unwrap();
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(
            temp.path().join("d3dx.ini"),
            "[Include]\ninclude_recursive = Mods\ninclude = Core/GIMI/main.ini\n",
        )
        .unwrap();
        std::fs::write(
            core.join("main.ini"),
            "namespace = GIMIv8\nResourceText = null\nResourceTextParams = null\n[CommandListPrintText]\nchecktextureoverride = vb0\n",
        )
        .unwrap();
        std::fs::write(
            mod_dir.join("Arlecchino.ini"),
            "[TextureOverrideArlecchinoPosition]\nhash = A1B2C3D4\n\n[KeyArlecchino]\nkey = CTRL+[\nback = NO_CTRL+ALT+]\n",
        )
        .unwrap();
        let game_path = temp.path().to_string_lossy().into_owned();
        let mods_path = mods.to_string_lossy().into_owned();
        crate::test_utils::insert_test_game(
            &ctx.pool,
            &crate::test_utils::TestGameFixture {
                id: "game-mod-fallback",
                name: "GIMI",
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                path: &game_path,
                mods_path: Some(&mods_path),
            },
        )
        .await
        .unwrap();
        crate::test_utils::insert_test_mod(
            &ctx.pool,
            &crate::test_utils::TestModFixture {
                id: "unclassified-mod-fallback",
                game_id: "game-mod-fallback",
                object_id: None,
                actual_name: "Unclassified Arlecchino",
                folder_path: "Unclassified Arlecchino",
                status: crate::modules::games::domain::models::ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Other"),
                mods_path: Some(&mods_path),
            },
        )
        .await
        .unwrap();

        run_post_apply_tasks(PostApplyContext {
            pool: ctx.pool,
            game_id: "game-mod-fallback".to_string(),
            mods_path: mods.clone(),
            hotkeys: HotkeyConfig::default(),
            keyviewer_enabled: true,
            safe_mode: false,
            status_fields: None,
        })
        .await
        .expect("a catalog is not required for the hash-based mod fallback");

        let entrypoint =
            std::fs::read_to_string(mods.join(".emmm_data").join("KeyViewer.ini")).unwrap();
        assert!(entrypoint.contains("hash = a1b2c3d4"));
        assert!(entrypoint.contains("[TextureOverride_EMMMv1_Unclassified_Arlecchino"));

        let generation = published_generation_dir(&mods);
        let keybind_text = std::fs::read_to_string(
            generation
                .join("keybinds")
                .join("active")
                .join("character_000.txt"),
        )
        .unwrap();
        assert!(keybind_text.contains("Unclassified Arlecchino"));
        assert!(keybind_text.contains("Key: CTRL+["));
        assert!(keybind_text.contains("Back: NO_CTRL+ALT+]"));
        assert!(keybind_text.contains("[F7] Toggle Overlay"));
    }

    #[tokio::test]
    async fn unclassified_enabled_mod_generates_keyviewer_from_catalog_and_ini() {
        let temp = TempDir::new().unwrap();
        let pool = file_backed_test_pool(&temp.path().join("app.db")).await;
        let importer = temp.path().join("importer");
        let mods = importer.join("Mods");
        let core = importer.join("Core").join("GIMI");
        let mod_dir = mods.join("Unclassified Arlecchino");
        std::fs::create_dir_all(&core).unwrap();
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(
            importer.join("d3dx.ini"),
            "[Include]\ninclude_recursive = Mods\ninclude = Core/GIMI/main.ini\n",
        )
        .unwrap();
        std::fs::write(
            core.join("main.ini"),
            "namespace = GIMIv8\nResourceText = null\nResourceTextParams = null\n[CommandListPrintText]\nchecktextureoverride = vb0\n",
        )
        .unwrap();
        std::fs::write(
            mod_dir.join("Arlecchino.ini"),
            "[TextureOverrideArlecchinoPosition]\nhash = A1B2C3D4\n\n[KeyArlecchino]\nkey = CTRL+[\nback = NO_CTRL+ALT+]\ntype = toggle\n",
        )
        .unwrap();
        let importer_path = importer.to_string_lossy().into_owned();
        let mods_path = mods.to_string_lossy().into_owned();
        crate::test_utils::insert_test_game(
            &pool,
            &crate::test_utils::TestGameFixture {
                id: "unclassified-keyviewer",
                name: "GIMI",
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                path: &importer_path,
                mods_path: Some(&mods_path),
            },
        )
        .await
        .unwrap();
        crate::test_utils::insert_test_mod(
            &pool,
            &crate::test_utils::TestModFixture {
                id: "unclassified-mod",
                game_id: "unclassified-keyviewer",
                object_id: None,
                actual_name: "Unclassified Arlecchino",
                folder_path: "Unclassified Arlecchino",
                status: crate::modules::games::domain::models::ItemStatus::Enabled,
                is_safe: true,
                object_type: Some("Other"),
                mods_path: Some(&mods_path),
            },
        )
        .await
        .unwrap();
        install_gimi_runtime_catalog(temp.path());
        let hotkeys = HotkeyConfig {
            safe_mode: "F8".to_string(),
            prev_preset: "Shift+F8".to_string(),
            next_preset: "Ctrl+F8".to_string(),
            toggle_overlay: "F9".to_string(),
            ..HotkeyConfig::default()
        };

        run_post_apply_tasks(PostApplyContext {
            pool: pool.clone(),
            game_id: "unclassified-keyviewer".to_string(),
            mods_path: mods.clone(),
            hotkeys,
            keyviewer_enabled: true,
            safe_mode: false,
            status_fields: None,
        })
        .await
        .expect("unclassified enabled mod must still generate a KeyViewer observer");

        let entrypoint =
            std::fs::read_to_string(mods.join(".emmm_data").join("KeyViewer.ini")).unwrap();
        assert!(entrypoint.contains("[TextureOverride_EMMMv1_Arlecchino_0_S0]"));
        assert!(!entrypoint.contains("[TextureOverride_EMMMv1_Unclassified_Arlecchino"));
        assert!(entrypoint.contains("hash = a1b2c3d4"));
        assert!(entrypoint.contains("key = F9"));
        assert!(entrypoint.contains("type = cycle"));

        let generation = published_generation_dir(&mods);
        let keybind_text = std::fs::read_to_string(
            generation
                .join("keybinds")
                .join("active")
                .join("character_000.txt"),
        )
        .unwrap();
        assert!(keybind_text.contains("Arlecchino"));
        assert!(keybind_text.contains("Back: NO_CTRL+ALT+]"));
        assert!(keybind_text.contains("Toggle: CTRL+["));
        assert!(keybind_text.contains("[F9] Toggle Overlay"));
        let status_text =
            std::fs::read_to_string(generation.join("status").join("runtime_status.txt")).unwrap();
        assert_eq!(
            status_text,
            "Safe: Off [F8] | Preset: None [SHIFT+F8] [CTRL+F8]"
        );

        pool.close().await;
    }

    #[test]
    fn disabling_only_touches_a_verified_emmm_entrypoint() {
        let temp = TempDir::new().unwrap();
        let emmm_data = temp.path().join(".emmm_data");
        std::fs::create_dir_all(&emmm_data).unwrap();
        let entrypoint = emmm_data.join("KeyViewer.ini");
        std::fs::write(
            &entrypoint,
            "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1",
        )
        .unwrap();

        disable_owned_entrypoint(&emmm_data).unwrap();
        assert!(!entrypoint.exists());
        assert!(emmm_data.join("KeyViewer.ini.emmm-disabled").exists());

        std::fs::write(&entrypoint, "; generated by EMMM").unwrap();
        assert!(disable_owned_entrypoint(&emmm_data).is_err());
        assert!(entrypoint.exists());
    }

    #[test]
    fn legacy_migration_requires_the_generated_keyviewer_structure() {
        assert!(is_migratable_legacy_entrypoint(
            b"; KeyViewer.ini - Auto-generated by EMMM\n[KeyEMMM_Toggle]\n[CommandList_EMM_Render]"
        ));
        assert!(!is_migratable_legacy_entrypoint(
            b"; KeyViewer.ini - Auto-generated by EMMM\n[KeyEMMM_Toggle]"
        ));
        assert!(!is_migratable_legacy_entrypoint(
            b"[CommandList_EMM_Render]"
        ));
    }

    #[test]
    fn duplicate_scan_only_returns_emmm_owned_entrypoints() {
        let temp = TempDir::new().unwrap();
        let mods = temp.path().join("Mods");
        let expected = mods.join(".emmm_data").join("KeyViewer.ini");
        std::fs::create_dir_all(expected.parent().unwrap()).unwrap();
        std::fs::write(&expected, "; KeyViewer.ini — generated by EMMM").unwrap();

        let user_sample = mods.join("_KeyViewer").join("KeyViewer.ini");
        std::fs::create_dir_all(user_sample.parent().unwrap()).unwrap();
        std::fs::write(&user_sample, "; user-owned sample").unwrap();
        assert!(duplicate_keyviewer_entrypoints(std::slice::from_ref(&mods), &mods).is_empty());

        let duplicate = mods.join("Legacy").join("KeyViewer.ini");
        std::fs::create_dir_all(duplicate.parent().unwrap()).unwrap();
        std::fs::write(&duplicate, "; KeyViewer.ini — generated by EMMM").unwrap();
        std::fs::write(
            &duplicate,
            "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1",
        )
        .unwrap();
        assert_eq!(
            duplicate_keyviewer_entrypoints(std::slice::from_ref(&mods), &mods),
            vec![duplicate]
        );
    }

    #[test]
    fn duplicate_scan_covers_every_effective_include_root() {
        let temp = TempDir::new().unwrap();
        let primary = temp.path().join("Mods");
        let secondary = temp.path().join("SharedMods");
        let duplicate = secondary.join(".emmm_data").join("KeyViewer.ini");
        std::fs::create_dir_all(&primary).unwrap();
        std::fs::create_dir_all(duplicate.parent().unwrap()).unwrap();
        std::fs::write(
            &duplicate,
            "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1",
        )
        .unwrap();

        assert_eq!(
            duplicate_keyviewer_entrypoints(&[primary.clone(), secondary], &primary),
            vec![duplicate]
        );
    }

    #[test]
    fn duplicate_migration_keeps_a_rollback_backup() {
        let temp = TempDir::new().unwrap();
        let mods = temp.path().join("Mods");
        let emmm_data = mods.join(".emmm_data");
        let duplicate = mods.join("Legacy").join("KeyViewer.ini");
        std::fs::create_dir_all(duplicate.parent().unwrap()).unwrap();
        std::fs::write(&duplicate, "; KeyViewer.ini — generated by EMMM").unwrap();

        std::fs::write(
            &duplicate,
            "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1",
        )
        .unwrap();
        let migrated = migrate_duplicate_keyviewer_entrypoints(
            std::slice::from_ref(&mods),
            &mods,
            &emmm_data,
            "g100-1-0",
        )
        .unwrap();
        assert!(!duplicate.exists());
        assert_eq!(migrated.len(), 1);
        assert!(migrated[0].backup.exists());
        assert!(duplicate_migration_journal_path(&emmm_data).exists());

        rollback_duplicate_keyviewer_migrations(&migrated);
        clear_duplicate_migration_journal(&emmm_data).unwrap();
        assert!(duplicate.exists());
    }

    #[test]
    fn incomplete_duplicate_migration_restores_legacy_entrypoints() {
        let temp = TempDir::new().unwrap();
        let mods = temp.path().join("Mods");
        let emmm_data = mods.join(".emmm_data");
        let entrypoint = emmm_data.join("KeyViewer.ini");
        let duplicate = mods.join("Legacy").join("KeyViewer.ini");
        std::fs::create_dir_all(duplicate.parent().unwrap()).unwrap();
        std::fs::write(
            &duplicate,
            "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1",
        )
        .unwrap();

        let migrated = migrate_duplicate_keyviewer_entrypoints(
            std::slice::from_ref(&mods),
            &mods,
            &emmm_data,
            "g100-1-0",
        )
        .unwrap();
        assert!(!duplicate.exists());

        recover_duplicate_migration(&emmm_data, &entrypoint).unwrap();

        assert!(duplicate.exists());
        assert!(!duplicate_migration_journal_path(&emmm_data).exists());
        assert!(!migrated[0].backup.exists());
    }

    #[test]
    fn committed_duplicate_migration_keeps_legacy_backups_after_recovery() {
        let temp = TempDir::new().unwrap();
        let mods = temp.path().join("Mods");
        let emmm_data = mods.join(".emmm_data");
        let entrypoint = emmm_data.join("KeyViewer.ini");
        let duplicate = mods.join("Legacy").join("KeyViewer.ini");
        std::fs::create_dir_all(duplicate.parent().unwrap()).unwrap();
        std::fs::write(
            &duplicate,
            "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1",
        )
        .unwrap();

        let migrated = migrate_duplicate_keyviewer_entrypoints(
            std::slice::from_ref(&mods),
            &mods,
            &emmm_data,
            "g100-1-0",
        )
        .unwrap();
        std::fs::create_dir_all(
            emmm_data
                .join(KEYVIEWER_RESOURCE_ROOT)
                .join("g100-1-0")
                .join("status"),
        )
        .unwrap();
        std::fs::write(
            &entrypoint,
            "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1\nfilename = generations/g100-1-0/status/runtime_status.txt",
        )
        .unwrap();

        recover_duplicate_migration(&emmm_data, &entrypoint).unwrap();

        assert!(!duplicate.exists());
        assert!(migrated[0].backup.exists());
        assert!(!duplicate_migration_journal_path(&emmm_data).exists());
    }

    #[test]
    fn multi_character_mod_requires_explicit_key_section_ownership() {
        assert!(section_mentions_object("KeyArlecchino", "Arlecchino"));
        assert!(section_mentions_object(
            "Key_Arlecchino_Skill",
            "Arlecchino"
        ));
        assert!(!section_mentions_object("KeyGeneric", "Arlecchino"));
        assert!(!section_mentions_object("KeyAmber", "Arlecchino"));
    }

    #[test]
    fn preflight_uses_the_effective_include_root_and_installed_capabilities() {
        let temp = TempDir::new().unwrap();
        let mods = temp.path().join("Mods");
        let core = temp.path().join("Core").join("GIMI");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::create_dir_all(&core).unwrap();
        std::fs::write(
            temp.path().join("d3dx.ini"),
            "[Include]\ninclude_recursive = Mods\ninclude = Core/GIMI/main.ini\n",
        )
        .unwrap();
        std::fs::write(
            core.join("main.ini"),
            "namespace = GIMIv8\nResourceText = null\nResourceTextParams = null\n[CommandListPrintText]\nchecktextureoverride = vb0\nchecktextureoverride = ps-t1\n",
        )
        .unwrap();

        let preflight = read_runtime_preflight(
            temp.path(),
            &mods,
            crate::modules::games::domain::models::GameType::GIMI,
        )
        .unwrap();

        assert_eq!(preflight.runtime_mods_root, canonical_or_original(&mods));
        assert!(preflight.renderer_available);
        assert!(preflight.callback_slots.contains("vb0"));
        assert!(preflight.callback_slots.contains("ps-t1"));
    }

    #[test]
    fn preflight_cache_invalidates_when_an_importer_ini_snapshot_changes() {
        let temp = TempDir::new().unwrap();
        let mods = temp.path().join("Mods");
        let core = temp.path().join("Core");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::create_dir_all(&core).unwrap();
        std::fs::write(
            temp.path().join("d3dx.ini"),
            "[Include]\ninclude_recursive = Mods\n",
        )
        .unwrap();
        let renderer = core.join("renderer.ini");
        std::fs::write(
            &renderer,
            "namespace = GIMIv8\nResourceText = null\nResourceTextParams = null\n[CommandListPrintText]\nchecktextureoverride = vb0\n",
        )
        .unwrap();

        let first = read_runtime_preflight(
            temp.path(),
            &mods,
            crate::modules::games::domain::models::GameType::GIMI,
        )
        .unwrap();
        let second = read_runtime_preflight(
            temp.path(),
            &mods,
            crate::modules::games::domain::models::GameType::GIMI,
        )
        .unwrap();
        assert_eq!(first, second);

        std::fs::write(
            renderer,
            "namespace = GIMIv8\nResourceText = null\nResourceTextParams = null\n[CommandListPrintText]\nchecktextureoverride = ib\nchecktextureoverride = ps-t1\n",
        )
        .unwrap();
        let changed = read_runtime_preflight(
            temp.path(),
            &mods,
            crate::modules::games::domain::models::GameType::GIMI,
        )
        .unwrap();
        assert!(!changed.callback_slots.contains("vb0"));
        assert!(changed.callback_slots.contains("ib"));
        assert!(changed.callback_slots.contains("ps-t1"));
    }

    #[test]
    fn preflight_rejects_a_configured_root_outside_include_recursive() {
        let temp = TempDir::new().unwrap();
        let mods = temp.path().join("Mods");
        let wrong_mods = temp.path().join("OtherMods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::create_dir_all(&wrong_mods).unwrap();
        std::fs::write(
            temp.path().join("d3dx.ini"),
            "[Include]\ninclude_recursive = Mods\n",
        )
        .unwrap();

        let error = read_runtime_preflight(
            temp.path(),
            &wrong_mods,
            crate::modules::games::domain::models::GameType::GIMI,
        )
        .unwrap_err();
        assert!(error.to_string().contains("RuntimeModsRootMismatch"));
    }

    #[test]
    fn target_requires_an_active_callback_for_its_resource_slot() {
        let preflight = RuntimePreflight {
            runtime_mods_root: PathBuf::from("Mods"),
            runtime_include_roots: vec![PathBuf::from("Mods")],
            text_namespace: "GIMIv8".to_string(),
            renderer_available: true,
            callback_slots: HashSet::from(["vb0".to_string()]),
            diagnostics: Vec::new(),
        };
        let position_target = RuntimeTarget {
            variant: "Default".to_string(),
            component: None,
            resource_kind: RuntimeResourceKind::PositionVb,
            hash: "1234abcd".to_string(),
            slot: None,
            match_first_index: None,
            provenance: crate::modules::matching::application::deep_matcher::models::types::RuntimeTargetProvenance {
                source_repo: "fixture".to_string(),
                commit: "abc".to_string(),
                path: "hash.json".to_string(),
            },
        };
        let texture_target = RuntimeTarget {
            resource_kind: RuntimeResourceKind::Texture,
            slot: Some("ps-t1".to_string()),
            ..position_target.clone()
        };

        assert!(target_is_supported_by_runtime(&position_target, &preflight));
        assert!(!target_is_supported_by_runtime(&texture_target, &preflight));
    }
}
