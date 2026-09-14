use crate::modules::automation::application::hotkeys::HotkeyConfig;
use crate::modules::automation::application::keyviewer::generator;
use crate::modules::automation::application::keyviewer::harvester;
use crate::modules::automation::application::keyviewer::matcher;
use crate::modules::matching::application::deep_matcher::models::types::{
    RuntimeResourceKind, RuntimeTarget,
};
use crate::modules::system::domain::mod_path::ModFolderPath;
use crate::shared::errors::AppError;
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use walkdir::WalkDir;

static KEYVIEWER_GENERATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static KEYVIEWER_SYNC_LOCKS: OnceLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    OnceLock::new();
static KEYVIEWER_SYNC_REVISIONS: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
static KEYVIEWER_RUNTIME_SYNC_SNAPSHOTS: OnceLock<Mutex<HashMap<String, RuntimeSyncSnapshot>>> =
    OnceLock::new();
const KNOWN_LEGACY_KEYVIEWER_SHA256: &str =
    "8741313bbaaae887483c7b3b6a5e7777d23145754f762efc5beaaa6a636c5579";

const KEYVIEWER_MANIFEST_FILE: &str = "manifest.json";
const KEYVIEWER_MIGRATION_JOURNAL_FILE: &str = "duplicate-migration.json";

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
    /// request does not: its files are already published and replaying input is
    /// not safe while the game is unfocused.
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
                    || "focus the active game and reload its 3DMigoto configuration".to_string(),
                    |key| {
                        format!("focus the active game and press {key} to reload its configuration")
                    },
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
    generation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PostApplyPublication {
    Published,
    Unchanged,
    Skipped,
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

fn read_runtime_preflight(
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
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| !path_is_within(entry.path(), &configured_mods_root))
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ini"))
        })
    {
        inspected_files += 1;
        if inspected_files > 1_000 {
            return Err(AppError::Validation(
                "RuntimeCapabilitiesInvalid: importer configuration exceeds the INI scan limit"
                    .to_string(),
            ));
        }
        let Ok(content) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
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

fn next_generation_id() -> String {
    let sequence = KEYVIEWER_GENERATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let milliseconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("g{milliseconds}-{}-{sequence}", std::process::id())
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

fn request_sync_revision(game_id: &str) -> Result<u64, AppError> {
    let revisions = KEYVIEWER_SYNC_REVISIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut revisions = revisions.lock().map_err(|_| {
        AppError::Internal(
            "KeyViewer sync revision state was poisoned; restart the application".to_string(),
        )
    })?;
    let revision = revisions.entry(game_id.to_string()).or_default();
    *revision = revision.saturating_add(1);
    Ok(*revision)
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
        .is_some_and(|current| *current == revision))
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

fn published_artifact_matches(emmm_data_dir: &Path, entrypoint: &Path, fingerprint: &str) -> bool {
    let Some(manifest) = read_manifest(emmm_data_dir) else {
        return false;
    };
    if manifest.version != 1 || manifest.fingerprint != fingerprint {
        return false;
    }
    let Ok(content) = std::fs::read(entrypoint) else {
        return false;
    };
    if !is_current_emmm_entrypoint(&content) {
        return false;
    }
    let generation = emmm_data_dir
        .join("generations")
        .join(&manifest.generation_id);
    generation.is_dir()
        && String::from_utf8_lossy(&content)
            .contains(&format!("generations/{}/", manifest.generation_id))
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
    use sysinfo::System;

    let Some(executable_name) = settings
        .games
        .iter()
        .find(|game| game.id == game_id)
        .and_then(|game| game.game_exe.as_ref())
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().to_string())
    else {
        // Absence of an executable is not evidence that the game stopped.
        return false;
    };

    let mut system = System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    !system.processes().values().any(|process| {
        process
            .name()
            .to_string_lossy()
            .eq_ignore_ascii_case(&executable_name)
    })
}

fn is_owned_generation_id(name: &str) -> bool {
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

fn cleanup_obsolete_generations(emmm_data_dir: &Path) -> Result<(), AppError> {
    let Some(manifest) = read_manifest(emmm_data_dir) else {
        return Ok(());
    };
    let generations_dir = emmm_data_dir.join("generations");
    if !generations_dir.is_dir() {
        return Ok(());
    }
    let mut owned_generations = std::fs::read_dir(&generations_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            is_owned_generation_id(&name).then_some((name, entry.path()))
        })
        .collect::<Vec<_>>();
    owned_generations.sort_by(|left, right| right.0.cmp(&left.0));

    let mut retained_rollback = false;
    for (name, path) in owned_generations {
        if name == manifest.generation_id {
            continue;
        }
        // A generation newer than the current manifest was never published
        // (for example, an interrupted entrypoint swap), so it is not a
        // rollback candidate. Keep only the latest older snapshot.
        if name < manifest.generation_id && !retained_rollback {
            retained_rollback = true;
            continue;
        }
        std::fs::remove_dir_all(&path).map_err(|error| {
            AppError::Io(format!(
                "Could not remove obsolete KeyViewer generation {}: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

fn is_current_emmm_entrypoint(content: &[u8]) -> bool {
    let content = String::from_utf8_lossy(content).to_ascii_lowercase();
    content.contains("namespace = emmmv1") && content.contains("emmm-artifact: keyviewer v1")
}

fn is_known_legacy_entrypoint(content: &[u8]) -> bool {
    format!("{:x}", Sha256::digest(content)) == KNOWN_LEGACY_KEYVIEWER_SHA256
}

fn is_verified_emmm_entrypoint(content: &[u8]) -> bool {
    is_current_emmm_entrypoint(content) || is_known_legacy_entrypoint(content)
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
            content.contains(&format!("generations/{}/", journal.generation_id))
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
fn group_fallback_panels(
    mut candidates: Vec<FallbackCandidate>,
) -> (Vec<FallbackPanel>, Vec<&'static str>) {
    candidates.sort_by(|left, right| left.mod_path.cmp(&right.mod_path));
    let mut diagnostics = Vec::new();
    candidates.retain(|candidate| {
        let keep = !candidate.sentinels.is_empty() && !candidate.keybinds.is_empty();
        if !keep {
            diagnostics.push("NoEligibleFallbackSentinel");
        }
        keep
    });

    let mut ambiguous = HashSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if fallback_is_geometry(candidate) {
            continue;
        }
        if candidates.iter().enumerate().any(|(other_index, other)| {
            index != other_index
                && candidate.sentinels.iter().any(|sentinel| {
                    other
                        .sentinels
                        .iter()
                        .any(|other| same_sentinel(sentinel, other))
                })
        }) {
            ambiguous.insert(index);
        }
    }
    if !ambiguous.is_empty() {
        diagnostics.push("AmbiguousFallbackHash");
    }

    let mut parent: Vec<usize> = (0..candidates.len()).collect();
    for left in 0..candidates.len() {
        if !fallback_is_geometry(&candidates[left]) {
            continue;
        }
        for right in 0..left {
            if fallback_is_geometry(&candidates[right])
                && candidates[left].sentinels.iter().any(|sentinel| {
                    candidates[right]
                        .sentinels
                        .iter()
                        .any(|other| same_sentinel(sentinel, other))
                })
            {
                union_find_union(&mut parent, left, right);
            }
        }
    }

    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..candidates.len() {
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
    (panels, diagnostics)
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
    run_post_apply_tasks_with_options(ctx, false)
        .await
        .map(|_| ())
}

async fn run_post_apply_tasks_with_options(
    ctx: PostApplyContext,
    force_publish: bool,
) -> Result<PostApplyPublication, AppError> {
    let pool = &ctx.pool;
    let game_id = &ctx.game_id;
    log::info!(
        "[post_apply] Starting post-apply tasks for game={}",
        game_id
    );

    let game_type = crate::modules::games::adapters::sqlite::game::get_game_type(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} not found")))?;
    let importer_root = sqlx::query("SELECT path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(pool)
        .await?
        .map(|row| PathBuf::from(row.get::<String, _>("path")))
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} not found")))?;
    let runtime_preflight = read_runtime_preflight(&importer_root, &ctx.mods_path, game_type)?;
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

    // Runtime projection is refreshed by its own writer/reconcile path. The
    // overlay only needs the effective enabled-mod rows.
    let enabled_mods =
        crate::modules::library::adapters::sqlite::mods::get_enabled_mods_paths(pool, game_id)
            .await?;

    // 3. KeyViewer Pipeline (Req-43)
    let emmm_data_dir = mods_path.join(".emmm_data");
    if !ctx.keyviewer_enabled {
        let duplicates =
            duplicate_keyviewer_entrypoints(&runtime_preflight.runtime_include_roots, mods_path);
        if !duplicates.is_empty() {
            return Err(AppError::Validation(format!(
                "DuplicateOverlay: {} additional verified EMMM entrypoint(s) remain in the effective include tree",
                duplicates.len()
            )));
        }
        disable_owned_entrypoint(&emmm_data_dir)?;
        log::info!("[post_apply] KeyViewer entrypoint disabled for game={game_id}");
        return Ok(PostApplyPublication::Skipped);
    }

    // Harvest: one pass per mod for both hashes and keybinds.
    let mut occurrence_counts = HashMap::new();
    let mut hash_to_mod_path = HashMap::new();
    let mut mod_keybinds = HashMap::new();
    let mut mod_targets = HashMap::new();
    let mut effective_ini_fingerprints = Vec::new();
    let harvest_capabilities = harvester::HarvestCapabilities::from_callback_slots(
        runtime_preflight.callback_slots.iter(),
    );
    let active_mod_paths: Vec<_> = enabled_mods
        .iter()
        .map(|stored_path| stored_path.resolve(mods_path))
        .collect();
    harvester::retain_cached_mods(active_mod_paths.iter().map(PathBuf::as_path));

    for (stored_path, abs_path) in enabled_mods.into_iter().zip(active_mod_paths) {
        let harvest = harvester::harvest_mod(&abs_path, &harvest_capabilities)?;

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
        mod_targets.insert(stored_path.clone(), harvest.targets);
        mod_keybinds.insert(stored_path, harvest.keybinds);
    }

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
                        if let Some(kbs) = mod_keybinds.get(mp) {
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
    let fallback_candidates = mod_targets
        .iter()
        .filter(|(mod_path, _)| !catalog_matched_mods.contains(*mod_path))
        .map(|(mod_path, targets)| FallbackCandidate {
            mod_path: mod_path.clone(),
            sentinels: fallback_sentinels(targets),
            keybinds: mod_keybinds.get(mod_path).cloned().unwrap_or_default(),
        })
        .collect();
    let (fallback_panels, fallback_diagnostics) = group_fallback_panels(fallback_candidates);
    for diagnostic in fallback_diagnostics {
        log::warn!("[post_apply] {diagnostic}");
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

    let entrypoint = emmm_data_dir.join("KeyViewer.ini");
    recover_duplicate_migration(&emmm_data_dir, &entrypoint)?;
    if entrypoint.exists() && !is_verified_emmm_entrypoint(&std::fs::read(&entrypoint)?) {
        return Err(AppError::Validation(format!(
            "AmbiguousOverlayOwnership: {} was left unchanged",
            entrypoint.display()
        )));
    }

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
        log::info!(
            "[post_apply] KeyViewer inputs are unchanged for game={game_id}; retaining current generation"
        );
        return Ok(PostApplyPublication::Unchanged);
    }

    // Stage only text resources. No staged directory contains an INI, then
    // publish the single entrypoint last. This is safe with include_recursive:
    // the running game sees either the old entrypoint or one whose immutable
    // generation already exists.
    let generation_id = next_generation_id();
    let generations_dir = emmm_data_dir.join("generations");
    let final_generation = generations_dir.join(&generation_id);
    let staging_artifacts = generator::create_staging_directory(&generations_dir)?;
    let kv_ini_content = generator::generate_keyviewer_ini_for_resources(
        &matches,
        &ctx.hotkeys.toggle_overlay,
        game_type,
        &format!("generations/{generation_id}"),
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
    if let Err(error) = std::fs::create_dir_all(&generations_dir) {
        return Err(cleanup_staging_after_error(
            &staging_artifacts,
            error.into(),
        ));
    }
    if final_generation.exists() {
        return Err(cleanup_staging_after_error(
            &staging_artifacts,
            AppError::Validation(format!(
                "KeyViewer generation collision at {}; retry the operation",
                final_generation.display()
            )),
        ));
    }
    if let Err(error) = std::fs::rename(&staging_artifacts, &final_generation) {
        return Err(cleanup_staging_after_error(
            &staging_artifacts,
            error.into(),
        ));
    }

    let migrated_entrypoints = migrate_duplicate_keyviewer_entrypoints(
        &runtime_preflight.runtime_include_roots,
        mods_path,
        &emmm_data_dir,
        &generation_id,
    )?;
    if let Err(error) = generator::atomic_write(&entrypoint, &kv_ini_content) {
        rollback_duplicate_keyviewer_migrations(&migrated_entrypoints);
        let _ = clear_duplicate_migration_journal(&emmm_data_dir);
        return Err(error);
    }
    if let Err(error) = clear_duplicate_migration_journal(&emmm_data_dir) {
        // The entrypoint is already the durable publication point. A journal
        // left behind is safe: the next sync recognizes this generation and
        // clears it without restoring the migrated legacy files.
        log::warn!("KeyViewer duplicate migration journal cleanup was deferred: {error}");
    }

    let manifest = KeyViewerManifest {
        version: 1,
        fingerprint,
        generation_id,
    };
    let manifest_content = serde_json::to_string_pretty(&manifest).map_err(|error| {
        AppError::Internal(format!("Could not serialize KeyViewer manifest: {error}"))
    })?;
    if let Err(error) = generator::atomic_write(&manifest_path(&emmm_data_dir), &manifest_content) {
        // Manifest is a cache for equality/cleanup. Never misreport a
        // successful entrypoint publication as failed because cache refresh
        // could not be persisted.
        log::warn!("KeyViewer manifest refresh was deferred: {error}");
    }

    log::info!(
        "[post_apply] Completed post-apply tasks for game={}",
        game_id
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

/// The only entrypoint for externally-triggered KeyViewer synchronization.
/// A game-scoped async lock serializes requests. Watcher debouncing limits
/// bursts before they get here; the fingerprint makes any remaining duplicate
/// request a no-op rather than another artifact generation.
pub async fn request_overlay_sync_for_game(
    pool: &SqlitePool,
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    cause: OverlaySyncCause,
) -> Result<RuntimeSyncResult, AppError> {
    let revision = request_sync_revision(game_id)?;
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
    synchronize_overlay_context(context, cause, Some(settings)).await
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
    let context = refresh_context_from_persisted_settings(context, &settings)?;
    if !is_current_sync_revision(&game_id, revision)? {
        return Ok(superseded_sync_result(cause));
    }
    synchronize_overlay_context(context, cause, Some(settings)).await
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

async fn synchronize_overlay_context(
    context: PostApplyContext,
    cause: OverlaySyncCause,
    reload_settings: Option<crate::modules::settings::application::config::AppSettings>,
) -> Result<RuntimeSyncResult, AppError> {
    let game_id = context.game_id.clone();
    let emmm_data_dir = context.mods_path.join(".emmm_data");
    let publication = match run_post_apply_tasks_with_options(context, cause.forces_publish()).await
    {
        Ok(publication) => match publication {
            PostApplyPublication::Published => RuntimeSyncPublication::Published,
            PostApplyPublication::Unchanged => RuntimeSyncPublication::Unchanged,
            PostApplyPublication::Skipped => RuntimeSyncPublication::Skipped,
        },
        Err(error) => {
            let result = RuntimeSyncResult {
                cause,
                publication: RuntimeSyncPublication::FailedBeforePublish,
                reload: RuntimeReloadOutcome::NotRequired,
                failure: Some(error.to_string()),
            };
            record_runtime_sync_result(&game_id, &result);
            return Ok(result);
        }
    };

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
        match crate::modules::automation::application::hotkeys::reload::trigger_reload_config(
            settings,
        ) {
            Ok(binding) => RuntimeReloadOutcome::ReloadSent { binding },
            Err(error) => RuntimeReloadOutcome::NeedsManualReload {
                binding: crate::modules::automation::application::hotkeys::reload::configured_reload_config_binding(settings)
                    .ok(),
                reason: error.to_string(),
            },
        }
    } else {
        RuntimeReloadOutcome::NotRequired
    };

    if matches!(
        cause,
        OverlaySyncCause::Startup | OverlaySyncCause::Recovery
    ) && reload_settings
        .as_ref()
        .is_some_and(|settings| game_is_confirmed_stopped(settings, &game_id))
    {
        if let Err(error) = cleanup_obsolete_generations(&emmm_data_dir) {
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
    record_runtime_sync_result(&game_id, &result);
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
            "format_version": 2,
            "id": "fixture-catalog",
            "version": "2026.09.14",
            "author": "EMMM test",
            "source": "https://example.invalid/catalog",
            "license": "MIT",
            "catalogs": { "gimi": { "path": "gimi.json", "sha256": checksum } }
        });
        std::fs::write(
            pack_dir.join("manifest.json"),
            serde_json::to_vec(&manifest).expect("serialize fixture manifest"),
        )
        .expect("write fixture manifest");
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
        let (panels, diagnostics) = group_fallback_panels(vec![
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
        ]);

        assert!(diagnostics.is_empty());
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
        let (panels, diagnostics) = group_fallback_panels(vec![
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
        ]);

        assert!(panels.is_empty());
        assert_eq!(diagnostics, vec!["AmbiguousFallbackHash"]);
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
        assert!(matches!(
            superseded_sync_result(OverlaySyncCause::EffectiveIniChanged).publication,
            RuntimeSyncPublication::Skipped
        ));
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
        let generation = emmm_data.join("generations").join("g-test");
        std::fs::create_dir_all(&generation).unwrap();
        let entrypoint = emmm_data.join("KeyViewer.ini");
        std::fs::write(
            &entrypoint,
            "; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1\nfilename = generations/g-test/status/runtime_status.txt\n",
        )
        .unwrap();
        let manifest = KeyViewerManifest {
            version: 1,
            fingerprint: "abc".to_string(),
            generation_id: "g-test".to_string(),
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
    fn startup_cleanup_retains_current_and_one_rollback_generation() {
        let temp = TempDir::new().unwrap();
        let emmm_data = temp.path().join(".emmm_data");
        let generations = emmm_data.join("generations");
        for generation in ["g100-1-0", "g200-1-0", "g300-1-0"] {
            std::fs::create_dir_all(generations.join(generation)).unwrap();
        }
        std::fs::create_dir_all(generations.join("game-config")).unwrap();
        std::fs::write(
            manifest_path(&emmm_data),
            serde_json::to_string(&KeyViewerManifest {
                version: 1,
                fingerprint: "fixture".to_string(),
                generation_id: "g200-1-0".to_string(),
            })
            .unwrap(),
        )
        .unwrap();

        cleanup_obsolete_generations(&emmm_data).unwrap();

        assert!(generations.join("g200-1-0").is_dir());
        assert!(generations.join("g100-1-0").is_dir());
        assert!(!generations.join("g300-1-0").exists());
        assert!(generations.join("game-config").is_dir());
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
        let generations = std::fs::read_dir(mods.join(".emmm_data").join("generations"))
            .unwrap()
            .count();
        assert_eq!(
            generations, 1,
            "unchanged inputs must retain the generation"
        );
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

        let generation = std::fs::read_dir(mods.join(".emmm_data").join("generations"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
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

        let generation = std::fs::read_dir(mods.join(".emmm_data").join("generations"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
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
        std::fs::create_dir_all(emmm_data.join("generations").join("g100-1-0")).unwrap();
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
