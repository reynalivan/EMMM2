use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rayon::slice::ParallelSliceMut;
use serde::{Deserialize, Serialize};

use crate::modules::catalog::adapters::sqlite::object::get_runtime_descriptors_for_folder_path_keys;
use crate::modules::catalog::domain::objects::ObjectRuntimeDescriptor;
use crate::modules::workspace::application::explorer::types::ModFolder;
use crate::modules::workspace::application::workspace_read_model::explorer_mapper::map_workspace_node;
use crate::modules::workspace::domain::normalizer::{is_disabled_folder, normalize_display_name};
use crate::modules::workspace::domain::workspace::{
    ResolvedWorkspaceExplorerSelection, WorkspaceExplorerPage, WorkspaceExplorerPageInput,
    WorkspaceExplorerQuery, WorkspaceExplorerSafetyFilter, WorkspaceExplorerSelection,
    WorkspaceExplorerSelectionInput, WorkspaceExplorerSortField, WorkspaceExplorerSortOrder,
};
use crate::shared::errors::AppError;
use crate::shared::path_key::{canonical_name_key, folder_path_key, path_file_name_lossy};

use super::builder::build_mod_folder_from_path;
use super::grid::{resolve_listing_target, ResolvedListingTarget};
use super::scan::find_disabled_ancestor;

pub const DEFAULT_EXPLORER_PAGE_SIZE: u32 = 100;
pub const MAX_EXPLORER_PAGE_SIZE: u32 = 200;
const CURSOR_VERSION: u8 = 2;
const MAX_RESOLVED_SELECTION_PATHS: usize = 10_000;
const MAX_LISTING_SNAPSHOTS: usize = 8;
const MAX_CACHED_LISTING_CANDIDATES: usize = 500_000;

static LISTING_SNAPSHOT_CACHE: OnceLock<Mutex<ListingSnapshotCache>> = OnceLock::new();
static LISTING_SNAPSHOT_CLOCK: AtomicU64 = AtomicU64::new(0);
static LISTING_BUILD_TOKENS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
static LISTING_BUILD_PERMITS: OnceLock<tokio::sync::Semaphore> = OnceLock::new();

struct ListingBuildLease {
    scope: String,
    token: Arc<AtomicBool>,
}

impl Drop for ListingBuildLease {
    fn drop(&mut self) {
        self.token.store(false, AtomicOrdering::Release);
        if let Err(error) = finish_listing_build(&self.scope, &self.token) {
            log::warn!("failed to release explorer listing build lease: {error}");
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CandidateSortKey {
    is_favorite: bool,
    is_container: bool,
    name_key: String,
    modified_at: u64,
    size_bytes: u64,
    path_key: String,
}

#[derive(Debug, Clone)]
struct ListingCandidate {
    path: PathBuf,
    filesystem_identity: Option<String>,
    sort_key: CandidateSortKey,
    state: ChildListingState,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExplorerCursor {
    version: u8,
    query_fingerprint: String,
    snapshot_id: String,
    offset: usize,
}

#[derive(Debug)]
struct ListingSnapshot {
    query_fingerprint: String,
    target_key: String,
    candidates: Vec<ListingCandidate>,
    estimated_bytes: usize,
}

#[derive(Debug)]
struct CachedListingSnapshot {
    snapshot: Arc<ListingSnapshot>,
    last_used: u64,
}

#[derive(Debug, Default)]
struct ListingSnapshotCache {
    entries: HashMap<String, CachedListingSnapshot>,
    total_candidates: usize,
    total_estimated_bytes: usize,
}

#[derive(Debug, Clone, Default)]
struct ChildListingState {
    is_terminal: bool,
    is_favorite: bool,
    exact_is_safe: bool,
    exact_is_classified: bool,
    contains_safe: bool,
    contains_unsafe: bool,
}

#[derive(Debug, Clone, Default)]
struct PagedListingIndex {
    children: HashMap<String, ChildListingState>,
}

impl PagedListingIndex {
    fn state(&self, folder_key: &str) -> ChildListingState {
        self.children.get(folder_key).cloned().unwrap_or_default()
    }

    #[cfg(test)]
    fn from_states(states: Vec<(String, bool, bool, bool)>) -> Self {
        Self {
            children: states
                .into_iter()
                .map(|(key, is_favorite, contains_safe, contains_unsafe)| {
                    (
                        key,
                        ChildListingState {
                            is_terminal: true,
                            is_favorite,
                            exact_is_safe: contains_safe && !contains_unsafe,
                            exact_is_classified: contains_safe || contains_unsafe,
                            contains_safe,
                            contains_unsafe,
                        },
                    )
                })
                .collect(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ChildListingRow {
    child_key: String,
    is_terminal: i64,
    is_favorite: i64,
    exact_is_safe: i64,
    exact_is_classified: i64,
    contains_safe: i64,
    contains_unsafe: i64,
}

struct PageBuild {
    folders: Vec<ModFolder>,
    next_cursor: Option<String>,
    total_matching: u64,
    query_fingerprint: String,
    listing_revision: String,
    ancestor_disabled_by: Option<String>,
}

impl PageBuild {
    fn into_page(self) -> WorkspaceExplorerPage {
        WorkspaceExplorerPage {
            items: self
                .folders
                .into_iter()
                .map(|folder| map_workspace_node(folder, self.ancestor_disabled_by.as_deref()))
                .collect(),
            next_cursor: self.next_cursor,
            total_matching: self.total_matching,
            query_fingerprint: self.query_fingerprint,
            listing_revision: self.listing_revision,
        }
    }
}

pub async fn list_workspace_explorer_page(
    pool: &sqlx::SqlitePool,
    mods_path: String,
    input: WorkspaceExplorerPageInput,
) -> Result<WorkspaceExplorerPage, AppError> {
    let started_at = std::time::Instant::now();
    let resolved =
        resolve_target_blocking(&mods_path, input.query.explorer_sub_path.as_deref()).await?;
    let query_fingerprint = query_fingerprint(&input.query)?;
    let cursor = input
        .cursor
        .as_deref()
        .map(|value| decode_cursor(value, &query_fingerprint))
        .transpose()?;
    let snapshot_hit = cursor.is_some();
    let mut index_ms = 0_u128;
    let game_id = input.query.game_id.clone();
    let build_mods_path = mods_path.clone();
    let mut page = if let Some(cursor) = cursor {
        tokio::task::spawn_blocking(move || {
            build_cached_workspace_explorer_page(build_mods_path, input, resolved, cursor)
        })
        .await??
    } else {
        let current_key = folder_path_key(&resolved.target.to_string_lossy(), Some(&mods_path));
        let index_started_at = std::time::Instant::now();
        let index = load_paged_listing_index(pool, &input.query.game_id, &current_key).await?;
        index_ms = index_started_at.elapsed().as_millis();
        let build_scope = physical_path_key(&resolved.target);
        let build_lease = register_listing_build(&build_scope)?;
        let _build_permit = LISTING_BUILD_PERMITS
            .get_or_init(|| tokio::sync::Semaphore::new(2))
            .acquire()
            .await
            .map_err(|_| AppError::Internal("Explorer listing worker pool closed".to_string()))?;
        let worker_token = Arc::clone(&build_lease.token);
        let build_result = tokio::task::spawn_blocking(move || {
            build_first_workspace_explorer_page(
                build_mods_path,
                input,
                resolved,
                index,
                query_fingerprint,
                Some(worker_token),
            )
        })
        .await;
        build_result??
    };
    log::debug!(
        "workspace explorer page game_id={} snapshot_hit={} index_ms={} returned={} total_matching={} total_ms={}",
        game_id,
        snapshot_hit,
        index_ms,
        page.folders.len(),
        page.total_matching,
        started_at.elapsed().as_millis(),
    );
    enrich_page_owners(pool, &game_id, &mods_path, &mut page.folders).await?;
    Ok(page.into_page())
}

pub async fn resolve_workspace_explorer_selection(
    mods_path: String,
    input: WorkspaceExplorerSelectionInput,
) -> Result<ResolvedWorkspaceExplorerSelection, AppError> {
    let resolved =
        resolve_target_blocking(&mods_path, input.query.explorer_sub_path.as_deref()).await?;
    let snapshot = get_listing_snapshot(&input.listing_revision)?;
    let query_fingerprint = query_fingerprint(&input.query)?;
    validate_listing_snapshot(&snapshot, &resolved.target, &query_fingerprint)?;
    tokio::task::spawn_blocking(move || {
        resolve_workspace_explorer_selection_blocking(input, snapshot)
    })
    .await?
}

/// Lightweight navigation context for the structure read model. Explorer
/// children are delivered exclusively by the paged endpoint; this keeps the
/// structure response bounded even when the current directory has 100k
/// folders.
pub async fn load_workspace_explorer_context(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &str,
    explorer_sub_path: Option<&str>,
) -> Result<crate::modules::workspace::application::explorer::types::FolderGridResponse, AppError> {
    let resolved = resolve_target_blocking(mods_path, explorer_sub_path).await?;
    let target_path = resolved.target.to_string_lossy().into_owned();
    let exact_mod =
        !crate::modules::library::adapters::sqlite::mods::get_exact_runtime_mods_for_paths(
            pool,
            game_id,
            Path::new(mods_path),
            std::slice::from_ref(&target_path),
        )
        .await?
        .is_empty();
    let mods_path_owned = mods_path.to_string();
    let sub_path_owned = explorer_sub_path.map(str::to_string);
    let mut response = tokio::task::spawn_blocking(move || {
        build_workspace_explorer_context(
            &mods_path_owned,
            sub_path_owned.as_deref(),
            resolved,
            exact_mod,
        )
    })
    .await??;

    if let Some(sub_path) = explorer_sub_path.filter(|path| !path.is_empty()) {
        let self_path = Path::new(mods_path).join(sub_path);
        let mut key = folder_path_key(&self_path.to_string_lossy(), Some(mods_path));
        let mut keys = Vec::new();
        while !key.is_empty() {
            keys.push(key.clone());
            let Some((parent, _)) = key.rsplit_once('/') else {
                break;
            };
            key.truncate(parent.len());
        }
        let owners = get_runtime_descriptors_for_folder_path_keys(pool, game_id, &keys).await?;
        if let Some(owner) = owners
            .into_iter()
            .max_by_key(|owner| owner.folder_path_key.len())
        {
            response.self_owner_object_id = Some(owner.id);
            response.self_owner_object_folder_path = Some(owner.folder_path);
        }
    }
    Ok(response)
}

fn build_workspace_explorer_context(
    mods_path: &str,
    explorer_sub_path: Option<&str>,
    resolved: ResolvedListingTarget,
    exact_mod: bool,
) -> Result<crate::modules::workspace::application::explorer::types::FolderGridResponse, AppError> {
    let is_root = explorer_sub_path.is_none_or(str::is_empty);
    let (node_type, classification_reasons) = if is_root || !exact_mod {
        (
            crate::modules::workspace::domain::classifier::NodeType::ContainerFolder,
            Vec::new(),
        )
    } else {
        let (node_type, reasons, _) =
            crate::modules::workspace::domain::classifier::classify_folder(&resolved.target);
        (node_type, reasons)
    };
    let self_is_mod = matches!(
        node_type,
        crate::modules::workspace::domain::classifier::NodeType::FlatModRoot
            | crate::modules::workspace::domain::classifier::NodeType::ModPackRoot
            | crate::modules::workspace::domain::classifier::NodeType::VariantContainer
    );
    let self_is_enabled = is_root
        || path_file_name_lossy(&resolved.target).is_none_or(|name| !is_disabled_folder(&name));
    let (mut ancestor_disabled_by, mut ancestor_disabled_path) = explorer_sub_path
        .filter(|path| !path.is_empty())
        .and_then(|path| find_disabled_ancestor(mods_path, path))
        .unzip();
    if resolved.is_root_disabled && ancestor_disabled_by.is_none() {
        ancestor_disabled_by = path_file_name_lossy(&resolved.base)
            .map(|name| normalize_display_name(&name).into_owned());
        ancestor_disabled_path = Some(resolved.base.to_string_lossy().into_owned());
    }

    Ok(
        crate::modules::workspace::application::explorer::types::FolderGridResponse {
            self_node_type: Some(node_type.as_str().to_string()),
            self_is_mod,
            self_is_enabled,
            self_owner_object_id: None,
            self_owner_object_folder_path: None,
            self_classification_reasons: classification_reasons,
            children: Vec::new(),
            conflicts: Vec::new(),
            ancestor_disabled_by,
            ancestor_disabled_path,
        },
    )
}

async fn resolve_target_blocking(
    mods_path: &str,
    sub_path: Option<&str>,
) -> Result<ResolvedListingTarget, AppError> {
    let mods_path = mods_path.to_string();
    let sub_path = sub_path.map(str::to_string);
    tokio::task::spawn_blocking(move || resolve_listing_target(&mods_path, sub_path.as_deref()))
        .await?
}

/// One indexed aggregate row per immediate child. Descendant mod rows are
/// reduced by SQLite and never materialized as a game-wide Rust map.
async fn load_paged_listing_index(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    current_key: &str,
) -> Result<PagedListingIndex, AppError> {
    let rows = if current_key.is_empty() {
        sqlx::query_as::<_, ChildListingRow>(
            r#"
            WITH child_rows AS (
                SELECT
                    folder_path_key,
                    COALESCE(is_favorite, 0) AS is_favorite,
                    COALESCE(is_safe, 1) AS is_safe,
                    COALESCE(safety_source, 'unknown') AS safety_source,
                    CASE
                        WHEN instr(folder_path_key, '/') = 0 THEN folder_path_key
                        ELSE substr(folder_path_key, 1, instr(folder_path_key, '/') - 1)
                    END AS child_key
                FROM mods
                WHERE game_id = ? AND folder_path_key <> ''
            )
            SELECT
                child_key,
                MAX(CASE WHEN folder_path_key = child_key THEN 1 ELSE 0 END) AS is_terminal,
                MAX(CASE WHEN folder_path_key = child_key THEN is_favorite ELSE 0 END) AS is_favorite,
                MAX(CASE WHEN folder_path_key = child_key THEN is_safe ELSE 0 END) AS exact_is_safe,
                MAX(CASE WHEN folder_path_key = child_key AND safety_source <> 'unknown' THEN 1 ELSE 0 END) AS exact_is_classified,
                MAX(CASE WHEN safety_source <> 'unknown' AND is_safe <> 0 THEN 1 ELSE 0 END) AS contains_safe,
                MAX(CASE WHEN safety_source <> 'unknown' AND is_safe = 0 THEN 1 ELSE 0 END) AS contains_unsafe
            FROM child_rows
            GROUP BY child_key
            "#,
        )
        .bind(game_id)
        .fetch_all(pool)
        .await?
    } else {
        let descendants = format!("{}/%", escape_like(current_key));
        sqlx::query_as::<_, ChildListingRow>(
            r#"
            WITH scoped AS (
                SELECT
                    folder_path_key,
                    COALESCE(is_favorite, 0) AS is_favorite,
                    COALESCE(is_safe, 1) AS is_safe,
                    COALESCE(safety_source, 'unknown') AS safety_source,
                    substr(folder_path_key, length(?) + 2) AS relative_key
                FROM mods
                WHERE game_id = ? AND folder_path_key LIKE ? ESCAPE '\'
            ), child_rows AS (
                SELECT
                    folder_path_key,
                    is_favorite,
                    is_safe,
                    safety_source,
                    ? || '/' || CASE
                        WHEN instr(relative_key, '/') = 0 THEN relative_key
                        ELSE substr(relative_key, 1, instr(relative_key, '/') - 1)
                    END AS child_key
                FROM scoped
            )
            SELECT
                child_key,
                MAX(CASE WHEN folder_path_key = child_key THEN 1 ELSE 0 END) AS is_terminal,
                MAX(CASE WHEN folder_path_key = child_key THEN is_favorite ELSE 0 END) AS is_favorite,
                MAX(CASE WHEN folder_path_key = child_key THEN is_safe ELSE 0 END) AS exact_is_safe,
                MAX(CASE WHEN folder_path_key = child_key AND safety_source <> 'unknown' THEN 1 ELSE 0 END) AS exact_is_classified,
                MAX(CASE WHEN safety_source <> 'unknown' AND is_safe <> 0 THEN 1 ELSE 0 END) AS contains_safe,
                MAX(CASE WHEN safety_source <> 'unknown' AND is_safe = 0 THEN 1 ELSE 0 END) AS contains_unsafe
            FROM child_rows
            GROUP BY child_key
            "#,
        )
        .bind(current_key)
        .bind(game_id)
        .bind(descendants)
        .bind(current_key)
        .fetch_all(pool)
        .await?
    };

    Ok(PagedListingIndex {
        children: rows
            .into_iter()
            .map(|row| {
                (
                    row.child_key,
                    ChildListingState {
                        is_terminal: row.is_terminal != 0,
                        is_favorite: row.is_favorite != 0,
                        exact_is_safe: row.exact_is_safe != 0,
                        exact_is_classified: row.exact_is_classified != 0,
                        contains_safe: row.contains_safe != 0,
                        contains_unsafe: row.contains_unsafe != 0,
                    },
                )
            })
            .collect(),
    })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn build_first_workspace_explorer_page(
    mods_path: String,
    input: WorkspaceExplorerPageInput,
    resolved: ResolvedListingTarget,
    index: PagedListingIndex,
    query_fingerprint: String,
    build_token: Option<Arc<AtomicBool>>,
) -> Result<PageBuild, AppError> {
    let mut candidates = Vec::new();
    visit_matching_candidates(
        &resolved.target,
        &mods_path,
        &input.query,
        &index,
        build_token.as_deref(),
        |candidate| {
            ensure_listing_snapshot_capacity(candidates.len().saturating_add(1))?;
            candidates.push(candidate);
            Ok(())
        },
    )?;
    if build_token
        .as_deref()
        .is_some_and(|token| !token.load(AtomicOrdering::Acquire))
    {
        return Err(AppError::ExplorerSnapshotExpired);
    }
    candidates.par_sort_unstable_by(|left, right| compare_candidates(left, right, &input.query));
    if build_token
        .as_deref()
        .is_some_and(|token| !token.load(AtomicOrdering::Acquire))
    {
        return Err(AppError::ExplorerSnapshotExpired);
    }
    let estimated_bytes = estimate_listing_candidates_bytes(&candidates);
    let snapshot = Arc::new(ListingSnapshot {
        query_fingerprint: query_fingerprint.clone(),
        target_key: physical_path_key(&resolved.target),
        candidates,
        estimated_bytes,
    });
    let snapshot_id = store_listing_snapshot(Arc::clone(&snapshot))?;
    build_workspace_explorer_page_from_snapshot(
        mods_path,
        input,
        resolved,
        snapshot_id,
        snapshot,
        0,
    )
}

fn build_cached_workspace_explorer_page(
    mods_path: String,
    input: WorkspaceExplorerPageInput,
    resolved: ResolvedListingTarget,
    cursor: ExplorerCursor,
) -> Result<PageBuild, AppError> {
    let snapshot = get_listing_snapshot(&cursor.snapshot_id)?;
    validate_listing_snapshot(&snapshot, &resolved.target, &cursor.query_fingerprint)?;
    build_workspace_explorer_page_from_snapshot(
        mods_path,
        input,
        resolved,
        cursor.snapshot_id,
        snapshot,
        cursor.offset,
    )
}

fn build_workspace_explorer_page_from_snapshot(
    mods_path: String,
    input: WorkspaceExplorerPageInput,
    resolved: ResolvedListingTarget,
    snapshot_id: String,
    snapshot: Arc<ListingSnapshot>,
    offset: usize,
) -> Result<PageBuild, AppError> {
    if offset > snapshot.candidates.len() {
        return Err(AppError::ExplorerSnapshotExpired);
    }
    let requested_page_size = if input.page_size == 0 {
        DEFAULT_EXPLORER_PAGE_SIZE
    } else {
        input.page_size
    };
    let page_size = requested_page_size.clamp(1, MAX_EXPLORER_PAGE_SIZE) as usize;
    let end = offset
        .saturating_add(page_size)
        .min(snapshot.candidates.len());
    let next_cursor = if end < snapshot.candidates.len() {
        Some(encode_cursor(
            &snapshot.query_fingerprint,
            &snapshot_id,
            end,
        )?)
    } else {
        None
    };
    let mut folders = Vec::with_capacity(end.saturating_sub(offset));
    for candidate in &snapshot.candidates[offset..end] {
        let identity_matches = candidate.filesystem_identity.as_deref().is_some_and(|expected| {
            crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(
                &candidate.path,
            )
            .as_deref()
                == Some(expected)
        });
        if !identity_matches {
            return Err(AppError::ExplorerSnapshotExpired);
        }
        let mut folder =
            build_mod_folder_from_path(&candidate.path, input.query.explorer_sub_path.as_deref())
                .ok_or(AppError::ExplorerSnapshotExpired)?;
        apply_child_state(&mut folder, candidate.state.clone());
        folders.push(folder);
    }

    let mut ancestor_disabled_by = input
        .query
        .explorer_sub_path
        .as_deref()
        .filter(|path| !path.is_empty())
        .and_then(|path| find_disabled_ancestor(&mods_path, path))
        .map(|(name, _)| name);
    if resolved.is_root_disabled && ancestor_disabled_by.is_none() {
        ancestor_disabled_by = path_file_name_lossy(&resolved.base)
            .map(|name| normalize_display_name(&name).into_owned());
    }

    Ok(PageBuild {
        folders,
        next_cursor,
        total_matching: snapshot.candidates.len() as u64,
        query_fingerprint: snapshot.query_fingerprint.clone(),
        listing_revision: snapshot_id,
        ancestor_disabled_by,
    })
}

fn store_listing_snapshot(snapshot: Arc<ListingSnapshot>) -> Result<String, AppError> {
    let candidate_count = snapshot.candidates.len();
    ensure_listing_snapshot_capacity(candidate_count)?;
    let sequence = LISTING_SNAPSHOT_CLOCK.fetch_add(1, AtomicOrdering::Relaxed);
    let snapshot_id = format!("{}-{sequence}", &snapshot.query_fingerprint[..16]);
    let estimated_bytes = snapshot.estimated_bytes;
    let cache = LISTING_SNAPSHOT_CACHE.get_or_init(|| Mutex::new(ListingSnapshotCache::default()));
    let mut cache = cache.lock().map_err(|_| {
        AppError::Internal("Explorer listing snapshot cache was poisoned".to_string())
    })?;
    let mut evicted_snapshots = 0_usize;
    while !cache.entries.is_empty()
        && (cache.entries.len() >= MAX_LISTING_SNAPSHOTS
            || cache.total_candidates.saturating_add(candidate_count)
                > MAX_CACHED_LISTING_CANDIDATES)
    {
        let Some(oldest_id) = cache
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(id, _)| id.clone())
        else {
            break;
        };
        if let Some(removed) = cache.entries.remove(&oldest_id) {
            cache.total_candidates = cache
                .total_candidates
                .saturating_sub(removed.snapshot.candidates.len());
            cache.total_estimated_bytes = cache
                .total_estimated_bytes
                .saturating_sub(removed.snapshot.estimated_bytes);
            evicted_snapshots += 1;
        }
    }
    cache.total_candidates = cache.total_candidates.saturating_add(candidate_count);
    cache.total_estimated_bytes = cache.total_estimated_bytes.saturating_add(estimated_bytes);
    cache.entries.insert(
        snapshot_id.clone(),
        CachedListingSnapshot {
            snapshot,
            last_used: sequence,
        },
    );
    log::debug!(
        "workspace explorer snapshot stored candidates={} estimated_bytes={} cache_candidates={} cache_estimated_bytes={} evicted_snapshots={}",
        candidate_count,
        estimated_bytes,
        cache.total_candidates,
        cache.total_estimated_bytes,
        evicted_snapshots,
    );
    Ok(snapshot_id)
}

fn register_listing_build(scope: &str) -> Result<ListingBuildLease, AppError> {
    let token = Arc::new(AtomicBool::new(true));
    let builds = LISTING_BUILD_TOKENS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut builds = builds
        .lock()
        .map_err(|_| AppError::Internal("Explorer listing build state was poisoned".to_string()))?;
    if let Some(previous) = builds.insert(scope.to_string(), Arc::clone(&token)) {
        previous.store(false, AtomicOrdering::Release);
    }
    Ok(ListingBuildLease {
        scope: scope.to_string(),
        token,
    })
}

fn finish_listing_build(scope: &str, token: &Arc<AtomicBool>) -> Result<(), AppError> {
    let builds = LISTING_BUILD_TOKENS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut builds = builds
        .lock()
        .map_err(|_| AppError::Internal("Explorer listing build state was poisoned".to_string()))?;
    if builds
        .get(scope)
        .is_some_and(|current| Arc::ptr_eq(current, token))
    {
        builds.remove(scope);
    }
    Ok(())
}

fn ensure_listing_snapshot_capacity(candidate_count: usize) -> Result<(), AppError> {
    if candidate_count > MAX_CACHED_LISTING_CANDIDATES {
        return Err(AppError::Validation(format!(
            "Explorer listing has {candidate_count} matching folders; narrow the search below the {MAX_CACHED_LISTING_CANDIDATES}-folder safety limit"
        )));
    }
    Ok(())
}

fn get_listing_snapshot(snapshot_id: &str) -> Result<Arc<ListingSnapshot>, AppError> {
    let cache = LISTING_SNAPSHOT_CACHE.get_or_init(|| Mutex::new(ListingSnapshotCache::default()));
    let mut cache = cache.lock().map_err(|_| {
        AppError::Internal("Explorer listing snapshot cache was poisoned".to_string())
    })?;
    let entry = cache
        .entries
        .get_mut(snapshot_id)
        .ok_or(AppError::ExplorerSnapshotExpired)?;
    entry.last_used = LISTING_SNAPSHOT_CLOCK.fetch_add(1, AtomicOrdering::Relaxed);
    Ok(Arc::clone(&entry.snapshot))
}

fn apply_child_state(folder: &mut ModFolder, state: ChildListingState) {
    folder.is_favorite = state.is_favorite;
    folder.contains_safe_mods = state.contains_safe;
    folder.contains_unsafe_mods = state.contains_unsafe;
    if state.is_terminal && state.exact_is_classified {
        folder.is_safe = state.exact_is_safe;
        folder.is_safety_classified = true;
    }
}

async fn enrich_page_owners(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    mods_path: &str,
    folders: &mut [ModFolder],
) -> Result<(), AppError> {
    let mut owner_keys = HashSet::new();
    for folder in folders.iter() {
        let mut key = folder_path_key(&folder.path, Some(mods_path));
        while !key.is_empty() {
            owner_keys.insert(key.clone());
            let Some((parent, _)) = key.rsplit_once('/') else {
                break;
            };
            key.truncate(parent.len());
        }
    }
    let owner_keys = owner_keys.into_iter().collect::<Vec<_>>();
    let owners = get_runtime_descriptors_for_folder_path_keys(pool, game_id, &owner_keys).await?;
    let owner_index = owners.into_iter().fold(
        HashMap::<String, ObjectRuntimeDescriptor>::new(),
        |mut index, owner| {
            index.entry(owner.folder_path_key.clone()).or_insert(owner);
            index
        },
    );

    for folder in folders {
        let mut key = folder_path_key(&folder.path, Some(mods_path));
        while !key.is_empty() {
            if let Some(owner) = owner_index.get(&key) {
                folder.owner_object_id = Some(owner.id.clone());
                folder.owner_object_folder_path = Some(owner.folder_path.clone());
                break;
            }
            let Some((parent, _)) = key.rsplit_once('/') else {
                break;
            };
            key.truncate(parent.len());
        }
    }
    Ok(())
}

fn resolve_workspace_explorer_selection_blocking(
    input: WorkspaceExplorerSelectionInput,
    snapshot: Arc<ListingSnapshot>,
) -> Result<ResolvedWorkspaceExplorerSelection, AppError> {
    let mut selected = Vec::new();
    let mut expected_identities = Vec::new();

    match input.selection {
        WorkspaceExplorerSelection::Explicit { paths } => {
            if paths.len() > MAX_RESOLVED_SELECTION_PATHS {
                return Err(selection_limit_error());
            }
            let mut requested = paths
                .iter()
                .map(|path| physical_path_key(Path::new(path)))
                .collect::<HashSet<_>>();
            if requested.len() > MAX_RESOLVED_SELECTION_PATHS {
                return Err(selection_limit_error());
            }
            for candidate in &snapshot.candidates {
                if requested.remove(&candidate.sort_key.path_key) {
                    push_snapshot_candidate(candidate, &mut selected, &mut expected_identities)?;
                }
            }
            if !requested.is_empty() {
                return Err(AppError::Validation(
                    "Explorer selection contains paths outside the current query scope".to_string(),
                ));
            }
        }
        WorkspaceExplorerSelection::AllMatching { excluded_paths } => {
            if excluded_paths.len() > MAX_RESOLVED_SELECTION_PATHS {
                return Err(selection_limit_error());
            }
            let excluded = excluded_paths
                .iter()
                .map(|path| physical_path_key(Path::new(path)))
                .collect::<HashSet<_>>();
            for candidate in &snapshot.candidates {
                if excluded.contains(&candidate.sort_key.path_key) {
                    continue;
                }
                if selected.len() == MAX_RESOLVED_SELECTION_PATHS {
                    return Err(selection_limit_error());
                }
                push_snapshot_candidate(candidate, &mut selected, &mut expected_identities)?;
            }
        }
    }

    Ok(ResolvedWorkspaceExplorerSelection {
        paths: selected,
        expected_identities,
    })
}

fn push_snapshot_candidate(
    candidate: &ListingCandidate,
    selected: &mut Vec<String>,
    expected_identities: &mut Vec<(String, String)>,
) -> Result<(), AppError> {
    let path = candidate.path.to_string_lossy().into_owned();
    let identity = candidate
        .filesystem_identity
        .clone()
        .ok_or(AppError::ExplorerSnapshotExpired)?;
    selected.push(path.clone());
    expected_identities.push((path, identity));
    Ok(())
}

/// Revalidate the exact directories shown by the listing snapshot. Callers
/// must hold the game's mutation lock so an ABA replacement cannot slip
/// between this check and mutation planning.
pub fn validate_workspace_explorer_selection_identities(
    expected_identities: &[(String, String)],
) -> Result<(), AppError> {
    let unchanged = expected_identities.iter().all(|(path, expected)| {
        crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(
            Path::new(path),
        )
        .as_deref()
            == Some(expected.as_str())
    });
    if unchanged {
        Ok(())
    } else {
        Err(AppError::ExplorerSnapshotExpired)
    }
}

fn validate_listing_snapshot(
    snapshot: &ListingSnapshot,
    target: &Path,
    query_fingerprint: &str,
) -> Result<(), AppError> {
    if snapshot.query_fingerprint != query_fingerprint
        || snapshot.target_key != physical_path_key(target)
    {
        return Err(AppError::ExplorerSnapshotExpired);
    }
    Ok(())
}

fn estimate_listing_candidates_bytes(candidates: &[ListingCandidate]) -> usize {
    candidates.iter().fold(
        std::mem::size_of::<ListingCandidate>().saturating_mul(candidates.len()),
        |estimated, candidate| {
            estimated
                .saturating_add(candidate.path.as_os_str().len())
                .saturating_add(
                    candidate
                        .filesystem_identity
                        .as_ref()
                        .map_or(0, String::capacity),
                )
                .saturating_add(candidate.sort_key.name_key.capacity())
                .saturating_add(candidate.sort_key.path_key.capacity())
        },
    )
}

fn visit_matching_candidates(
    target: &Path,
    mods_path: &str,
    query: &WorkspaceExplorerQuery,
    index: &PagedListingIndex,
    build_token: Option<&AtomicBool>,
    mut visit: impl FnMut(ListingCandidate) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let entries = std::fs::read_dir(target).map_err(|error| {
        AppError::Io(format!(
            "Could not read explorer directory {}: {error}",
            target.display()
        ))
    })?;
    let search = query
        .search_query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);

    for (entry_index, entry) in entries.enumerate() {
        if entry_index % 64 == 0
            && build_token.is_some_and(|token| !token.load(AtomicOrdering::Acquire))
        {
            return Err(AppError::ExplorerSnapshotExpired);
        }
        let entry = entry.map_err(|error| {
            AppError::Io(format!(
                "Explorer directory enumeration was incomplete for {}: {error}",
                target.display()
            ))
        })?;
        let path = entry.path();
        if !entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
            continue;
        }
        let Some(folder_name) = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
        else {
            continue;
        };
        if folder_name.starts_with('.') {
            continue;
        }
        let display_name = if is_disabled_folder(&folder_name) {
            normalize_display_name(&folder_name).into_owned()
        } else {
            folder_name
        };
        if search
            .as_ref()
            .is_some_and(|search| !display_name.to_lowercase().contains(search))
        {
            continue;
        }

        let child_key = folder_path_key(&path.to_string_lossy(), Some(mods_path));
        let state = index.state(&child_key);
        let safety_matches = match query.safety_filter {
            WorkspaceExplorerSafetyFilter::All => true,
            WorkspaceExplorerSafetyFilter::Safe => !state.is_terminal || state.contains_safe,
            WorkspaceExplorerSafetyFilter::Unsafe => !state.is_terminal || state.contains_unsafe,
        };
        if !safety_matches {
            continue;
        }

        let metadata = entry.metadata().ok();
        let modified_at = metadata
            .as_ref()
            .and_then(|value| value.modified().ok())
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_secs())
            .unwrap_or(0);
        let size_bytes = metadata.map(|value| value.len()).unwrap_or(0);
        visit(ListingCandidate {
            filesystem_identity: crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::filesystem_identity(&path),
            sort_key: CandidateSortKey {
                is_favorite: state.is_favorite,
                is_container: !state.is_terminal,
                name_key: canonical_name_key(&display_name),
                modified_at,
                size_bytes,
                path_key: physical_path_key(&path),
            },
            path,
            state,
        })?;
    }
    Ok(())
}

fn compare_candidates(
    left: &ListingCandidate,
    right: &ListingCandidate,
    query: &WorkspaceExplorerQuery,
) -> Ordering {
    compare_sort_keys(&left.sort_key, &right.sort_key, query)
}

fn compare_sort_keys(
    left: &CandidateSortKey,
    right: &CandidateSortKey,
    query: &WorkspaceExplorerQuery,
) -> Ordering {
    right
        .is_favorite
        .cmp(&left.is_favorite)
        .then_with(|| right.is_container.cmp(&left.is_container))
        .then_with(|| {
            let ordering = match query.sort_field {
                WorkspaceExplorerSortField::Name => left.name_key.cmp(&right.name_key),
                WorkspaceExplorerSortField::ModifiedAt => left.modified_at.cmp(&right.modified_at),
                WorkspaceExplorerSortField::SizeBytes => left.size_bytes.cmp(&right.size_bytes),
            };
            apply_sort_order(ordering, query.sort_order)
        })
        .then_with(|| apply_sort_order(left.name_key.cmp(&right.name_key), query.sort_order))
        .then_with(|| left.path_key.cmp(&right.path_key))
}

fn apply_sort_order(ordering: Ordering, sort_order: WorkspaceExplorerSortOrder) -> Ordering {
    match sort_order {
        WorkspaceExplorerSortOrder::Asc => ordering,
        WorkspaceExplorerSortOrder::Desc => ordering.reverse(),
    }
}

fn physical_path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn query_fingerprint(query: &WorkspaceExplorerQuery) -> Result<String, AppError> {
    let bytes = serde_json::to_vec(query).map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn encode_cursor(
    query_fingerprint: &str,
    snapshot_id: &str,
    offset: usize,
) -> Result<String, AppError> {
    let payload = ExplorerCursor {
        version: CURSOR_VERSION,
        query_fingerprint: query_fingerprint.to_string(),
        snapshot_id: snapshot_id.to_string(),
        offset,
    };
    let bytes =
        serde_json::to_vec(&payload).map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn decode_cursor(cursor: &str, query_fingerprint: &str) -> Result<ExplorerCursor, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| AppError::ExplorerSnapshotExpired)?;
    let payload = serde_json::from_slice::<ExplorerCursor>(&bytes)
        .map_err(|_| AppError::ExplorerSnapshotExpired)?;
    if payload.version != CURSOR_VERSION || payload.query_fingerprint != query_fingerprint {
        return Err(AppError::ExplorerSnapshotExpired);
    }
    Ok(payload)
}

fn selection_limit_error() -> AppError {
    AppError::Validation(format!(
        "Bulk operations support at most {MAX_RESOLVED_SELECTION_PATHS} paths"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(sort_field: WorkspaceExplorerSortField) -> WorkspaceExplorerQuery {
        WorkspaceExplorerQuery {
            game_id: "game".to_string(),
            explorer_sub_path: None,
            search_query: None,
            sort_field,
            sort_order: WorkspaceExplorerSortOrder::Asc,
            safety_filter: WorkspaceExplorerSafetyFilter::All,
        }
    }

    fn candidate(name: &str, modified_at: u64, size_bytes: u64) -> ListingCandidate {
        ListingCandidate {
            path: PathBuf::from(name),
            filesystem_identity: Some(format!("fixture:{name}")),
            sort_key: CandidateSortKey {
                is_favorite: false,
                is_container: false,
                name_key: name.to_string(),
                modified_at,
                size_bytes,
                path_key: name.to_string(),
            },
            state: ChildListingState::default(),
        }
    }

    fn page_input(
        query: WorkspaceExplorerQuery,
        page_size: u32,
        cursor: Option<String>,
    ) -> WorkspaceExplorerPageInput {
        WorkspaceExplorerPageInput {
            query,
            cursor,
            page_size,
        }
    }

    fn create_folders(root: &Path, names: &[&str]) {
        for name in names {
            std::fs::create_dir(root.join(name)).expect("create test folder");
        }
    }

    fn test_page(
        mods_path: String,
        input: WorkspaceExplorerPageInput,
        index: PagedListingIndex,
    ) -> WorkspaceExplorerPage {
        let resolved = resolve_listing_target(&mods_path, input.query.explorer_sub_path.as_deref())
            .expect("resolve target");
        let fingerprint = query_fingerprint(&input.query).expect("query fingerprint");
        let cursor = input
            .cursor
            .as_deref()
            .map(|cursor| decode_cursor(cursor, &fingerprint).expect("decode cursor"));
        let page = if let Some(cursor) = cursor {
            build_cached_workspace_explorer_page(mods_path, input, resolved, cursor)
        } else {
            build_first_workspace_explorer_page(
                mods_path,
                input,
                resolved,
                index,
                fingerprint,
                None,
            )
        };
        page.expect("build page").into_page()
    }

    #[test]
    fn cursor_round_trip_is_query_bound_and_stable() {
        let query = query(WorkspaceExplorerSortField::Name);
        let fingerprint = query_fingerprint(&query).expect("fingerprint");
        let cursor = encode_cursor(&fingerprint, "snapshot", 2).expect("cursor");

        let decoded = decode_cursor(&cursor, &fingerprint).expect("decode cursor");
        assert_eq!(decoded.snapshot_id, "snapshot");
        assert_eq!(decoded.offset, 2);

        let changed = query_fingerprint(&WorkspaceExplorerQuery {
            search_query: Some("other".to_string()),
            ..query
        })
        .expect("changed fingerprint");
        assert!(matches!(
            decode_cursor(&cursor, &changed),
            Err(AppError::ExplorerSnapshotExpired)
        ));
    }

    #[test]
    fn superseded_first_page_build_stops_before_snapshot_publication() {
        let temp = tempfile::tempdir().expect("temp directory");
        create_folders(temp.path(), &["alpha"]);
        let mods_path = temp.path().to_string_lossy().into_owned();
        let input = page_input(query(WorkspaceExplorerSortField::Name), 1, None);
        let resolved = resolve_listing_target(&mods_path, None).expect("resolve target");
        let fingerprint = query_fingerprint(&input.query).expect("query fingerprint");
        let token = Arc::new(AtomicBool::new(false));

        assert!(matches!(
            build_first_workspace_explorer_page(
                mods_path,
                input,
                resolved,
                PagedListingIndex::default(),
                fingerprint,
                Some(token),
            ),
            Err(AppError::ExplorerSnapshotExpired)
        ));
    }

    #[test]
    fn sort_is_deterministic_for_equal_primary_values() {
        let query = query(WorkspaceExplorerSortField::SizeBytes);
        let mut candidates = vec![candidate("bravo", 1, 10), candidate("alpha", 2, 10)];
        candidates.sort_by(|left, right| compare_candidates(left, right, &query));

        assert_eq!(candidates[0].sort_key.name_key, "alpha");
        assert_eq!(candidates[1].sort_key.name_key, "bravo");
    }

    #[test]
    fn cursor_reuses_a_stable_snapshot_when_directory_contents_change() {
        let temp = tempfile::tempdir().expect("temp directory");
        create_folders(temp.path(), &["alpha", "bravo", "charlie", "delta"]);
        let mods_path = temp.path().to_string_lossy().into_owned();
        let query = query(WorkspaceExplorerSortField::Name);
        let first = test_page(
            mods_path.clone(),
            page_input(query.clone(), 2, None),
            PagedListingIndex::default(),
        );
        assert_eq!(
            first
                .items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "bravo"]
        );

        create_folders(temp.path(), &["baker"]);
        let second = test_page(
            mods_path,
            page_input(query, 2, first.next_cursor),
            PagedListingIndex::default(),
        );
        assert_eq!(second.listing_revision, first.listing_revision);
        assert_eq!(
            second
                .items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["charlie", "delta"]
        );
    }

    #[test]
    fn page_size_is_bounded() {
        let temp = tempfile::tempdir().expect("temp directory");
        for index in 0..=MAX_EXPLORER_PAGE_SIZE {
            std::fs::create_dir(temp.path().join(format!("folder-{index:03}")))
                .expect("create test folder");
        }
        let page = test_page(
            temp.path().to_string_lossy().into_owned(),
            page_input(query(WorkspaceExplorerSortField::Name), u32::MAX, None),
            PagedListingIndex::default(),
        );

        assert_eq!(page.items.len(), MAX_EXPLORER_PAGE_SIZE as usize);
        assert!(page.next_cursor.is_some());
    }

    #[test]
    fn search_sort_and_safety_filter_run_before_pagination() {
        let temp = tempfile::tempdir().expect("temp directory");
        create_folders(temp.path(), &["alpha", "beta", "gamma"]);
        let mods_path = temp.path().to_string_lossy().into_owned();
        let alpha_key = folder_path_key(
            &temp.path().join("alpha").to_string_lossy(),
            Some(&mods_path),
        );
        let beta_key = folder_path_key(
            &temp.path().join("beta").to_string_lossy(),
            Some(&mods_path),
        );
        let index = PagedListingIndex::from_states(vec![
            (alpha_key, false, true, false),
            (beta_key, false, false, true),
        ]);
        let page = test_page(
            mods_path,
            page_input(
                WorkspaceExplorerQuery {
                    search_query: Some("a".to_string()),
                    sort_order: WorkspaceExplorerSortOrder::Desc,
                    safety_filter: WorkspaceExplorerSafetyFilter::Safe,
                    ..query(WorkspaceExplorerSortField::Name)
                },
                1,
                None,
            ),
            index,
        );

        assert_eq!(page.total_matching, 2);
        assert_eq!(page.items[0].name, "gamma");
        assert!(page.next_cursor.is_some());
    }

    #[test]
    fn all_matching_selection_resolves_exclusions_without_client_path_expansion() {
        let temp = tempfile::tempdir().expect("temp directory");
        create_folders(temp.path(), &["alpha", "bravo", "charlie"]);
        let mods_path = temp.path().to_string_lossy().into_owned();
        let excluded = temp.path().join("bravo").to_string_lossy().into_owned();
        let page = test_page(
            mods_path,
            page_input(query(WorkspaceExplorerSortField::Name), 2, None),
            PagedListingIndex::default(),
        );
        create_folders(temp.path(), &["delta"]);
        let input = WorkspaceExplorerSelectionInput {
            query: query(WorkspaceExplorerSortField::Name),
            listing_revision: page.listing_revision.clone(),
            selection: WorkspaceExplorerSelection::AllMatching {
                excluded_paths: vec![excluded],
            },
        };
        let snapshot = get_listing_snapshot(&page.listing_revision).expect("listing snapshot");
        let result =
            resolve_workspace_explorer_selection_blocking(input, snapshot).expect("selection");

        assert_eq!(result.paths.len(), 2);
        assert_eq!(result.expected_identities.len(), 2);
        assert_eq!(
            result
                .paths
                .iter()
                .filter_map(|path| Path::new(path).file_name()?.to_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "charlie"]
        );
    }

    #[test]
    fn snapshot_identity_rejects_a_replacement_at_the_same_path() {
        let temp = tempfile::tempdir().expect("temp directory");
        create_folders(temp.path(), &["alpha"]);
        let mods_path = temp.path().to_string_lossy().into_owned();
        let page = test_page(
            mods_path.clone(),
            page_input(query(WorkspaceExplorerSortField::Name), 1, None),
            PagedListingIndex::default(),
        );
        let input = WorkspaceExplorerSelectionInput {
            query: query(WorkspaceExplorerSortField::Name),
            listing_revision: page.listing_revision.clone(),
            selection: WorkspaceExplorerSelection::AllMatching {
                excluded_paths: Vec::new(),
            },
        };
        let snapshot = get_listing_snapshot(&page.listing_revision).expect("listing snapshot");
        let resolved =
            resolve_workspace_explorer_selection_blocking(input, snapshot).expect("selection");
        let selected_path = temp.path().join("alpha");
        std::fs::remove_dir(&selected_path).expect("remove original folder");
        std::fs::create_dir(&selected_path).expect("create replacement folder");

        assert!(matches!(
            validate_workspace_explorer_selection_identities(&resolved.expected_identities),
            Err(AppError::ExplorerSnapshotExpired)
        ));
    }

    #[test]
    fn cached_page_rejects_a_replacement_at_the_same_path() {
        let temp = tempfile::tempdir().expect("temp directory");
        create_folders(temp.path(), &["alpha", "bravo"]);
        let mods_path = temp.path().to_string_lossy().into_owned();
        let query = query(WorkspaceExplorerSortField::Name);
        let first = test_page(
            mods_path.clone(),
            page_input(query.clone(), 1, None),
            PagedListingIndex::default(),
        );
        let cursor_value = first.next_cursor.expect("second page cursor");
        let fingerprint = query_fingerprint(&query).expect("fingerprint");
        let cursor = decode_cursor(&cursor_value, &fingerprint).expect("cursor");
        let replacement = temp.path().join("bravo");
        std::fs::remove_dir(&replacement).expect("remove original folder");
        std::fs::create_dir(&replacement).expect("create replacement folder");
        let resolved = resolve_listing_target(&mods_path, None).expect("resolve target");

        assert!(matches!(
            build_cached_workspace_explorer_page(
                mods_path,
                page_input(query, 1, Some(cursor_value)),
                resolved,
                cursor,
            ),
            Err(AppError::ExplorerSnapshotExpired)
        ));
    }

    #[test]
    fn missing_listing_revision_is_a_typed_recoverable_error() {
        assert!(matches!(
            get_listing_snapshot("missing-listing-revision"),
            Err(AppError::ExplorerSnapshotExpired)
        ));
    }

    #[test]
    fn oversized_listing_is_rejected_before_it_can_exceed_the_snapshot_bound() {
        assert!(matches!(
            ensure_listing_snapshot_capacity(MAX_CACHED_LISTING_CANDIDATES + 1),
            Err(AppError::Validation(message)) if message.contains("narrow the search")
        ));
    }

    /// Perf harness, not a CI timing gate. It exercises the 100k candidate
    /// sort and memory estimator without creating 100k filesystem entries.
    ///
    /// cargo test --lib benchmark_100k_listing_candidates -- --ignored --nocapture
    #[test]
    #[ignore = "manual 100k explorer benchmark"]
    fn benchmark_100k_listing_candidates() {
        const COUNT: usize = 100_000;
        const SAMPLES: usize = 5;
        let query = query(WorkspaceExplorerSortField::Name);
        let base = (0..COUNT)
            .rev()
            .map(|index| candidate(&format!("mod-{index:06}"), index as u64, index as u64))
            .collect::<Vec<_>>();
        let estimated_bytes = estimate_listing_candidates_bytes(&base);
        let mut samples = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let mut candidates = base.clone();
            let started = std::time::Instant::now();
            candidates.par_sort_unstable_by(|left, right| compare_candidates(left, right, &query));
            samples.push(started.elapsed());
            assert_eq!(candidates.len(), COUNT);
        }
        samples.sort_unstable();
        let p50 = samples[SAMPLES / 2];
        let p95 = samples[SAMPLES - 1];
        println!(
            "workspace_explorer_100k: candidates={COUNT} estimated_bytes={estimated_bytes} p50={p50:?} p95={p95:?}"
        );
    }
}
