use crate::modules::library::application::mods::trash;
use crate::modules::workspace::application::scanner::watcher::{
    SuppressionGuard, WatcherSuppressor,
};
use crate::shared::errors::AppError;
use crate::shared::errors::ScannerError;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod hardlink;

#[cfg(test)]
pub(crate) use hardlink::replace_file_with_hardlink_using;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionRequest {
    pub group_id: String,
    pub action: ResolutionAction,
    pub folder_a: String,
    pub folder_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionAction {
    KeepA,
    KeepB,
    Ignore,
    Hardlink,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionSummary {
    #[specta(type = f64)]
    pub total: usize,
    #[specta(type = f64)]
    pub successful: usize,
    #[specta(type = f64)]
    pub failed: usize,
    pub errors: Vec<ResolutionError>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionError {
    pub group_id: String,
    pub action: ResolutionAction,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionProgress {
    #[specta(type = f64)]
    pub current: usize,
    #[specta(type = f64)]
    pub total: usize,
    pub group_id: String,
    pub action: ResolutionAction,
}

pub struct DurableResolutionBatch {
    actions: Vec<DurableResolutionAction>,
    steps: Vec<crate::modules::mutation::journal::PlannedStep>,
}

enum DurableResolutionAction {
    Quarantine {
        request: ResolutionRequest,
        source: PathBuf,
        quarantine: PathBuf,
        sequence: u32,
    },
    Hardlink {
        request: ResolutionRequest,
        replacements: Vec<hardlink::PreparedHardlinkReplacement>,
    },
    Ignore(ResolutionRequest),
}

impl DurableResolutionBatch {
    pub fn operation_plan(
        &self,
        game_id: &str,
    ) -> Option<crate::modules::mutation::journal::OperationPlan> {
        (!self.steps.is_empty()).then(|| {
            crate::modules::mutation::journal::OperationPlan::new(
                "duplicate-resolution",
                game_id,
                self.steps.clone(),
            )
        })
    }

    pub fn requests(&self) -> Vec<ResolutionRequest> {
        self.actions
            .iter()
            .map(|action| match action {
                DurableResolutionAction::Quarantine { request, .. }
                | DurableResolutionAction::Hardlink { request, .. }
                | DurableResolutionAction::Ignore(request) => request.clone(),
            })
            .collect()
    }

    pub fn finalize(&self) {
        for action in &self.actions {
            let result = match action {
                DurableResolutionAction::Quarantine { quarantine, .. } if quarantine.exists() => {
                    crate::platform::fs::recycle_bin::move_path_to_recycle_bin(quarantine)
                        .map_err(|error| ScannerError::Io(error.to_string()))
                }
                DurableResolutionAction::Hardlink { replacements, .. } => replacements
                    .iter()
                    .try_for_each(hardlink::PreparedHardlinkReplacement::finalize),
                _ => Ok(()),
            };
            if let Err(error) = result {
                log::warn!("Duplicate resolution cleanup remains pending: {error}");
            }
        }
    }
}

pub async fn prepare_durable_batch(
    requests: Vec<ResolutionRequest>,
    game_id: &str,
    db: &SqlitePool,
) -> Result<DurableResolutionBatch, AppError> {
    validate_ignore_group_coverage(&requests, game_id, db)
        .await
        .map_err(|error| AppError::Validation(error.to_string()))?;

    let mut actions = Vec::with_capacity(requests.len());
    let mut steps = Vec::new();
    let mut sequence = 0u32;
    for request in requests {
        let confidence_score = authorize_request(&request, game_id, db)
            .await
            .map_err(|error| AppError::Validation(error.to_string()))?;
        match &request.action {
            ResolutionAction::KeepA | ResolutionAction::KeepB => {
                if confidence_score == 100 {
                    verify_full_folder_match(&request.folder_a, &request.folder_b)
                        .map_err(|error| AppError::Validation(error.to_string()))?;
                }
                let source = PathBuf::from(if matches!(&request.action, ResolutionAction::KeepA) {
                    &request.folder_b
                } else {
                    &request.folder_a
                });
                let quarantine = source.with_file_name(format!(
                    ".emmm-duplicate-quarantine-{}",
                    uuid::Uuid::new_v4().simple()
                ));
                steps.push(crate::modules::mutation::journal::PlannedStep::quarantine(
                    sequence,
                    source.clone(),
                    quarantine.clone(),
                ));
                actions.push(DurableResolutionAction::Quarantine {
                    request,
                    source,
                    quarantine,
                    sequence,
                });
                sequence += 1;
            }
            ResolutionAction::Hardlink => {
                verify_full_folder_match(&request.folder_a, &request.folder_b)
                    .map_err(|error| AppError::Validation(error.to_string()))?;
                let replacements = hardlink::prepare_hardlinks(
                    &request.folder_a,
                    &request.folder_b,
                    &mut sequence,
                )
                .map_err(|error| AppError::Validation(error.to_string()))?;
                steps.extend(
                    replacements
                        .iter()
                        .map(|replacement| replacement.journal_step()),
                );
                actions.push(DurableResolutionAction::Hardlink {
                    request,
                    replacements,
                });
            }
            ResolutionAction::Ignore => actions.push(DurableResolutionAction::Ignore(request)),
        }
    }
    Ok(DurableResolutionBatch { actions, steps })
}

pub async fn resolve_durable_batch<F>(
    batch: &DurableResolutionBatch,
    game_id: &str,
    db: &SqlitePool,
    lease: &crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskMutationLease,
    watcher_suppressor: &Arc<WatcherSuppressor>,
    mut on_progress: F,
) -> Result<ResolutionSummary, AppError>
where
    F: FnMut(ResolutionProgress),
{
    let _suppression_guard = SuppressionGuard::new(watcher_suppressor);
    let total = batch.actions.len();
    let mut pending_ignores_by_group =
        count_pending_ignores(batch.actions.iter().filter_map(|action| match action {
            DurableResolutionAction::Ignore(request) => Some(request),
            DurableResolutionAction::Quarantine { .. }
            | DurableResolutionAction::Hardlink { .. } => None,
        }));
    for (index, action) in batch.actions.iter().enumerate() {
        let request = match action {
            DurableResolutionAction::Quarantine { request, .. }
            | DurableResolutionAction::Hardlink { request, .. }
            | DurableResolutionAction::Ignore(request) => request,
        };
        on_progress(ResolutionProgress {
            current: index + 1,
            total,
            group_id: request.group_id.clone(),
            action: request.action.clone(),
        });
        match action {
            DurableResolutionAction::Quarantine {
                source,
                quarantine,
                sequence,
                ..
            } => {
                std::fs::rename(source, quarantine)?;
                lease.mark_step_applied(*sequence)?;
                set_group_status(db, &request.group_id, "resolved")
                    .await
                    .map_err(|error| AppError::Db(error.to_string()))?;
            }
            DurableResolutionAction::Hardlink { replacements, .. } => {
                for replacement in replacements {
                    replacement
                        .execute()
                        .map_err(|error| AppError::Io(error.to_string()))?;
                    lease.mark_step_applied(replacement.sequence)?;
                }
                set_group_status(db, &request.group_id, "resolved")
                    .await
                    .map_err(|error| AppError::Db(error.to_string()))?;
            }
            DurableResolutionAction::Ignore(_) => {
                persist_whitelist_pair(db, game_id, &request.folder_a, &request.folder_b)
                    .await
                    .map_err(|error| AppError::Db(error.to_string()))?;
                mark_group_ignored_after_last_pair(
                    db,
                    &request.group_id,
                    &mut pending_ignores_by_group,
                )
                .await
                .map_err(|error| AppError::Db(error.to_string()))?;
            }
        }
    }
    Ok(ResolutionSummary {
        total,
        successful: total,
        failed: 0,
        errors: Vec::new(),
    })
}

pub async fn resolve_batch<F>(
    requests: Vec<ResolutionRequest>,
    game_id: String,
    db: &SqlitePool,
    _op_guard: &crate::platform::fs::operation_lock::OpGuard,
    watcher_suppressor: &Arc<WatcherSuppressor>,
    mut on_progress: F,
) -> Result<ResolutionSummary, AppError>
where
    F: FnMut(ResolutionProgress),
{
    if requests.is_empty() {
        return Ok(ResolutionSummary {
            total: 0,
            successful: 0,
            failed: 0,
            errors: Vec::new(),
        });
    }

    validate_ignore_group_coverage(&requests, &game_id, db)
        .await
        .map_err(|error| AppError::Validation(error.to_string()))?;

    let _suppression_guard = SuppressionGuard::new(watcher_suppressor);

    let total = requests.len();
    let mut successful = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();
    let mut pending_ignores_by_group = count_pending_ignores(
        requests
            .iter()
            .filter(|request| matches!(request.action, ResolutionAction::Ignore)),
    );

    for (index, request) in requests.iter().enumerate() {
        on_progress(ResolutionProgress {
            current: index + 1,
            total,
            group_id: request.group_id.clone(),
            action: request.action.clone(),
        });

        let outcome: Result<(), ScannerError> = async {
            resolve_one(request, &game_id, db).await?;
            if matches!(request.action, ResolutionAction::Ignore) {
                mark_group_ignored_after_last_pair(
                    db,
                    &request.group_id,
                    &mut pending_ignores_by_group,
                )
                .await?;
            }
            Ok(())
        }
        .await;
        match outcome {
            Ok(()) => {
                successful += 1;
            }
            Err(message) => {
                failed += 1;
                let _ = set_group_status(db, &request.group_id, "partial").await;
                errors.push(ResolutionError {
                    group_id: request.group_id.clone(),
                    action: request.action.clone(),
                    message: message.to_string(),
                });
            }
        }
    }

    Ok(ResolutionSummary {
        total,
        successful,
        failed,
        errors,
    })
}

async fn resolve_one(
    request: &ResolutionRequest,
    game_id: &str,
    db: &SqlitePool,
) -> Result<(), ScannerError> {
    let confidence_score = authorize_request(request, game_id, db).await?;

    match request.action {
        ResolutionAction::KeepA => {
            if confidence_score == 100 {
                verify_full_folder_match(&request.folder_a, &request.folder_b)?;
            }
            move_folder_to_trash(&request.folder_b)?;
            set_group_status(db, &request.group_id, "resolved").await?;
            Ok(())
        }
        ResolutionAction::KeepB => {
            if confidence_score == 100 {
                verify_full_folder_match(&request.folder_a, &request.folder_b)?;
            }
            move_folder_to_trash(&request.folder_a)?;
            set_group_status(db, &request.group_id, "resolved").await?;
            Ok(())
        }
        ResolutionAction::Ignore => {
            persist_whitelist_pair(db, game_id, &request.folder_a, &request.folder_b).await?;
            Ok(())
        }
        ResolutionAction::Hardlink => {
            verify_full_folder_match(&request.folder_a, &request.folder_b)?;
            hardlink::apply_hardlinks(&request.folder_a, &request.folder_b)?;
            set_group_status(db, &request.group_id, "resolved").await?;
            Ok(())
        }
    }
}

fn count_pending_ignores<'a>(
    requests: impl Iterator<Item = &'a ResolutionRequest>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for request in requests {
        *counts.entry(request.group_id.clone()).or_default() += 1;
    }
    counts
}

async fn mark_group_ignored_after_last_pair(
    db: &SqlitePool,
    group_id: &str,
    pending_ignores_by_group: &mut BTreeMap<String, usize>,
) -> Result<(), ScannerError> {
    let remaining = pending_ignores_by_group.get_mut(group_id).ok_or_else(|| {
        ScannerError::Validation(format!(
            "Ignore resolution has no tracked group: {group_id}"
        ))
    })?;
    *remaining = remaining.checked_sub(1).ok_or_else(|| {
        ScannerError::Validation(format!(
            "Ignore resolution count underflow for group: {group_id}"
        ))
    })?;

    if *remaining == 0 {
        set_group_status(db, group_id, "ignored").await?;
    }

    Ok(())
}

async fn validate_ignore_group_coverage(
    requests: &[ResolutionRequest],
    game_id: &str,
    db: &SqlitePool,
) -> Result<(), ScannerError> {
    let ignore_group_ids = requests
        .iter()
        .filter(|request| matches!(request.action, ResolutionAction::Ignore))
        .map(|request| request.group_id.as_str())
        .collect::<BTreeSet<_>>();

    for group_id in ignore_group_ids {
        if requests.iter().any(|request| {
            request.group_id == group_id && !matches!(request.action, ResolutionAction::Ignore)
        }) {
            return Err(ScannerError::Validation(format!(
                "Ignore resolution cannot be mixed with another action for group: {group_id}"
            )));
        }

        let group = crate::modules::duplicates::adapters::sqlite::dedup::load_pending_group(
            db, game_id, group_id,
        )
        .await?
        .ok_or_else(|| {
            ScannerError::Validation(format!(
                "Duplicate group is missing, stale, or already resolved: {group_id}"
            ))
        })?;
        let member_paths = group_member_paths(&group)?;
        let expected_pairs = unique_member_pairs(&member_paths)?;
        let ignore_requests = requests
            .iter()
            .filter(|request| {
                request.group_id == group_id && matches!(request.action, ResolutionAction::Ignore)
            })
            .collect::<Vec<_>>();
        let supplied_pairs = ignore_requests
            .iter()
            .map(|request| canonical_request_paths(request).map(canonicalize_path_pair))
            .collect::<Result<BTreeSet<_>, _>>()?;

        if supplied_pairs.len() != ignore_requests.len() {
            return Err(ScannerError::Validation(format!(
                "Ignore resolution contains duplicate pairs for group: {group_id}"
            )));
        }
        if supplied_pairs != expected_pairs {
            return Err(ScannerError::Validation(format!(
                "Ignore resolution must include every unique member pair for group: {group_id}"
            )));
        }
    }

    Ok(())
}

fn unique_member_pairs(
    member_paths: &[PathBuf],
) -> Result<BTreeSet<(PathBuf, PathBuf)>, ScannerError> {
    let mut pairs = BTreeSet::new();
    for (index, left) in member_paths.iter().enumerate() {
        for right in member_paths.iter().skip(index + 1) {
            if left == right {
                return Err(ScannerError::Validation(
                    "Duplicate group contains the same physical folder more than once".to_string(),
                ));
            }
            pairs.insert(canonicalize_path_pair((left.clone(), right.clone())));
        }
    }

    if pairs.is_empty() {
        return Err(ScannerError::Validation(
            "Ignore resolution requires a duplicate group with at least two folders".to_string(),
        ));
    }

    Ok(pairs)
}

fn canonicalize_path_pair(paths: (PathBuf, PathBuf)) -> (PathBuf, PathBuf) {
    if paths.0 <= paths.1 {
        paths
    } else {
        (paths.1, paths.0)
    }
}

async fn authorize_request(
    request: &ResolutionRequest,
    game_id: &str,
    db: &SqlitePool,
) -> Result<u8, ScannerError> {
    let requested_paths = canonical_request_paths(request)?;
    let group = crate::modules::duplicates::adapters::sqlite::dedup::load_pending_group(
        db,
        game_id,
        &request.group_id,
    )
    .await?
    .ok_or_else(|| {
        ScannerError::Validation(format!(
            "Duplicate group is missing, stale, or already resolved: {}",
            request.group_id
        ))
    })?;
    let member_paths = group_member_paths(&group)?;
    if !member_paths.contains(&requested_paths.0) || !member_paths.contains(&requested_paths.1) {
        return Err(ScannerError::Validation(
            "Resolution paths are not members of the persisted duplicate group".to_string(),
        ));
    }
    if matches!(request.action, ResolutionAction::Hardlink) && group.confidence_score != 100 {
        return Err(ScannerError::Validation(
            "Hardlink resolution requires a fully verified exact duplicate group".to_string(),
        ));
    }
    Ok(group.confidence_score)
}

fn canonical_request_paths(
    request: &ResolutionRequest,
) -> Result<(PathBuf, PathBuf), ScannerError> {
    let left = canonical_folder_path(&request.folder_a)?;
    let right = canonical_folder_path(&request.folder_b)?;
    if left == right {
        return Err(ScannerError::Validation(
            "Duplicate resolution requires two distinct physical folders".to_string(),
        ));
    }
    Ok((left, right))
}

fn group_member_paths(
    group: &crate::modules::duplicates::domain::dup_scan::DupScanGroup,
) -> Result<Vec<PathBuf>, ScannerError> {
    group
        .members
        .iter()
        .map(|member| canonical_folder_path(&member.folder_path))
        .collect()
}

fn canonical_folder_path(folder_path: &str) -> Result<PathBuf, ScannerError> {
    std::fs::canonicalize(folder_path).map_err(|error| {
        ScannerError::Io(format!(
            "failed to resolve duplicate folder '{folder_path}': {error}"
        ))
    })
}

fn verify_full_folder_match(left: &str, right: &str) -> Result<(), ScannerError> {
    let left_manifest = full_folder_manifest(Path::new(left))?;
    let right_manifest = full_folder_manifest(Path::new(right))?;

    if left_manifest != right_manifest {
        return Err(ScannerError::Validation(
            "Folders are not full-content duplicates; rescan before resolving".to_string(),
        ));
    }

    Ok(())
}

fn full_folder_manifest(folder: &Path) -> Result<BTreeMap<String, (u64, String)>, ScannerError> {
    if !folder.is_dir() {
        return Err(ScannerError::Validation(format!(
            "Duplicate folder does not exist: {}",
            folder.display()
        )));
    }

    let mut manifest = BTreeMap::new();
    for item in walkdir::WalkDir::new(folder)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || (!entry.file_type().is_dir()
                    || !entry.file_name().to_string_lossy().starts_with('.'))
        })
    {
        let entry = item.map_err(|error| ScannerError::Io(error.to_string()))?;
        if entry.depth() == 0 || entry.file_type().is_dir() {
            continue;
        }
        if super::snapshot::is_ignored_file_name(entry.file_name()) {
            continue;
        }
        if !entry.file_type().is_file() {
            return Err(ScannerError::Validation(format!(
                "Unsupported link or special file in duplicate folder: {}",
                entry.path().display()
            )));
        }

        let relative = entry
            .path()
            .strip_prefix(folder)
            .map_err(|error| ScannerError::Io(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        let size = entry
            .metadata()
            .map_err(|error| ScannerError::Io(error.to_string()))?
            .len();
        let hash = super::hashing::full_blake3_hash(entry.path())?;
        manifest.insert(relative, (size, hash));
    }

    Ok(manifest)
}

fn move_folder_to_trash(folder_path: &str) -> Result<(), ScannerError> {
    let source_path = Path::new(folder_path);
    trash::move_to_trash(source_path).map_err(|error| ScannerError::Io(error.to_string()))
}

async fn persist_whitelist_pair(
    db: &SqlitePool,
    game_id: &str,
    folder_a_path: &str,
    folder_b_path: &str,
) -> Result<(), ScannerError> {
    let folder_a_id = fetch_mod_id(db, game_id, folder_a_path).await?;
    let folder_b_id = fetch_mod_id(db, game_id, folder_b_path).await?;

    if folder_a_id == folder_b_id {
        return Err(ScannerError::Validation(
            "Whitelist pair must reference two different folders".to_string(),
        ));
    }

    let (canonical_a, canonical_b) = canonicalize_pair(&folder_a_id, &folder_b_id);

    crate::modules::duplicates::adapters::sqlite::dedup::insert_whitelist_pair(
        db,
        game_id,
        canonical_a,
        canonical_b,
    )
    .await?;

    Ok(())
}

async fn fetch_mod_id(
    db: &SqlitePool,
    game_id: &str,
    folder_path: &str,
) -> Result<String, ScannerError> {
    crate::modules::library::adapters::sqlite::mods::get_mod_id_and_status_by_path(
        db,
        folder_path,
        game_id,
    )
    .await?
    .map(|(id, _, _)| id)
    .ok_or_else(|| {
        ScannerError::Validation(format!(
            "mod entry not found for game '{game_id}' and folder '{folder_path}'"
        ))
    })
}

async fn set_group_status(
    db: &SqlitePool,
    group_id: &str,
    status: &str,
) -> Result<(), ScannerError> {
    let set_resolved_at = status == "resolved" || status == "ignored";
    let rows_affected = crate::modules::duplicates::adapters::sqlite::dedup::update_group_status(
        db,
        group_id,
        status,
        set_resolved_at,
    )
    .await?;

    if rows_affected == 0 {
        return Err(ScannerError::Validation(format!(
            "Duplicate group is missing or stale: {group_id}"
        )));
    }

    Ok(())
}

fn canonicalize_pair<'a>(left: &'a str, right: &'a str) -> (&'a str, &'a str) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

#[cfg(test)]
#[path = "tests/dedup_resolver_tests.rs"]
mod tests;
