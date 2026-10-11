use crate::modules::library::application::mods::core_ops::standardize_prefix;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use crate::platform::fs::file_utils::FilesystemIdentityProof;
use crate::platform::fs::guard::ValidatedPath;
use crate::platform::fs::rename::PinnedDirectory;
use crate::shared::errors::AppError;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

pub struct MoveModsToObjectParams<'a> {
    pub game_id: &'a str,
    pub folder_paths: &'a [ValidatedPath],
    pub target_object_id: &'a str,
    pub target_subpath: Option<&'a str>,
    pub status: Option<&'a str>,
}

/// Exact filesystem transitions that terminal reconcile must apply in its
/// projection transaction. The organizer never writes these paths to SQLite.
#[derive(Debug, Clone)]
pub struct OrganizerMovePathHint {
    pub old_path: String,
    pub new_path: String,
    pub target_object_id: String,
}

pub struct OrganizerMoveOutcome {
    pub result: crate::modules::library::application::mods::bulk::BulkResult,
    pub path_hints: Vec<OrganizerMovePathHint>,
}

#[derive(Debug, Clone)]
struct PreparedOrganizerRename {
    source_proof: FilesystemIdentityProof,
    old_path: PathBuf,
    new_path: PathBuf,
    old_rel: String,
    new_rel: String,
    target_object_id: String,
}

#[derive(Debug, Clone)]
pub struct PreparedOrganizerMove {
    namespace_proof: crate::platform::fs::file_utils::FilesystemNamespaceProof,
    target_namespace_proof: crate::platform::fs::file_utils::FilesystemNamespaceProof,
    target_scope_path: PathBuf,
    target_anchor: PathBuf,
    target_proof: FilesystemIdentityProof,
    renames: Vec<PreparedOrganizerRename>,
    primary_results: Vec<String>,
    target_base_path: PathBuf,
    target_base_requires_creation: bool,
}

impl PreparedOrganizerMove {
    pub fn is_rolled_back(&self) -> bool {
        self.renames
            .iter()
            .all(|rename| rename.source_proof.validate(&rename.old_path).is_ok())
    }
    pub fn durable_journal_steps(&self) -> Vec<crate::modules::mutation::api::PlannedStep> {
        self.renames
            .iter()
            .enumerate()
            .map(|(sequence, rename)| {
                crate::modules::mutation::api::PlannedStep::rename(
                    sequence as u32,
                    rename.old_path.clone(),
                    rename.new_path.clone(),
                )
                .with_expected_identity(Some(rename.source_proof.identity().to_string()))
            })
            .collect()
    }
    pub fn journal_steps(&self) -> Vec<(u32, PathBuf, PathBuf)> {
        self.renames
            .iter()
            .enumerate()
            .map(|(sequence, rename)| {
                (
                    sequence as u32,
                    rename.old_path.clone(),
                    rename.new_path.clone(),
                )
            })
            .collect()
    }

    pub fn changed_paths(&self) -> Vec<String> {
        self.renames
            .iter()
            .flat_map(|rename| {
                [
                    rename.old_path.to_string_lossy().into_owned(),
                    rename.new_path.to_string_lossy().into_owned(),
                ]
            })
            .collect()
    }
}

pub async fn prepare_move_mods_to_object(
    pool: &sqlx::SqlitePool,
    params: MoveModsToObjectParams<'_>,
) -> Result<PreparedOrganizerMove, AppError> {
    let game_mod_path =
        crate::modules::games::adapters::sqlite::game::get_mod_path(pool, params.game_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let target_obj = crate::modules::catalog::adapters::sqlite::object::get_game_object_by_id(
        pool,
        params.target_object_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Target object not found".to_string()))?;
    if target_obj.game_id != params.game_id {
        return Err(AppError::Validation(format!(
            "Target object '{}' belongs to game '{}', but requested move is for game '{}'",
            params.target_object_id, target_obj.game_id, params.game_id
        )));
    }

    let base_path = Path::new(&game_mod_path);
    let target_obj_path = base_path.join(&target_obj.folder_path);
    let (target_base_path, target_base_requires_creation) =
        resolve_target_base_path(&target_obj_path, params.target_subpath)?;
    let (target_namespace_proof, target_scope_path) =
        capture_move_destination_scope(base_path, &target_base_path)?;
    let target_anchor = target_scope_path
        .parent()
        .ok_or_else(|| AppError::Validation("Organizer destination has no anchor".into()))?
        .to_path_buf();
    let target_proof = FilesystemIdentityProof::capture(&target_anchor)?;
    let mut renames = Vec::new();
    let mut primary_results = Vec::new();
    let mut seen_sources = HashSet::new();

    for folder in params.folder_paths {
        let current_path = folder.to_path_buf();
        let source_proof = FilesystemIdentityProof::capture(&current_path)?;
        let folder_name = current_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| AppError::Validation("Mod folder name is invalid".to_string()))?;
        let target_name = match params.status {
            Some("disabled") => standardize_prefix(folder_name, false),
            Some("only-enable") => standardize_prefix(folder_name, true),
            _ => folder_name.to_string(),
        };
        let new_path = target_base_path.join(target_name);
        let old_rel = relative_mod_path(&current_path, base_path);
        let new_rel = relative_mod_path(&new_path, base_path);
        let target_mod_id =
            crate::modules::library::adapters::sqlite::mods::get_mod_id_and_status_by_path(
                pool,
                &old_rel,
                params.game_id,
            )
            .await?
            .map(|(id, _, _)| id);
        primary_results.push(new_rel.clone());

        if current_path != new_path {
            if new_path.exists() {
                return Err(AppError::Validation(format!(
                    "Destination already exists: {}",
                    new_path.display()
                )));
            }
            seen_sources.insert(current_path.clone());
            renames.push(PreparedOrganizerRename {
                source_proof,
                old_path: current_path,
                new_path,
                old_rel,
                new_rel: new_rel.clone(),
                target_object_id: params.target_object_id.to_string(),
            });
        }

        if params.status == Some("only-enable") {
            let siblings = crate::modules::library::adapters::sqlite::mods::get_enabled_duplicates(
                pool,
                params.target_object_id,
                params.game_id,
                target_mod_id.as_deref(),
            )
            .await?;
            for (_, sibling_rel, _) in siblings {
                let sibling_path = sibling_rel.resolve(base_path);
                let Some(sibling_name) = sibling_path.file_name().and_then(|value| value.to_str())
                else {
                    continue;
                };
                if crate::modules::workspace::domain::normalizer::is_disabled_folder(sibling_name)
                    || sibling_name.starts_with('.')
                    || !seen_sources.insert(sibling_path.clone())
                {
                    continue;
                }
                let disabled_path = sibling_path
                    .parent()
                    .unwrap_or(&target_obj_path)
                    .join(standardize_prefix(sibling_name, false));
                if disabled_path.exists() {
                    continue;
                }
                renames.push(PreparedOrganizerRename {
                    source_proof: FilesystemIdentityProof::capture(&sibling_path)?,
                    old_rel: sibling_rel.into_stored(),
                    new_rel: relative_mod_path(&disabled_path, base_path),
                    old_path: sibling_path,
                    new_path: disabled_path,
                    target_object_id: params.target_object_id.to_string(),
                });
            }
        }
    }

    target_namespace_proof.validate_paths(std::slice::from_ref(&target_scope_path))?;
    target_proof.validate(&target_anchor)?;
    Ok(PreparedOrganizerMove {
        namespace_proof: crate::platform::fs::file_utils::FilesystemNamespaceProof::capture(
            base_path,
            &renames
                .iter()
                .map(|rename| rename.old_path.clone())
                .collect::<Vec<_>>(),
        )?,
        target_namespace_proof,
        target_scope_path,
        target_proof,
        target_anchor,
        renames,
        primary_results,
        target_base_path,
        target_base_requires_creation,
    })
}

pub async fn execute_prepared_move(
    watcher: &WatcherState,
    prepared: &PreparedOrganizerMove,
) -> Result<OrganizerMoveOutcome, AppError> {
    prepared
        .target_namespace_proof
        .validate_paths(std::slice::from_ref(&prepared.target_scope_path))?;
    prepared.target_proof.validate(&prepared.target_anchor)?;
    let suppression_paths = prepared
        .renames
        .iter()
        .flat_map(|rename| [rename.old_path.as_path(), rename.new_path.as_path()])
        .chain(std::iter::once(prepared.target_base_path.as_path()));
    let _suppression = watcher.suppressor.suppress_paths(suppression_paths);
    let destination_pins = std::sync::Arc::new(pin_organizer_destination(prepared)?);
    let target_parent_proof = FilesystemIdentityProof::capture(&prepared.target_base_path)?;
    let mut retry_budget = crate::platform::fs::retry::RetryBudget::default();
    let mut applied: Vec<&PreparedOrganizerRename> = Vec::new();
    for rename in &prepared.renames {
        let outcome = rename_organizer_step(
            prepared,
            rename,
            &target_parent_proof,
            Some(std::sync::Arc::clone(&destination_pins)),
            false,
            &mut retry_budget,
        )
        .await;
        if let Err(error) = outcome {
            let mut rollback_budget = crate::platform::fs::retry::RetryBudget::default();
            for applied_rename in applied.iter().rev() {
                if let Err(rollback_error) = rename_organizer_step(
                    prepared,
                    applied_rename,
                    &target_parent_proof,
                    Some(std::sync::Arc::clone(&destination_pins)),
                    true,
                    &mut rollback_budget,
                )
                .await
                {
                    return Err(AppError::Io(format!(
                        "Organizer move failed: {error}; rollback failed: {rollback_error}"
                    )));
                }
            }
            return Err(error);
        }
        applied.push(rename);
    }

    let path_hints = prepared
        .renames
        .iter()
        .map(|rename| OrganizerMovePathHint {
            old_path: rename.old_rel.clone(),
            new_path: rename.new_rel.clone(),
            target_object_id: rename.target_object_id.clone(),
        })
        .collect();
    let path_rewrites = prepared
        .renames
        .iter()
        .map(
            |rename| crate::modules::workspace::domain::workspace::WorkspacePathRewrite {
                old_path: rename.old_rel.clone(),
                new_path: rename.new_rel.clone(),
            },
        )
        .collect();
    Ok(OrganizerMoveOutcome {
        result:
            crate::modules::library::application::mods::bulk::BulkResult::with_collection_impact(
                prepared.primary_results.clone(),
                Vec::new(),
                crate::modules::collections::domain::collection::CollectionReferenceImpact::default(
                ),
                path_rewrites,
            ),
        path_hints,
    })
}

fn pin_organizer_destination(
    prepared: &PreparedOrganizerMove,
) -> Result<Vec<PinnedDirectory>, AppError> {
    let mut pins = prepared
        .target_namespace_proof
        .pin_paths(std::slice::from_ref(&prepared.target_scope_path))?;
    let anchor = PinnedDirectory::open(&prepared.target_anchor)?;
    if anchor.identity() != prepared.target_proof.identity() {
        return Err(AppError::Io(
            "Organizer destination ownership changed".into(),
        ));
    }
    pins.push(anchor);
    let missing = prepared
        .target_base_path
        .strip_prefix(&prepared.target_anchor)
        .map_err(|_| {
            AppError::Security("Organizer destination escaped its captured anchor".into())
        })?;
    if !missing.as_os_str().is_empty() && !prepared.target_base_requires_creation {
        return Err(AppError::Validation(
            "Organizer destination was not prepared for creation".into(),
        ));
    }
    for component in missing.components() {
        let Component::Normal(name) = component else {
            return Err(AppError::Security(
                "Invalid organizer destination component".into(),
            ));
        };
        prepared
            .target_namespace_proof
            .validate_paths(std::slice::from_ref(&prepared.target_scope_path))?;
        // Publish a new owned directory through its pinned parent. A directory
        // or junction that appeared after preparation is never adopted.
        let child = pins
            .last()
            .ok_or_else(|| AppError::Io("Organizer destination anchor is unavailable".into()))?
            .create_child(name)?;
        pins.push(child);
    }
    prepared
        .target_namespace_proof
        .validate_paths(std::slice::from_ref(&prepared.target_scope_path))?;
    let destination = pins
        .last()
        .ok_or_else(|| AppError::Io("Organizer destination is unavailable".into()))?;
    let identity = FilesystemIdentityProof::capture(&prepared.target_base_path)?;
    if destination.identity() != identity.identity() {
        return Err(AppError::Io(
            "Organizer destination changed during creation".into(),
        ));
    }
    Ok(pins)
}

pub async fn rollback_prepared_move(
    watcher: &WatcherState,
    prepared: &PreparedOrganizerMove,
) -> Result<(), AppError> {
    let suppression_paths = prepared
        .renames
        .iter()
        .flat_map(|rename| [rename.old_path.as_path(), rename.new_path.as_path()]);
    let _suppression = watcher.suppressor.suppress_paths(suppression_paths);
    let mut failures = Vec::new();
    let target_parent_proof = FilesystemIdentityProof::capture(&prepared.target_base_path)?;
    let mut retry_budget = crate::platform::fs::retry::RetryBudget::default();
    for rename in prepared.renames.iter().rev() {
        if rename.new_path.exists() {
            if let Err(error) = rename_organizer_step(
                prepared,
                rename,
                &target_parent_proof,
                None,
                true,
                &mut retry_budget,
            )
            .await
            {
                failures.push(format!(
                    "{} -> {}: {error}",
                    rename.new_path.display(),
                    rename.old_path.display()
                ));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Io(format!(
            "Organizer rollback was incomplete: {}",
            failures.join("; ")
        )))
    }
}

pub async fn move_mods_to_object_service(
    pool: &sqlx::SqlitePool,
    _op_guard: &crate::platform::fs::operation_lock::OpGuard,
    watcher: &WatcherState,
    params: MoveModsToObjectParams<'_>,
) -> Result<OrganizerMoveOutcome, AppError> {
    if params.folder_paths.is_empty() {
        return Ok(OrganizerMoveOutcome {
            result: crate::modules::library::application::mods::bulk::BulkResult::new(
                Vec::new(),
                Vec::new(),
            ),
            path_hints: Vec::new(),
        });
    }

    let prepared = prepare_move_mods_to_object(pool, params).await?;
    execute_prepared_move(watcher, &prepared).await
}

async fn rename_organizer_step(
    prepared: &PreparedOrganizerMove,
    rename: &PreparedOrganizerRename,
    target_parent_proof: &FilesystemIdentityProof,
    destination_pins: Option<std::sync::Arc<Vec<PinnedDirectory>>>,
    reverse: bool,
    budget: &mut crate::platform::fs::retry::RetryBudget,
) -> Result<(), AppError> {
    budget
        .run(|| {
            let namespace = prepared.namespace_proof.clone();
            let target_namespace = prepared.target_namespace_proof.clone();
            let target_scope = prepared.target_scope_path.clone();
            let target_parent = prepared.target_base_path.clone();
            let target_proof = target_parent_proof.clone();
            let target_anchor = prepared.target_anchor.clone();
            let anchor_proof = prepared.target_proof.clone();
            let rename = rename.clone();
            let destination_pins = destination_pins.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    let (source, destination) = if reverse {
                        (&rename.new_path, &rename.old_path)
                    } else {
                        (&rename.old_path, &rename.new_path)
                    };
                    if let Some(pins) = &destination_pins {
                        validate_retained_organizer_destination(pins)?;
                    }
                    let _source_pins = namespace
                        .pin_paths(std::slice::from_ref(&rename.old_path))
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    let _target_pins = target_namespace
                        .pin_paths(std::slice::from_ref(&target_scope))
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    anchor_proof
                        .validate(&target_anchor)
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    target_proof
                        .validate(&target_parent)
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    rename
                        .source_proof
                        .validate(source)
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    let parent_path = destination.parent().ok_or_else(|| {
                        std::io::Error::other("Organizer destination has no parent")
                    })?;
                    let parent_proof = FilesystemIdentityProof::capture(parent_path)
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    let opened_parent;
                    let parent = if !reverse {
                        match destination_pins
                            .as_ref()
                            .and_then(|pins| pins.last())
                            .filter(|parent| parent.identity() == parent_proof.identity())
                        {
                            Some(parent) => parent,
                            None => {
                                opened_parent = PinnedDirectory::open(parent_path)?;
                                &opened_parent
                            }
                        }
                    } else {
                        opened_parent = PinnedDirectory::open(parent_path)?;
                        &opened_parent
                    };
                    if parent.identity() != parent_proof.identity() {
                        return Err(std::io::Error::other(
                            "Organizer destination ownership changed",
                        ));
                    }
                    let owned = PinnedDirectory::open_for_rename(source)?;
                    if owned.identity() != rename.source_proof.identity() {
                        return Err(std::io::Error::other("Organizer source ownership changed"));
                    }
                    namespace
                        .validate_paths(std::slice::from_ref(&rename.old_path))
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    target_namespace
                        .validate_paths(std::slice::from_ref(&target_scope))
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    target_proof
                        .validate(&target_parent)
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    parent_proof
                        .validate(parent_path)
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    if let Some(pins) = &destination_pins {
                        validate_retained_organizer_destination(pins)?;
                    }
                    owned.rename_into(
                        parent,
                        destination.file_name().ok_or_else(|| {
                            std::io::Error::other("Organizer destination has no name")
                        })?,
                    )
                })
                .await
                .map_err(crate::platform::fs::retry::blocking_error)?
            }
        })
        .await
        .map_err(|error| {
            crate::platform::fs::retry::map_file_error_with_identity(
                if reverse {
                    &rename.new_path
                } else {
                    &rename.old_path
                },
                rename.source_proof.identity(),
                "move folder",
                error,
            )
        })
}

fn validate_retained_organizer_destination(pins: &[PinnedDirectory]) -> std::io::Result<()> {
    for pin in pins {
        let original = crate::shared::path_key::physical_namespace_path(pin.path())
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let current = crate::shared::path_key::physical_namespace_path(&pin.current_path()?)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let canonical =
            crate::shared::path_key::physical_namespace_path(&pin.path().canonicalize()?)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
        if original != current
            || original != canonical
            || crate::platform::fs::file_utils::filesystem_identity(pin.path()).as_deref()
                != Some(pin.identity())
        {
            return Err(std::io::Error::other(
                "Organizer destination ancestry changed after creation",
            ));
        }
    }
    Ok(())
}

pub(super) fn capture_move_destination_scope(
    mods_root: &Path,
    destination: &Path,
) -> Result<
    (
        crate::platform::fs::file_utils::FilesystemNamespaceProof,
        PathBuf,
    ),
    AppError,
> {
    let anchor = destination
        .ancestors()
        .find(|path| path.is_dir())
        .ok_or_else(|| AppError::NotFound("Organizer destination root is unavailable".into()))?;
    let name = destination
        .file_name()
        .ok_or_else(|| AppError::Validation("Organizer destination has no name".into()))?;
    // Namespace proofs inspect the parent chain, not the leaf. Bind the
    // existing anchor even when the final destination directory is not created yet.
    let scope_path = anchor.join(name);
    let proof = crate::platform::fs::file_utils::FilesystemNamespaceProof::capture(
        mods_root,
        std::slice::from_ref(&scope_path),
    )?;
    Ok((proof, scope_path))
}

fn resolve_target_base_path(
    target_obj_path: &Path,
    target_subpath: Option<&str>,
) -> Result<(PathBuf, bool), AppError> {
    let Some(relative_subpath) = parse_target_subpath(target_subpath)? else {
        return Ok((target_obj_path.to_path_buf(), true));
    };

    let target = target_obj_path.join(relative_subpath);
    if !target.is_dir() {
        return Err(AppError::NotFound(format!(
            "Target subfolder does not exist: {}",
            target.to_string_lossy()
        )));
    }

    Ok((target, false))
}

fn parse_target_subpath(target_subpath: Option<&str>) -> Result<Option<PathBuf>, AppError> {
    let Some(raw_subpath) = target_subpath else {
        return Ok(None);
    };
    let trimmed = raw_subpath.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let mut relative = PathBuf::new();
    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(value) => relative.push(value),
            _ => {
                return Err(AppError::Security(format!(
                    "Invalid target subfolder: {trimmed}"
                )));
            }
        }
    }

    Ok(Some(relative))
}

fn relative_mod_path(path: &Path, mods_root: &Path) -> String {
    let canonical_root = mods_root
        .canonicalize()
        .unwrap_or_else(|_| mods_root.to_path_buf());
    path.strip_prefix(&canonical_root)
        .or_else(|_| path.strip_prefix(mods_root))
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
#[path = "tests/organizer_move_tests.rs"]
mod tests;
