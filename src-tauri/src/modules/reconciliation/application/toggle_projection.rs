use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::modules::mutation::api::{MutationStepKind, Operation, StepStatus};
use crate::modules::workspace::api::scanner::watcher::ModWatchEvent;
use crate::shared::errors::AppError;

pub(super) use crate::shared::path_key::physical_namespace_path as projection_physical_path;

#[derive(Debug)]
pub(crate) enum ProjectionRenameProof {
    Proven(Vec<ModWatchEvent>),
    Replaced(Vec<String>),
}

pub(crate) fn projection_rename_proof(
    pending: &[Operation],
    root: &Path,
) -> Result<ProjectionRenameProof, AppError> {
    let renames = final_projection_renames(pending, root)?;
    let targets = renames
        .iter()
        .map(|(_, target, identity)| (identity, target))
        .collect::<HashMap<_, _>>();
    let mut replaced_roots = Vec::new();
    let mut unavailable = None;
    for (identity, target) in &targets {
        match crate::platform::fs::file_utils::filesystem_identity(target) {
            Some(actual) if actual.as_str() != identity.as_str() => replaced_roots.push(*target),
            None => unavailable = Some(*target),
            Some(_) => {}
        }
    }
    if !replaced_roots.is_empty() {
        // A replaced parent also invalidates the owned lineage beneath it,
        // even when a child's endpoint has disappeared inside that replacement.
        let replaced_identities = targets
            .iter()
            .filter(|(_, target)| {
                replaced_roots
                    .iter()
                    .any(|replaced| target.starts_with(replaced))
            })
            .map(|(identity, _)| identity.as_str())
            .collect::<std::collections::HashSet<_>>();
        let affected = pending
            .iter()
            .filter(|operation| {
                operation.steps.iter().any(|step| {
                    step.status != StepStatus::Skipped
                        && step
                            .expected_identity
                            .as_deref()
                            .is_some_and(|identity| replaced_identities.contains(identity))
                })
            })
            .map(|operation| operation.id.clone())
            .collect();
        return Ok(ProjectionRenameProof::Replaced(affected));
    }
    if let Some(target) = unavailable {
        return Err(AppError::Io(format!(
            "Projection target identity is unavailable: {}",
            target.display()
        )));
    }
    Ok(ProjectionRenameProof::Proven(
        renames
            .into_iter()
            .filter(|(old, new, _)| old != new)
            .map(|(old, new, _)| ModWatchEvent::Renamed {
                from: old.to_string_lossy().into_owned(),
                to: new.to_string_lossy().into_owned(),
            })
            .collect(),
    ))
}

pub(crate) fn rename_confirmation_operation_ids(
    pending: &[Operation],
    root: &Path,
    groups: &[super::disk_reconcile::types::RenameConfirmationGroup],
) -> Vec<String> {
    use super::disk_reconcile::types::RenameConfirmationKind;
    if groups
        .iter()
        .any(|group| group.kind == RenameConfirmationKind::Object)
    {
        return pending
            .iter()
            .map(|operation| operation.id.clone())
            .collect();
    }
    let mut scopes = std::collections::HashSet::new();
    for group in groups {
        if !group.scope_key.is_empty() {
            scopes.insert(crate::shared::path_key::canonical_name_key(
                &group.scope_key,
            ));
        }
        scopes.extend(
            group
                .previous_paths
                .iter()
                .chain(&group.current_paths)
                .filter_map(|path| projection_top_level_scope(Path::new(path), root)),
        );
    }
    pending
        .iter()
        .filter(|operation| {
            operation
                .steps
                .iter()
                .filter(|step| step.status != StepStatus::Skipped)
                .flat_map(|step| [&step.old_path, &step.new_path])
                .flatten()
                .filter_map(|path| projection_top_level_scope(path, root))
                .any(|scope| scopes.contains(&scope))
        })
        .map(|operation| operation.id.clone())
        .collect()
}

fn projection_top_level_scope(path: &Path, root: &Path) -> Option<String> {
    let path = if path.is_absolute() {
        projection_physical_path(path).ok()?
    } else {
        path.to_path_buf()
    };
    let root = projection_physical_path(root).ok()?;
    let relative = if path.is_absolute() {
        path.strip_prefix(&root).ok()?
    } else {
        &path
    };
    let text = relative.to_string_lossy();
    text.split(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .map(crate::shared::path_key::canonical_name_key)
}

pub(crate) async fn settle_repaired_projection_operations(
    pool: &sqlx::SqlitePool,
    coordinator: &crate::modules::mutation::api::MutationCoordinator,
    game_id: &str,
    epoch: &str,
    root: &Path,
    pending: &[Operation],
) -> Result<Vec<Operation>, AppError> {
    let repairs = coordinator.repair_toggle_disk_commits_in_epoch(game_id, epoch)?;
    let ids = repairs
        .iter()
        .map(|operation| operation.id.clone())
        .collect::<Vec<_>>();
    let mut current = pending.to_vec();
    current.extend(repairs);
    current.sort_by_key(|operation| operation.disk_revision);
    if crate::platform::fs::file_utils::filesystem_identity(root).as_deref() != Some(epoch) {
        return Err(AppError::Io(
            "Mods root identity changed during repair validation".to_string(),
        ));
    }
    let mut lineage = coordinator.toggle_projection_lineage_in_epoch(game_id, epoch)?;
    lineage.extend(
        current
            .iter()
            .filter(|operation| operation.source_epoch.is_none())
            .cloned(),
    );
    lineage.sort_by_key(|operation| operation.disk_revision);
    let invalid =
        unprojected_toggle_operation_ids_with_lineage(pool, game_id, &current, &lineage, root)
            .await?;
    if crate::platform::fs::file_utils::filesystem_identity(root).as_deref() != Some(epoch) {
        return Err(AppError::Io(
            "Mods root identity changed during repair validation".to_string(),
        ));
    }
    let repaired_ids = ids
        .into_iter()
        .filter(|id| !invalid.contains(id))
        .collect::<Vec<_>>();
    coordinator.complete_repaired_disk_projection(game_id, epoch, &repaired_ids)?;
    if !invalid.is_empty() {
        return Err(AppError::Io(format!(
            "Owned disk rename still requires repair: {}",
            invalid.join(", ")
        )));
    }
    Ok(current)
}

fn final_projection_renames(
    pending: &[Operation],
    root: &Path,
) -> Result<Vec<(PathBuf, PathBuf, String)>, AppError> {
    let root = projection_physical_path(root)?;
    let mut rewrites = HashMap::<PathBuf, PathBuf>::new();
    let mut result = Vec::new();
    for operation in pending.iter().rev() {
        for step in operation.steps.iter().rev() {
            if step.status == StepStatus::Skipped {
                continue;
            }
            let (Some(old), Some(new), Some(identity)) =
                (&step.old_path, &step.new_path, &step.expected_identity)
            else {
                return Err(AppError::Io(
                    "Projection requires identity-proven rename steps".to_string(),
                ));
            };
            let old = projection_physical_path(old)?;
            let new = projection_physical_path(new)?;
            if step.kind != MutationStepKind::Rename
                || step.status != StepStatus::Applied
                || !old.starts_with(&root)
                || !new.starts_with(&root)
            {
                return Err(AppError::Io(
                    "Projection rename lineage is outside its Mods root".to_string(),
                ));
            }
            let mut final_path = new.clone();
            let mut visited = std::collections::HashSet::new();
            loop {
                if !visited.insert(final_path.clone()) {
                    return Err(AppError::Io(
                        "Projection rename lineage contains a cycle".to_string(),
                    ));
                }
                let next = final_path.ancestors().find_map(|ancestor| {
                    rewrites.get(ancestor).map(|target| {
                        target.join(final_path.strip_prefix(ancestor).expect("ancestor prefix"))
                    })
                });
                let Some(next) = next.filter(|next| next != &final_path) else {
                    break;
                };
                final_path = next;
            }
            rewrites.insert(old.clone(), final_path.clone());
            result.push((old.clone(), final_path, identity.clone()));
        }
    }
    result.reverse();
    Ok(result)
}

#[cfg(test)]
pub(crate) fn proven_projection_events(
    pending: &[Operation],
    root: &Path,
) -> Result<Vec<ModWatchEvent>, AppError> {
    match projection_rename_proof(pending, root)? {
        ProjectionRenameProof::Proven(events) => Ok(events),
        ProjectionRenameProof::Replaced(ids) => Err(AppError::Io(format!(
            "Projection target identity changed for operations: {}",
            ids.join(", ")
        ))),
    }
}

#[cfg(test)]
pub(crate) async fn validate_projected_toggle_rows(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    pending: &[Operation],
    root: &Path,
) -> Result<(), AppError> {
    let invalid = unprojected_toggle_operation_ids(pool, game_id, pending, root).await?;
    if !invalid.is_empty() {
        return Err(AppError::Io(format!("Owned disk rename is not projected; resolve the folder conflict before capturing a collection: {}", invalid.join(", "))));
    }
    Ok(())
}

pub(crate) async fn unprojected_toggle_operation_ids(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    pending: &[Operation],
    root: &Path,
) -> Result<Vec<String>, AppError> {
    unprojected_toggle_operation_ids_with_lineage(pool, game_id, pending, pending, root).await
}

async fn unprojected_toggle_operation_ids_with_lineage(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    pending: &[Operation],
    lineage: &[Operation],
    root: &Path,
) -> Result<Vec<String>, AppError> {
    let root = projection_physical_path(root)?;
    let required = pending
        .iter()
        .flat_map(|operation| &operation.steps)
        .filter_map(|step| step.expected_identity.as_ref())
        .collect::<std::collections::HashSet<_>>();
    let renames = final_projection_renames(lineage, &root)?
        .into_iter()
        .filter(|(_, _, identity)| required.contains(identity))
        .collect::<Vec<_>>();
    let scope_keys = renames
        .iter()
        .filter_map(|(old, _, _)| {
            old.strip_prefix(&root)
                .ok()?
                .components()
                .next()
                .map(|component| {
                    crate::shared::path_key::folder_path_key(
                        &component.as_os_str().to_string_lossy(),
                        Some(&root.to_string_lossy()),
                    )
                })
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let targets = renames
        .iter()
        .map(|(_, path, identity)| (identity.clone(), path.clone()))
        .collect::<HashMap<_, _>>();
    let identities = targets.keys().cloned().collect::<Vec<_>>();
    let mut conn = pool.acquire().await?;
    let mods = crate::modules::library::api::get_reconcile_mod_rows(
        &mut conn,
        game_id,
        &scope_keys,
        &identities,
    )
    .await?;
    let objects = crate::modules::catalog::api::get_reconcile_object_rows(
        &mut conn,
        game_id,
        &[],
        &identities,
    )
    .await?;
    let rows = mods
        .iter()
        .map(|row| {
            (
                row.filesystem_identity.clone(),
                row.folder_path.clone(),
                row.status,
            )
        })
        .chain(
            objects
                .into_iter()
                .map(|row| (row.filesystem_identity, row.folder_path, row.status)),
        )
        .filter_map(|(identity, path, status)| identity.map(|identity| (identity, (path, status))))
        .collect::<HashMap<_, _>>();
    let mut invalid_identities = std::collections::HashSet::new();
    let mut unindexed_targets = Vec::new();
    let mut descendant_checks = std::collections::BTreeMap::new();
    let mods_by_path = mods
        .iter()
        .map(|row| {
            (
                crate::shared::path_key::exact_location_key_for_path(Path::new(&row.folder_path)),
                row,
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for (old, new, identity) in &renames {
        let old = crate::shared::path_key::exact_location_key_for_path(
            old.strip_prefix(&root)
                .map_err(|_| AppError::Io("Projection source is outside its root".to_string()))?,
        );
        let new =
            crate::shared::path_key::exact_location_key_for_path(new.strip_prefix(&root).map_err(
                |_| AppError::Io("Projection destination is outside its root".to_string()),
            )?);
        if old != new
            && (mods_by_path.contains_key(&old)
                || mods_by_path
                    .range(format!("{old}/")..format!("{old}0"))
                    .next()
                    .is_some())
        {
            invalid_identities.insert(identity.clone());
        }
        for row in mods_by_path.get(&new).into_iter().chain(
            mods_by_path
                .range(format!("{new}/")..format!("{new}0"))
                .map(|(_, row)| row),
        ) {
            descendant_checks.insert(
                (identity.clone(), root.join(&row.folder_path)),
                (row.filesystem_identity.clone(), row.status),
            );
        }
    }
    for (identity, target) in targets {
        let relative = target
            .strip_prefix(&root)
            .map_err(|_| AppError::Io("Projection target is outside its root".to_string()))?;
        if let Some((stored, status)) = rows.get(&identity) {
            let enabled = !crate::modules::workspace::domain::normalizer::is_disabled_folder(
                &relative.file_name().unwrap_or_default().to_string_lossy(),
            );
            if crate::shared::path_key::exact_location_key_for_path(Path::new(stored))
                != crate::shared::path_key::exact_location_key_for_path(relative)
                || *status
                    != crate::modules::games::domain::models::ItemStatus::from_is_disabled(!enabled)
            {
                invalid_identities.insert(identity);
            }
        } else {
            if relative.components().count() == 1 {
                invalid_identities.insert(identity);
            } else {
                unindexed_targets.push((identity, target));
            }
        }
    }
    let proof_targets = renames
        .into_iter()
        .map(|(_, path, identity)| (identity, path))
        .collect::<HashMap<_, _>>();
    let disk_invalid = tokio::task::spawn_blocking(move || {
        let mut invalid = std::collections::HashSet::new();
        // A rowless container is valid only when it has no terminal INI of its
        // own; descendant ownership is verified separately below.
        for (identity, path) in unindexed_targets {
            for entry in std::fs::read_dir(path)? {
                if entry?
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("ini"))
                {
                    invalid.insert(identity);
                    break;
                }
            }
        }
        for (identity, path) in proof_targets {
            if crate::platform::fs::file_utils::filesystem_identity(&path).as_ref()
                != Some(&identity)
            {
                invalid.insert(identity);
            }
        }
        for ((owner, path), (identity, status)) in descendant_checks {
            let expected_status =
                crate::modules::games::domain::models::ItemStatus::from_is_disabled(
                    crate::modules::workspace::domain::normalizer::is_disabled_folder(
                        &path.file_name().unwrap_or_default().to_string_lossy(),
                    ),
                );
            if identity.is_none()
                || crate::platform::fs::file_utils::filesystem_identity(&path) != identity
                || status != expected_status
            {
                invalid.insert(owner);
            }
        }
        Ok::<_, AppError>(invalid)
    })
    .await??;
    invalid_identities.extend(disk_invalid);
    Ok(pending
        .iter()
        .filter(|operation| {
            operation.steps.iter().any(|step| {
                step.expected_identity
                    .as_ref()
                    .is_some_and(|identity| invalid_identities.contains(identity))
            })
        })
        .map(|operation| operation.id.clone())
        .collect())
}

pub(crate) fn trusted_projection_paths(pending: &[Operation]) -> Option<Vec<String>> {
    let mut paths = Vec::new();
    for operation in pending {
        for step in &operation.steps {
            if step.status == StepStatus::Skipped && step.kind == MutationStepKind::Rename {
                continue;
            }
            if step.kind != MutationStepKind::Rename
                || step.status != StepStatus::Applied
                || step.expected_identity.is_none()
            {
                return None;
            }
            let (Some(old), Some(new)) = (&step.old_path, &step.new_path) else {
                return None;
            };
            paths.push(old.to_string_lossy().into_owned());
            paths.push(new.to_string_lossy().into_owned());
        }
    }
    (!paths.is_empty()).then_some(paths)
}

// Legacy receipts have no root epoch. Adopt only a complete, identity-proven
// final lineage inside the configured root; never infer ownership from names.
pub(crate) fn legacy_projection_matches_root(pending: &[Operation], root: &Path) -> bool {
    let Ok(root) = projection_physical_path(root) else {
        return false;
    };
    let mut endpoints = HashMap::<String, PathBuf>::new();
    for operation in pending {
        for step in &operation.steps {
            if step.status == StepStatus::Skipped {
                continue;
            }
            let (Some(old), Some(new), Some(identity)) =
                (&step.old_path, &step.new_path, &step.expected_identity)
            else {
                return false;
            };
            let (Ok(old), Ok(new)) = (projection_physical_path(old), projection_physical_path(new))
            else {
                return false;
            };
            if step.kind != MutationStepKind::Rename
                || step.status != StepStatus::Applied
                || !old.starts_with(&root)
                || !new.starts_with(&root)
            {
                return false;
            }
            for endpoint in endpoints.values_mut() {
                if let Ok(suffix) = endpoint.strip_prefix(&old) {
                    *endpoint = new.join(suffix);
                }
            }
            endpoints.insert(identity.clone(), new.clone());
        }
    }
    !endpoints.is_empty()
        && endpoints.iter().all(|(identity, path)| {
            crate::platform::fs::file_utils::filesystem_identity(path).as_ref() == Some(identity)
        })
}
