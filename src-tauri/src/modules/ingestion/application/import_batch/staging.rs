use super::types::{ImportBatch, ImportBatchStatus, ImportItemStatus};
use crate::modules::ingestion::adapters::sqlite::import_batch::{self, StagedRootRecord};
use crate::modules::library::application::mods::archive::{ExtractionEvent, StagingExtractOptions};
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RootPackageGroup {
    root: PathBuf,
    is_bundle: bool,
}

const DEFERRED_CLEANUP_DIRECTORY: &str = ".cleanup";
const FOLDER_COPY_BUFFER_BYTES: usize = 64 * 1024;

/// Detach startup garbage before import commands can create unreferenced attempts.
/// The event-loop worker only deletes detached payloads, never live staging.
pub fn prepare_startup_staging_cleanup(
    staging_root: &Path,
    terminal_batch_ids: &[String],
    referenced_staging_paths: &[String],
) -> Result<usize, AppError> {
    if !staging_root.exists() {
        return Ok(0);
    }
    let root = staging_root.canonicalize()?;
    let retained_attempts = referenced_attempt_directories(&root, referenced_staging_paths)?;
    let terminal_ids = terminal_batch_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut garbage = Vec::new();
    for batch in owned_uuid_directories(&root)? {
        let batch_id = batch.file_name().unwrap_or_default().to_string_lossy();
        if terminal_ids.contains(batch_id.as_ref())
            && !retained_attempts
                .iter()
                .any(|attempt| attempt.starts_with(&batch))
        {
            garbage.push(batch);
            continue;
        }
        for item in owned_uuid_directories(&batch)? {
            for attempt in owned_uuid_directories(&item)? {
                if !retained_attempts.contains(&attempt) {
                    garbage.push(attempt);
                }
            }
        }
    }
    if garbage.is_empty() {
        return Ok(0);
    }
    let cleanup = root.join(DEFERRED_CLEANUP_DIRECTORY);
    std::fs::create_dir_all(&cleanup)?;
    let cleanup = validated_cleanup_directory(&root)?;
    let mut empty_parents = BTreeSet::new();
    for source in &garbage {
        let destination = cleanup.join(uuid::Uuid::new_v4().to_string());
        crate::platform::fs::rename::rename_no_replace(source, &destination)?;
        if let Some(item) = source.parent().filter(|parent| *parent != root) {
            empty_parents.insert(item.to_path_buf());
            if let Some(batch) = item.parent().filter(|parent| *parent != root) {
                empty_parents.insert(batch.to_path_buf());
            }
        }
    }
    for parent in empty_parents.iter().rev() {
        remove_dir_if_empty(parent)?;
    }
    Ok(garbage.len())
}

pub fn cleanup_deferred_staging(staging_root: &Path) -> Result<usize, AppError> {
    if !staging_root.exists() {
        return Ok(0);
    }
    let root = staging_root.canonicalize()?;
    if !root.join(DEFERRED_CLEANUP_DIRECTORY).exists() {
        return Ok(0);
    }
    let cleanup = validated_cleanup_directory(&root)?;
    let payloads = owned_uuid_directories(&cleanup)?;
    for payload in &payloads {
        std::fs::remove_dir_all(payload)?;
    }
    remove_dir_if_empty(&cleanup)?;
    Ok(payloads.len())
}

fn validated_cleanup_directory(root: &Path) -> Result<PathBuf, AppError> {
    let path = root.join(DEFERRED_CLEANUP_DIRECTORY);
    let metadata = std::fs::symlink_metadata(&path)?;
    let canonical = path.canonicalize()?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || canonical.parent() != Some(root) {
        return Err(AppError::Security(
            "Deferred staging cleanup directory escaped its owned root".to_string(),
        ));
    }
    Ok(canonical)
}

pub fn cleanup_batch_staging(staging_root: &Path, batch_id: &str) -> Result<bool, AppError> {
    if uuid::Uuid::parse_str(batch_id).is_err() || !staging_root.exists() {
        return Ok(false);
    }
    let root = staging_root.canonicalize()?;
    let target = root.join(batch_id);
    if !target.exists() {
        return Ok(false);
    }
    let canonical_target = target.canonicalize()?;
    if canonical_target.parent() != Some(root.as_path()) {
        return Err(AppError::Security(
            "Import staging cleanup target escaped its owned root".to_string(),
        ));
    }
    std::fs::remove_dir_all(canonical_target)?;
    Ok(true)
}

fn referenced_attempt_directories(
    root: &Path,
    referenced_staging_paths: &[String],
) -> Result<HashSet<PathBuf>, AppError> {
    let mut attempts = HashSet::new();

    for staging_path in referenced_staging_paths {
        let path = Path::new(staging_path);
        if !path.exists() {
            continue;
        }

        let canonical_path = path.canonicalize()?;
        let relative = canonical_path.strip_prefix(root).map_err(|_| {
            AppError::Security("Import staging reference escaped its owned root".to_string())
        })?;
        let components = relative.components().collect::<Vec<_>>();
        let [std::path::Component::Normal(batch_id), std::path::Component::Normal(item_id), std::path::Component::Normal(attempt_id), std::path::Component::Normal(kind), ..] =
            components.as_slice()
        else {
            return Err(AppError::Security(
                "Import staging reference has an invalid owned layout".to_string(),
            ));
        };
        if uuid::Uuid::parse_str(&batch_id.to_string_lossy()).is_err()
            || uuid::Uuid::parse_str(&item_id.to_string_lossy()).is_err()
            || uuid::Uuid::parse_str(&attempt_id.to_string_lossy()).is_err()
            || !matches!(kind.to_str(), Some("extracted" | "roots"))
        {
            return Err(AppError::Security(
                "Import staging reference has an invalid owned layout".to_string(),
            ));
        }

        let attempt = root
            .join(batch_id)
            .join(item_id)
            .join(attempt_id)
            .canonicalize()?;
        attempts.insert(attempt);
    }

    Ok(attempts)
}

fn owned_uuid_directories(parent: &Path) -> Result<Vec<PathBuf>, AppError> {
    std::fs::read_dir(parent)?
        .filter_map(|entry| match entry {
            Ok(entry)
                if uuid::Uuid::parse_str(&entry.file_name().to_string_lossy()).is_ok()
                    && entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) =>
            {
                Some(Ok(entry))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .map(|entry| {
            let entry = entry?;
            let canonical = entry.path().canonicalize()?;
            if canonical.parent() != Some(parent) {
                return Err(AppError::Security(
                    "Import staging directory escaped its owned root".to_string(),
                ));
            }
            Ok(canonical)
        })
        .collect()
}

fn remove_dir_if_empty(path: &Path) -> Result<(), AppError> {
    if std::fs::read_dir(path)?.next().is_none() {
        std::fs::remove_dir(path)?;
    }
    Ok(())
}

fn cleanup_item_staging(
    staging_root: &Path,
    batch_id: &str,
    item_id: &str,
) -> Result<bool, AppError> {
    if uuid::Uuid::parse_str(batch_id).is_err()
        || uuid::Uuid::parse_str(item_id).is_err()
        || !staging_root.exists()
    {
        return Ok(false);
    }
    let root = staging_root.canonicalize()?;
    let batch = root.join(batch_id);
    let target = batch.join(item_id);
    if !target.exists() {
        return Ok(false);
    }
    let canonical_batch = batch.canonicalize()?;
    let canonical_target = target.canonicalize()?;
    if canonical_batch.parent() != Some(root.as_path())
        || canonical_target.parent() != Some(canonical_batch.as_path())
    {
        return Err(AppError::Security(
            "Import item staging cleanup target escaped its owned root".to_string(),
        ));
    }
    std::fs::remove_dir_all(canonical_target)?;
    Ok(true)
}

#[cfg(test)]
pub async fn stage_import_batch_sources(
    db: &SqlitePool,
    batch_id: &str,
    staging_root: &Path,
) -> Result<ImportBatch, AppError> {
    stage_import_batch_sources_with_options(
        db,
        batch_id,
        staging_root,
        &StagingExtractOptions::default(),
    )
    .await
}

pub async fn stage_import_batch_sources_with_options(
    db: &SqlitePool,
    batch_id: &str,
    staging_root: &Path,
    options: &StagingExtractOptions,
) -> Result<ImportBatch, AppError> {
    let batch = import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    if !matches!(
        batch.status,
        ImportBatchStatus::Draft | ImportBatchStatus::Failed | ImportBatchStatus::Partial
    ) {
        return Err(AppError::Validation(format!(
            "Import batch '{batch_id}' cannot be staged from status {:?}",
            batch.status
        )));
    }
    std::fs::create_dir_all(staging_root)?;
    import_batch::set_batch_status(db, batch_id, ImportBatchStatus::Analyzing).await?;
    // Keep archive duplicates pending until every representative has staged
    // successfully. A broken representative must never make a duplicate look
    // like it was successfully imported.
    let mut archive_representatives = BTreeMap::<(u64, String), String>::new();
    let mut archive_duplicates = Vec::<(String, String, String)>::new();

    for item in batch.items.iter().filter(|item| {
        matches!(
            item.status,
            ImportItemStatus::Discovered | ImportItemStatus::Failed
        )
    }) {
        let outcome: Result<(), AppError> = async {
            check_staging_cancelled(options.cancel_token.as_deref())?;
            cleanup_item_staging(staging_root, batch_id, &item.id)?;
            if item.status == ImportItemStatus::Failed
                && !import_batch::reset_failed_item_for_staging(db, &item.id).await?
            {
                return Err(AppError::Validation(format!(
                    "Import item '{}' changed while preparing a staging retry",
                    item.id
                )));
            }
            let source = PathBuf::from(&item.source_path);
            if source.is_dir() {
                let source_for_validation = source.clone();
                let cancellation = options.cancel_token.clone();
                tokio::task::spawn_blocking(move || {
                    super::payload_manifest::validate_import_payload_tree(
                        &source_for_validation,
                        cancellation.as_deref(),
                    )
                })
                .await??;
                if batch.flow == super::types::ImportFlow::ReadyToMove {
                    stage_ready_to_move_folder(
                        db,
                        batch_id,
                        &item.id,
                        &source,
                        staging_root,
                        options,
                    )
                    .await
                } else {
                    transition_folder_to_staged(db, &item.id).await
                }
            } else if source.is_file() {
                let archive_path = source.clone();
                let cancellation = options.cancel_token.clone();
                let archive_sha256 = tokio::task::spawn_blocking(move || {
                    super::payload_manifest::sha256_file(&archive_path, cancellation.as_deref())
                })
                .await??;
                let archive_size = source.metadata()?.len();
                let archive_key = (archive_size, archive_sha256.clone());
                if let Some(representative_item_id) = archive_representatives.get(&archive_key) {
                    archive_duplicates.push((
                        item.id.clone(),
                        representative_item_id.clone(),
                        archive_sha256,
                    ));
                    return Ok(());
                }
                archive_representatives.insert(archive_key, item.id.clone());
                if !import_batch::store_archive_hash(db, &item.id, &archive_sha256).await? {
                    return Err(AppError::Validation(format!(
                        "Import item '{}' changed while archive hashing was running",
                        item.id
                    )));
                }
                stage_archive_item(
                    db,
                    batch_id,
                    &item.id,
                    &source,
                    &item.planned_name,
                    staging_root,
                    options,
                )
                .await
            } else {
                Err(AppError::Validation(format!(
                    "Import source disappeared before staging: {}",
                    source.display()
                )))
            }
        }
        .await;

        if let Err(error) = outcome {
            if matches!(error, AppError::Cancelled) {
                return Err(error);
            }
            let persisted_error = match &error {
                AppError::ArchiveUnsupported { reason } => reason.storage_code().to_string(),
                _ => error.to_string(),
            };
            import_batch::set_item_failure(db, &item.id, &persisted_error).await?;
            import_batch::set_batch_status(db, batch_id, ImportBatchStatus::Partial).await?;
            return Err(error);
        }
    }

    for (item_id, representative_item_id, archive_sha256) in archive_duplicates {
        if !import_batch::mark_archive_duplicate(
            db,
            &item_id,
            &representative_item_id,
            &archive_sha256,
        )
        .await?
        {
            return Err(AppError::Validation(format!(
                "Import item '{item_id}' changed while duplicate archive analysis was running"
            )));
        }
    }

    import_batch::set_batch_status(db, batch_id, ImportBatchStatus::AwaitingReview).await?;
    import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::Internal("Staged import batch could not be reloaded".to_string()))
}

async fn transition_folder_to_staged(db: &SqlitePool, item_id: &str) -> Result<(), AppError> {
    let updated = import_batch::transition_item_status(
        db,
        item_id,
        ImportItemStatus::Discovered,
        ImportItemStatus::Staged,
    )
    .await?;
    updated.then_some(()).ok_or_else(|| {
        AppError::Validation(format!("Import item '{item_id}' changed while staging"))
    })
}

async fn stage_ready_to_move_folder(
    db: &SqlitePool,
    batch_id: &str,
    item_id: &str,
    source: &Path,
    staging_root: &Path,
    options: &StagingExtractOptions,
) -> Result<(), AppError> {
    let source_for_worker = source.to_path_buf();
    let item_staging = staging_root
        .join(batch_id)
        .join(item_id)
        .join(uuid::Uuid::new_v4().to_string())
        .join("roots");
    let worker_options = options.clone();
    let records = tokio::task::spawn_blocking(move || {
        let cancellation = worker_options.cancel_token.as_deref();
        let mut roots = crate::modules::library::application::mods::archive::classify::find_mod_roots_with_limits(
            &source_for_worker,
            crate::modules::library::application::mods::archive::classify::MOD_ROOT_MAX_DEPTH,
            crate::modules::library::application::mods::archive::classify::MOD_ROOT_MAX_ENTRIES,
            cancellation,
        )?.roots;
        roots.sort();
        if roots.is_empty() || (roots.len() == 1 && roots[0] == source_for_worker) {
            return Ok(Vec::new());
        }

        let groups = group_mod_roots(&source_for_worker, &roots, cancellation)?;
        std::fs::create_dir_all(&item_staging)?;
        groups
            .into_iter()
            .enumerate()
            .map(|(index, group)| {
                check_staging_cancelled(cancellation)?;
                let root_name = if group.is_bundle {
                    source_for_worker
                        .file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                } else {
                    group
                        .root
                        .file_name()
                        .map(|value| value.to_string_lossy().into_owned())
                }
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| format!("mod-{}", index + 1));
                let target = item_staging.join(format!("{}-{}", index + 1, root_name));
                std::fs::create_dir_all(&target)?;
                copy_folder_payload(&group.root, &target, &worker_options)?;
                Ok(StagedRootRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    staging_path: target.to_string_lossy().into_owned(),
                    planned_name:
                        crate::modules::library::application::mods::core_ops::standardize_prefix(
                            &root_name, false,
                        ),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()
    })
    .await??;

    check_staging_cancelled(options.cancel_token.as_deref())?;
    if records.is_empty() {
        return transition_folder_to_staged(db, item_id).await;
    }
    if !import_batch::replace_archive_item_with_roots(db, item_id, &records).await? {
        return Err(AppError::Validation(format!(
            "Import item '{item_id}' changed while folder-pack staging completed"
        )));
    }
    Ok(())
}

async fn stage_archive_item(
    db: &SqlitePool,
    batch_id: &str,
    item_id: &str,
    source: &Path,
    fallback_name: &str,
    staging_root: &Path,
    options: &StagingExtractOptions,
) -> Result<(), AppError> {
    let extract_dir = staging_root
        .join(batch_id)
        .join(item_id)
        .join(uuid::Uuid::new_v4().to_string())
        .join("extracted");
    let archive = source.to_path_buf();
    let extract_for_worker = extract_dir.clone();
    let cancellation = options.cancel_token.clone();
    let worker_options = options.clone();
    let staged = tokio::task::spawn_blocking(move || {
        crate::modules::library::application::mods::archive::extract_archive_to_staging_with_options(
            &archive,
            &extract_for_worker,
            worker_options,
        )
    })
    .await??;
    let unreadable_ini_files = staged.unreadable_ini_files;
    let mut roots = staged.mod_roots;
    roots.sort();
    let records = group_mod_roots(&extract_dir, &roots, cancellation.as_deref())?
        .into_iter()
        .map(|group| {
            let planned_name = if group.is_bundle || group.root == extract_dir {
                fallback_name.to_string()
            } else {
                group
                    .root
                    .file_name()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_else(|| fallback_name.to_string())
            };
            let planned_name =
                crate::modules::library::application::mods::core_ops::standardize_prefix(
                    &planned_name,
                    false,
                );
            StagedRootRecord {
                id: uuid::Uuid::new_v4().to_string(),
                staging_path: group.root.to_string_lossy().into_owned(),
                planned_name,
            }
        })
        .collect::<Vec<_>>();
    if !import_batch::replace_archive_item_with_roots(db, item_id, &records).await? {
        return Err(AppError::Validation(format!(
            "Import item '{item_id}' changed while archive staging completed"
        )));
    }
    if unreadable_ini_files > 0 {
        let diagnostic = super::types::ImportDiagnostic {
            code: "ini_decode_incomplete".to_string(),
            stage: super::types::ImportDiagnosticStage::RootDiscovery,
            member_path: None,
            recovery: format!(
                "{unreadable_ini_files} INI file(s) could not be decoded completely; confirm this package manually"
            ),
        };
        for root_item_id in
            std::iter::once(item_id).chain(records.iter().skip(1).map(|root| root.id.as_str()))
        {
            if !import_batch::append_item_diagnostic(db, root_item_id, &diagnostic).await? {
                return Err(AppError::Validation(format!(
                    "Import item '{root_item_id}' changed while recording incomplete INI inspection"
                )));
            }
        }
    }
    Ok(())
}

fn copy_folder_payload(
    source: &Path,
    destination: &Path,
    options: &StagingExtractOptions,
) -> Result<(), AppError> {
    let cancellation = options.cancel_token.as_deref();
    let total_files = options
        .on_progress
        .as_ref()
        .map(|_| super::payload_manifest::payload_manifest_metadata(source, cancellation))
        .transpose()?
        .map_or(0, |metadata| metadata.file_count as usize);
    let mut file_index = 0;
    for entry in walkdir::WalkDir::new(source)
        .follow_links(false)
        .sort_by_file_name()
    {
        check_staging_cancelled(cancellation)?;
        let entry = entry.map_err(|error| AppError::Io(error.to_string()))?;
        let relative = entry.path().strip_prefix(source).map_err(|error| {
            AppError::Security(format!("Folder-pack entry escaped its source: {error}"))
        })?;
        let target = destination.join(relative);
        if entry.file_type().is_symlink() {
            return Err(AppError::Security(format!(
                "Folder-pack copy cannot follow symbolic links: {}",
                entry.path().display()
            )));
        }
        if entry.file_type().is_dir() {
            fs::create_dir_all(target)?;
        } else if entry.file_type().is_file() {
            let mut input = fs::File::open(entry.path())?;
            let metadata = input.metadata()?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)?;
            copy_payload_bytes(&mut input, &mut output, cancellation)?;
            output.set_times(fs::FileTimes::new().set_modified(metadata.modified()?))?;
            output.set_permissions(metadata.permissions())?;
            file_index += 1;
            if let Some(on_progress) = &options.on_progress {
                let _ = on_progress.send(ExtractionEvent::FileProgress {
                    file_name: relative.to_string_lossy().into_owned(),
                    file_index,
                    total_files,
                });
            }
        } else {
            return Err(AppError::Validation(format!(
                "Folder-pack copy encountered an unsupported filesystem entry: {}",
                entry.path().display()
            )));
        }
    }
    check_staging_cancelled(cancellation)
}

fn copy_payload_bytes<R: Read, W: Write>(
    input: &mut R,
    output: &mut W,
    cancellation: Option<&AtomicBool>,
) -> Result<(), AppError> {
    let mut buffer = [0_u8; FOLDER_COPY_BUFFER_BYTES];
    loop {
        check_staging_cancelled(cancellation)?;
        let read = match input.read(&mut buffer) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        check_staging_cancelled(cancellation)?;
        if read == 0 {
            return Ok(());
        }
        output.write_all(&buffer[..read])?;
    }
}

fn check_staging_cancelled(cancellation: Option<&AtomicBool>) -> Result<(), AppError> {
    if cancellation.is_some_and(|token| token.load(Ordering::SeqCst)) {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

fn group_mod_roots(
    package_root: &Path,
    roots: &[PathBuf],
    cancellation: Option<&AtomicBool>,
) -> Result<Vec<RootPackageGroup>, AppError> {
    check_staging_cancelled(cancellation)?;
    let canonical_package = package_root.canonicalize()?;
    for root in roots {
        check_staging_cancelled(cancellation)?;
        if !root.canonicalize()?.starts_with(&canonical_package) {
            return Err(AppError::Security(
                "Discovered mod root escaped its source package".to_string(),
            ));
        }
    }
    let requires_grouping = roots
        .iter()
        .map(|root| root_has_external_or_dynamic_reference(root, cancellation))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|requires_grouping| requires_grouping);
    if requires_grouping {
        // Shared assets can sit above the discovered roots' common parent.
        // Keep the selected package boundary without resolving arbitrary references.
        return Ok(vec![RootPackageGroup {
            root: package_root.to_path_buf(),
            is_bundle: true,
        }]);
    }
    Ok(roots
        .iter()
        .cloned()
        .map(|root| RootPackageGroup {
            root,
            is_bundle: false,
        })
        .collect())
}

fn root_has_external_or_dynamic_reference(
    root: &Path,
    cancellation: Option<&AtomicBool>,
) -> Result<bool, AppError> {
    let mut pending = vec![root.to_path_buf()];
    let mut visited_entries = 0_usize;
    while let Some(directory) = pending.pop() {
        check_staging_cancelled(cancellation)?;
        for entry in fs::read_dir(&directory)? {
            check_staging_cancelled(cancellation)?;
            let entry = entry?;
            visited_entries += 1;
            if visited_entries > crate::modules::library::application::mods::archive::classify::MOD_ROOT_MAX_ENTRIES {
                return Ok(true);
            }
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                return Ok(true);
            }
            let path = entry.path();
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            if !file_type.is_file()
                || !path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"))
            {
                continue;
            }
            let mut input = fs::File::open(path)?;
            let mut bytes = Vec::new();
            copy_payload_bytes(&mut input, &mut bytes, cancellation)?;
            let (contents, _, clean) =
                crate::modules::library::application::ini::document::decode_ini_bytes(&bytes);
            if !clean || ini_has_external_or_dynamic_reference(&contents) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ini_has_external_or_dynamic_reference(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            return false;
        }
        let Some((key, raw_value)) = line.split_once('=') else {
            return false;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = raw_value
            .split([';', '#'])
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches(['\"', '\'']);
        let is_reference = key.contains("include")
            || key.contains("resource")
            || key.contains("filename")
            || key.contains("file");
        is_reference
            && (value.contains("../")
                || value.contains("..\\")
                || value.contains('$')
                || value.contains('%'))
    })
}

#[cfg(test)]
mod copy_tests {
    use super::{copy_payload_bytes, FOLDER_COPY_BUFFER_BYTES};
    use crate::shared::errors::AppError;
    use std::io::Read;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn cancellation_during_a_file_read_stops_within_one_copy_chunk() {
        struct CancellingReader<'a> {
            remaining: usize,
            cancellation: &'a AtomicBool,
        }

        impl Read for CancellingReader<'_> {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                let read = self.remaining.min(buffer.len());
                buffer[..read].fill(7);
                self.remaining -= read;
                self.cancellation.store(true, Ordering::SeqCst);
                Ok(read)
            }
        }

        let cancellation = AtomicBool::new(false);
        let source_bytes = 3 * FOLDER_COPY_BUFFER_BYTES + 7;
        let mut input = CancellingReader {
            remaining: source_bytes,
            cancellation: &cancellation,
        };
        let mut output = Vec::new();

        let result = copy_payload_bytes(&mut input, &mut output, Some(&cancellation));

        assert!(matches!(result, Err(AppError::Cancelled)));
        assert_eq!(input.remaining, source_bytes - FOLDER_COPY_BUFFER_BYTES);
        assert!(output.is_empty(), "cancelled bytes must not be written");
    }
}

#[cfg(test)]
mod cleanup_tests {
    use std::path::Path;

    use super::{
        cleanup_batch_staging, cleanup_deferred_staging, group_mod_roots,
        prepare_startup_staging_cleanup,
    };

    #[test]
    fn startup_detaches_garbage_without_deleting_payloads_or_active_imports() {
        let root = tempfile::tempdir().unwrap();
        let terminal_id = uuid::Uuid::new_v4().to_string();
        let terminal = root.path().join(&terminal_id);
        std::fs::create_dir_all(&terminal).unwrap();
        std::fs::write(terminal.join("terminal.txt"), "terminal").unwrap();
        let item = root
            .path()
            .join(uuid::Uuid::new_v4().to_string())
            .join(uuid::Uuid::new_v4().to_string());
        let active = item
            .join(uuid::Uuid::new_v4().to_string())
            .join("roots/Mod");
        let orphan = item.join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&orphan).unwrap();
        std::fs::write(orphan.join("orphan.txt"), "orphan").unwrap();

        assert_eq!(
            prepare_startup_staging_cleanup(
                root.path(),
                &[terminal_id],
                &[active.to_string_lossy().into_owned()]
            )
            .unwrap(),
            2
        );
        assert!(active.exists());
        assert!(!terminal.exists());
        assert!(!orphan.exists());
        let payloads = std::fs::read_dir(root.path().join(".cleanup"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(payloads.len(), 2);
        assert!(payloads
            .iter()
            .any(|path| path.join("terminal.txt").exists()));
        assert!(payloads.iter().any(|path| path.join("orphan.txt").exists()));

        let fresh = item
            .join(uuid::Uuid::new_v4().to_string())
            .join("extracted/New");
        std::fs::create_dir_all(&fresh).unwrap();
        assert_eq!(cleanup_deferred_staging(root.path()).unwrap(), 2);
        assert!(
            fresh.exists(),
            "new, not-yet-persisted extractions must survive"
        );
        assert!(active.exists());
    }

    #[test]
    fn deferred_staging_cleanup_retries_payloads_left_by_a_previous_shutdown() {
        let root = tempfile::tempdir().unwrap();
        let terminal_id = uuid::Uuid::new_v4().to_string();
        let terminal = root.path().join(&terminal_id);
        std::fs::create_dir_all(&terminal).unwrap();
        std::fs::write(terminal.join("payload.txt"), "payload").unwrap();
        prepare_startup_staging_cleanup(root.path(), &[terminal_id], &[]).unwrap();
        assert_eq!(
            prepare_startup_staging_cleanup(root.path(), &[], &[]).unwrap(),
            0
        );
        assert_eq!(cleanup_deferred_staging(root.path()).unwrap(), 1);
        assert_eq!(cleanup_deferred_staging(root.path()).unwrap(), 0);
    }

    #[test]
    #[ignore = "manual comparison of startup detach and recursive staging deletion"]
    fn benchmark_staging_detach_versus_recursive_cleanup() {
        fn payload(root: &Path) -> String {
            let batch_id = uuid::Uuid::new_v4().to_string();
            let batch = root.join(&batch_id);
            for directory in 0..50 {
                let path = batch.join(directory.to_string());
                std::fs::create_dir_all(&path).unwrap();
                for file in 0..20 {
                    std::fs::write(path.join(format!("{file}.buf")), [0_u8; 128]).unwrap();
                }
            }
            batch_id
        }
        let mut recursive_us = Vec::new();
        let mut detach_us = Vec::new();
        let mut deferred_us = Vec::new();
        for _ in 0..3 {
            let old_root = tempfile::tempdir().unwrap();
            let new_root = tempfile::tempdir().unwrap();
            let old_batch = payload(old_root.path());
            let new_batch = payload(new_root.path());
            let started = std::time::Instant::now();
            assert!(cleanup_batch_staging(old_root.path(), &old_batch).unwrap());
            recursive_us.push(started.elapsed().as_micros());
            let started = std::time::Instant::now();
            assert_eq!(
                prepare_startup_staging_cleanup(new_root.path(), &[new_batch], &[]).unwrap(),
                1
            );
            detach_us.push(started.elapsed().as_micros());
            let started = std::time::Instant::now();
            assert_eq!(cleanup_deferred_staging(new_root.path()).unwrap(), 1);
            deferred_us.push(started.elapsed().as_micros());
        }
        println!("staging payload_files=1000 recursive_us={recursive_us:?} startup_detach_us={detach_us:?} deferred_delete_us={deferred_us:?}");
    }

    #[test]
    fn startup_staging_cleanup_validates_references_before_detaching_anything() {
        let root = tempfile::tempdir().unwrap();
        let foreign = tempfile::tempdir().unwrap();
        let terminal_id = uuid::Uuid::new_v4().to_string();
        let terminal = root.path().join(&terminal_id);
        std::fs::create_dir_all(&terminal).unwrap();
        assert!(prepare_startup_staging_cleanup(
            root.path(),
            &[terminal_id],
            &[foreign.path().to_string_lossy().into_owned()]
        )
        .is_err());
        assert!(terminal.exists());
    }

    #[test]
    fn cleanup_removes_only_a_uuid_owned_batch_directory() {
        let root = tempfile::tempdir().unwrap();
        let batch_id = uuid::Uuid::new_v4().to_string();
        let owned = root.path().join(&batch_id);
        let sibling = root.path().join("keep-me");
        std::fs::create_dir_all(owned.join("item/extracted")).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();

        assert!(cleanup_batch_staging(root.path(), &batch_id).unwrap());
        assert!(!owned.exists());
        assert!(sibling.exists());
        assert!(!cleanup_batch_staging(root.path(), "../keep-me").unwrap());
        assert!(sibling.exists());
    }

    #[test]
    fn startup_detach_preserves_referenced_attempts_even_with_a_stale_terminal_id() {
        let root = tempfile::tempdir().unwrap();
        let batch_id = uuid::Uuid::new_v4().to_string();
        let item_id = uuid::Uuid::new_v4().to_string();
        let retained_attempt = root
            .path()
            .join(&batch_id)
            .join(&item_id)
            .join(uuid::Uuid::new_v4().to_string());
        let orphaned_attempt = root
            .path()
            .join(&batch_id)
            .join(&item_id)
            .join(uuid::Uuid::new_v4().to_string());
        let retained_root = retained_attempt.join("extracted/Mod");
        std::fs::create_dir_all(&retained_root).unwrap();
        std::fs::create_dir_all(orphaned_attempt.join("extracted")).unwrap();

        let removed = prepare_startup_staging_cleanup(
            root.path(),
            &[batch_id],
            &[retained_root.to_string_lossy().into_owned()],
        )
        .unwrap();

        assert_eq!(removed, 1);
        assert!(retained_attempt.exists());
        assert!(!orphaned_attempt.exists());
    }

    #[test]
    fn dependent_roots_remain_in_one_bundle() {
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("body");
        let right = root.path().join("face");
        std::fs::create_dir_all(&left).unwrap();
        std::fs::create_dir_all(&right).unwrap();
        std::fs::write(
            left.join("merged.ini"),
            "include = ../face/merged.ini\n[TextureOverride]\nhash = abc\n",
        )
        .unwrap();
        std::fs::write(right.join("merged.ini"), "[TextureOverride]\nhash = def\n").unwrap();

        let groups = group_mod_roots(root.path(), &[left, right], None).unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].root, root.path());
        assert!(groups[0].is_bundle);
    }

    #[test]
    fn grouping_rejects_roots_outside_the_selected_package() {
        let package = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let error =
            group_mod_roots(package.path(), &[outside.path().to_path_buf()], None).unwrap_err();
        assert!(matches!(
            error,
            crate::shared::errors::AppError::Security(_)
        ));
    }

    #[test]
    fn dynamic_single_root_preserves_package_without_following_external_references() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("package");
        let body = package.join("Body");
        let outside = root.path().join("outside.dds");
        std::fs::create_dir_all(&body).unwrap();
        std::fs::write(&outside, "outside").unwrap();
        std::fs::write(
            body.join("merged.ini"),
            "filename = $texture\ninclude = ../../outside.dds\n",
        )
        .unwrap();

        let groups = group_mod_roots(&package, &[body], None).unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].root, package);
        assert!(groups[0].is_bundle);
        assert!(!groups[0].root.join("outside.dds").exists());
        assert!(outside.exists());
    }
}
