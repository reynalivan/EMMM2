use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::modules::catalog::adapters::sqlite::object::ReconcileObjectRow;
use crate::modules::library::adapters::sqlite::mods::ReconcileModRow;
use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{
    collect_disk_projection, DiskProjection,
};
use crate::shared::errors::AppError;

const GENERATED_EMMM_DATA_DIRECTORY: &str = ".emmm_data";

fn move_generated_emmm_data_directory(
    from_mods_path: &Path,
    to_mods_path: &Path,
) -> Result<bool, AppError> {
    let source = from_mods_path.join(GENERATED_EMMM_DATA_DIRECTORY);
    if !source.is_dir() {
        return Ok(false);
    }

    let destination = to_mods_path.join(GENERATED_EMMM_DATA_DIRECTORY);
    if destination.exists() {
        return Err(AppError::Validation(format!(
            "Cannot move generated EMMM data because '{}' already exists",
            destination.display()
        )));
    }

    crate::platform::fs::file_utils::rename_cross_drive_fallback(&source, &destination)
        .map_err(|error| AppError::Io(format!("Could not move generated EMMM data: {error}")))?;
    Ok(true)
}

fn restore_generated_emmm_data_directory(
    moved: bool,
    applied_mods_path: &Path,
    previous_mods_path: &Path,
) -> Result<(), AppError> {
    if !moved {
        return Ok(());
    }
    if !applied_mods_path
        .join(GENERATED_EMMM_DATA_DIRECTORY)
        .is_dir()
    {
        return Err(AppError::Io(
            "Generated EMMM data disappeared before it could be restored".to_string(),
        ));
    }
    move_generated_emmm_data_directory(applied_mods_path, previous_mods_path)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum GameModsDirectoryClassification {
    Matching,
    NewLibrary,
    Empty,
    Different,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct GameModsDirectoryCandidateSummary {
    pub classification: GameModsDirectoryClassification,
    pub existing_object_count: u32,
    pub existing_mod_count: u32,
    pub candidate_object_count: u32,
    pub candidate_mod_count: u32,
    pub physical_entry_count: u32,
    pub filesystem_identity_match_count: u32,
    pub relative_path_match_count: u32,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct GameModsDirectoryInspection {
    pub game_id: String,
    pub candidate_path: String,
    pub fingerprint: String,
    pub summary: GameModsDirectoryCandidateSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ApplyGameModsDirectoryRequest {
    pub game_id: String,
    pub candidate_path: String,
    pub expected_fingerprint: String,
    pub confirm_empty: bool,
    pub different_confirmation_game_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ApplyGameModsDirectoryResult {
    pub game: crate::modules::settings::application::config::GameConfig,
    pub inspection: GameModsDirectoryInspection,
    pub reconcile:
        crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileResult,
    pub emmm_data_moved: bool,
    pub watcher_warning: Option<String>,
}

fn identity_set<'a>(values: impl Iterator<Item = Option<&'a str>>) -> HashSet<&'a str> {
    values.flatten().filter(|value| !value.is_empty()).collect()
}

pub(crate) fn classify_candidate(
    existing_objects: &[ReconcileObjectRow],
    existing_mods: &[ReconcileModRow],
    candidate: &DiskProjection,
    physical_entry_count: usize,
) -> GameModsDirectoryCandidateSummary {
    let existing_identity = identity_set(
        existing_objects
            .iter()
            .map(|row| row.filesystem_identity.as_deref())
            .chain(
                existing_mods
                    .iter()
                    .map(|row| row.filesystem_identity.as_deref()),
            ),
    );
    let candidate_identity = identity_set(
        candidate
            .objects
            .iter()
            .map(|entry| entry.filesystem_identity.as_deref())
            .chain(
                candidate
                    .mods
                    .iter()
                    .map(|entry| entry.filesystem_identity.as_deref()),
            ),
    );
    let filesystem_identity_match_count =
        existing_identity.intersection(&candidate_identity).count();

    let existing_entry_count = existing_objects.len() + existing_mods.len();
    let candidate_entry_count = candidate.objects.len() + candidate.mods.len();
    let existing_paths = existing_objects
        .iter()
        .map(|row| format!("object\0{}", row.folder_path_key))
        .chain(
            existing_mods
                .iter()
                .map(|row| format!("mod\0{}", row.folder_path_key)),
        )
        .collect::<HashSet<_>>();
    let candidate_paths = candidate
        .objects
        .iter()
        .map(|entry| format!("object\0{}", entry.folder_path_key))
        .chain(
            candidate
                .mods
                .iter()
                .map(|entry| format!("mod\0{}", entry.folder_path_key)),
        )
        .collect::<HashSet<_>>();
    let relative_path_match_count = existing_paths.intersection(&candidate_paths).count();

    let has_complete_identity_match = existing_entry_count == candidate_entry_count
        && existing_identity.len() == existing_entry_count
        && candidate_identity.len() == candidate_entry_count
        && existing_identity == candidate_identity;
    let has_complete_relative_path_match = !existing_paths.is_empty()
        && existing_entry_count == candidate_entry_count
        && existing_paths == candidate_paths;

    let existing_is_empty = existing_objects.is_empty() && existing_mods.is_empty();
    let candidate_is_empty = candidate.objects.is_empty() && candidate.mods.is_empty();
    let classification = if candidate_is_empty && physical_entry_count == 0 {
        GameModsDirectoryClassification::Empty
    } else if existing_is_empty && !candidate_is_empty {
        GameModsDirectoryClassification::NewLibrary
    } else if has_complete_identity_match || has_complete_relative_path_match {
        GameModsDirectoryClassification::Matching
    } else {
        GameModsDirectoryClassification::Different
    };

    GameModsDirectoryCandidateSummary {
        classification,
        existing_object_count: existing_objects.len() as u32,
        existing_mod_count: existing_mods.len() as u32,
        candidate_object_count: candidate.objects.len() as u32,
        candidate_mod_count: candidate.mods.len() as u32,
        physical_entry_count: physical_entry_count as u32,
        filesystem_identity_match_count: filesystem_identity_match_count as u32,
        relative_path_match_count: relative_path_match_count as u32,
        requires_confirmation: matches!(
            classification,
            GameModsDirectoryClassification::Empty | GameModsDirectoryClassification::Different
        ),
    }
}

fn candidate_fingerprint(
    canonical_path: &Path,
    projection: &DiskProjection,
    physical_entry_count: usize,
) -> String {
    let mut entries = projection
        .objects
        .iter()
        .map(|entry| {
            format!(
                "object\0{}\0{}",
                entry.folder_path,
                entry.filesystem_identity.as_deref().unwrap_or_default()
            )
        })
        .chain(projection.mods.iter().map(|entry| {
            format!(
                "mod\0{}\0{}",
                entry.folder_path,
                entry.filesystem_identity.as_deref().unwrap_or_default()
            )
        }))
        .collect::<Vec<_>>();
    entries.sort_unstable();

    let mut hasher = blake3::Hasher::new();
    hasher.update(canonical_path.to_string_lossy().as_bytes());
    hasher.update(b"\0");
    hasher.update(physical_entry_count.to_string().as_bytes());
    for entry in entries {
        hasher.update(b"\0");
        hasher.update(entry.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

pub async fn inspect_game_mods_directory(
    pool: &sqlx::SqlitePool,
    game_id: &str,
    candidate_path: &Path,
) -> Result<GameModsDirectoryInspection, AppError> {
    if game_id.trim().is_empty() {
        return Err(AppError::Validation("Game id is required".to_string()));
    }

    let candidate_path = PathBuf::from(candidate_path);
    let (canonical_path, physical_entry_count, projection) = tokio::task::spawn_blocking(
        move || -> Result<(PathBuf, usize, DiskProjection), AppError> {
            if !candidate_path.is_dir() {
                return Err(AppError::Validation(format!(
                    "Selected mods directory is unavailable: {}",
                    candidate_path.display()
                )));
            }
            let canonical_path = candidate_path.canonicalize()?;
            let physical_entry_count = std::fs::read_dir(&canonical_path)?
                .collect::<Result<Vec<_>, _>>()?
                .len();
            let projection = collect_disk_projection(&canonical_path, &[], false)
                .map_err(|error| AppError::Io(error.into_message()))?;
            Ok((canonical_path, physical_entry_count, projection))
        },
    )
    .await??;

    let mut conn = pool.acquire().await?;
    let existing_objects =
        crate::modules::catalog::adapters::sqlite::object::get_rows_for_reconcile(
            &mut conn, game_id,
        )
        .await?;
    let existing_mods =
        crate::modules::library::adapters::sqlite::mods::get_rows_for_reconcile(&mut conn, game_id)
            .await?;
    let summary = classify_candidate(
        &existing_objects,
        &existing_mods,
        &projection,
        physical_entry_count,
    );
    let fingerprint = candidate_fingerprint(&canonical_path, &projection, physical_entry_count);

    Ok(GameModsDirectoryInspection {
        game_id: game_id.to_string(),
        candidate_path: canonical_path.to_string_lossy().to_string(),
        fingerprint,
        summary,
    })
}

pub async fn apply_game_mods_directory(
    context: crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext<'_>,
    request: ApplyGameModsDirectoryRequest,
    game_guard: &tokio::sync::MutexGuard<'_, ()>,
    operation_guard: &crate::platform::fs::operation_lock::OpGuard,
) -> Result<ApplyGameModsDirectoryResult, AppError> {
    let inspection = inspect_game_mods_directory(
        context.pool,
        &request.game_id,
        Path::new(&request.candidate_path),
    )
    .await?;
    if inspection.fingerprint != request.expected_fingerprint {
        return Err(AppError::Validation(
            "The selected mods directory changed after it was inspected. Review it again before applying."
                .to_string(),
        ));
    }

    let previous_settings = context.config.get_settings();
    let game_index = previous_settings
        .games
        .iter()
        .position(|game| game.id == request.game_id)
        .ok_or_else(|| AppError::Validation(format!("Game '{}' was not found", request.game_id)))?;
    let current_game = &previous_settings.games[game_index];
    match inspection.summary.classification {
        GameModsDirectoryClassification::Matching | GameModsDirectoryClassification::NewLibrary => {
        }
        GameModsDirectoryClassification::Empty if request.confirm_empty => {}
        GameModsDirectoryClassification::Empty => {
            return Err(AppError::Validation(
                "Confirm the empty directory before creating a new mods library".to_string(),
            ));
        }
        GameModsDirectoryClassification::Different
            if request
                .different_confirmation_game_name
                .as_deref()
                .is_some_and(|name| name.trim() == current_game.name) => {}
        GameModsDirectoryClassification::Different => {
            return Err(AppError::Validation(format!(
                "Type the game name '{}' to confirm replacing this mods library",
                current_game.name
            )));
        }
    }

    let candidate_path = PathBuf::from(&inspection.candidate_path);
    let previous_mod_path = current_game.mod_path.clone();
    let emmm_data_moved = move_generated_emmm_data_directory(&previous_mod_path, &candidate_path)?;
    let (previous_mod_path, applied_game) = match context.config.update_settings(|settings| {
        let game = settings
            .games
            .iter_mut()
            .find(|game| game.id == request.game_id)
            .ok_or_else(|| {
                AppError::Validation(format!("Game '{}' was not found", request.game_id))
            })?;
        let previous_mod_path = std::mem::replace(&mut game.mod_path, candidate_path.clone());
        Ok((previous_mod_path, game.clone()))
    }) {
        Ok(result) => result,
        Err(error) => {
            restore_generated_emmm_data_directory(
                emmm_data_moved,
                &candidate_path,
                &previous_mod_path,
            )
            .map_err(|restore| {
                AppError::Internal(format!(
                    "Could not save the new mods directory ({error}) and could not restore generated EMMM data: {restore}"
                ))
            })?;
            return Err(error);
        }
    };
    let reconcile_result =
        crate::modules::reconciliation::application::disk_reconcile::orchestrator::reconcile_disk_state_under_locks(
            context.clone(),
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                request.game_id.clone(),
                crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileReason::ManualRepair,
                Vec::new(),
                true,
            )
            .defer_overlay_sync(),
            game_guard,
            operation_guard,
        )
        .await;

    let reconcile = match reconcile_result {
        Ok(result)
            if result.status
                != crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileStatus::SourceUnavailable =>
        {
            result
        }
        Ok(result) => {
            let error = AppError::Io(result.error_message.unwrap_or_else(|| {
                "The selected mods directory became unavailable during reconcile".to_string()
            }));
            rollback_game_mods_directory(
                context.config,
                &request.game_id,
                &candidate_path,
                &previous_mod_path,
            )
            .map_err(|rollback| {
                AppError::Internal(format!(
                    "Applying the mods directory failed ({error}) and restoring the previous directory also failed: {rollback}"
                ))
            })?;
            restore_generated_emmm_data_directory(
                emmm_data_moved,
                &candidate_path,
                &previous_mod_path,
            )
            .map_err(|restore| {
                AppError::Internal(format!(
                    "Applying the mods directory failed ({error}); the previous directory was restored, but generated EMMM data could not be restored: {restore}"
                ))
            })?;
            return Err(error);
        }
        Err(error) => {
            rollback_game_mods_directory(
                context.config,
                &request.game_id,
                &candidate_path,
                &previous_mod_path,
            )
            .map_err(|rollback| {
                AppError::Internal(format!(
                    "Applying the mods directory failed ({error}) and restoring the previous directory also failed: {rollback}"
                ))
            })?;
            restore_generated_emmm_data_directory(
                emmm_data_moved,
                &candidate_path,
                &previous_mod_path,
            )
            .map_err(|restore| {
                AppError::Internal(format!(
                    "Applying the mods directory failed ({error}); the previous directory was restored, but generated EMMM data could not be restored: {restore}"
                ))
            })?;
            return Err(error);
        }
    };

    Ok(ApplyGameModsDirectoryResult {
        game: applied_game,
        inspection,
        reconcile,
        emmm_data_moved,
        watcher_warning: None,
    })
}

fn rollback_game_mods_directory(
    config: &crate::modules::settings::application::config::ConfigService,
    game_id: &str,
    applied_path: &Path,
    previous_path: &Path,
) -> Result<(), AppError> {
    config.update_settings(|settings| {
        let game = settings
            .games
            .iter_mut()
            .find(|game| game.id == game_id)
            .ok_or_else(|| AppError::Validation(format!("Game '{game_id}' was not found")))?;
        if game.mod_path != applied_path {
            return Err(AppError::Validation(format!(
                "The mods directory for game '{game_id}' changed again before rollback"
            )));
        }
        game.mod_path = previous_path.to_path_buf();
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use crate::modules::catalog::adapters::sqlite::object::ReconcileObjectRow;
    use crate::modules::games::domain::models::ItemStatus;
    use crate::modules::library::adapters::sqlite::mods::ReconcileModRow;
    use crate::modules::reconciliation::application::disk_reconcile::disk_snapshot::{
        DiskModEntry, DiskObjectEntry, DiskProjection,
    };

    use super::{
        apply_game_mods_directory, candidate_fingerprint, classify_candidate,
        inspect_game_mods_directory, ApplyGameModsDirectoryRequest,
        GameModsDirectoryClassification,
    };

    fn object(path: &str, identity: Option<&str>) -> ReconcileObjectRow {
        ReconcileObjectRow {
            id: format!("object-{path}"),
            name: path.to_string(),
            folder_path: path.to_string(),
            folder_path_key: crate::shared::path_key::folder_path_key(path, None),
            status: ItemStatus::Enabled,
            object_type: "Other".to_string(),
            filesystem_identity: identity.map(str::to_string),
        }
    }

    fn db_mod(path: &str, identity: Option<&str>) -> ReconcileModRow {
        ReconcileModRow {
            id: format!("mod-{path}"),
            folder_path: path.to_string(),
            folder_path_key: crate::shared::path_key::folder_path_key(path, None),
            actual_name: path.to_string(),
            status: ItemStatus::Enabled,
            object_id: Some("object".to_string()),
            is_safe: false,
            safety_source: None,
            object_type: Some("Other".to_string()),
            filesystem_identity: identity.map(str::to_string),
            size_bytes: 0,
        }
    }

    fn disk_object(path: &str, identity: Option<&str>) -> DiskObjectEntry {
        DiskObjectEntry {
            folder_path: path.to_string(),
            folder_path_key: crate::shared::path_key::folder_path_key(path, None),
            name: path.to_string(),
            is_disabled: false,
            absolute_path: std::path::PathBuf::from(path),
            filesystem_identity: identity.map(str::to_string),
        }
    }

    fn disk_mod(path: &str, identity: Option<&str>) -> DiskModEntry {
        DiskModEntry {
            folder_path: path.to_string(),
            folder_path_key: crate::shared::path_key::folder_path_key(path, None),
            object_folder_path_key: path
                .split(['/', '\\'])
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase(),
            raw_name: path.to_string(),
            absolute_path: std::path::PathBuf::from(path),
            filesystem_identity: identity.map(str::to_string),
            size_bytes: None,
        }
    }

    #[test]
    fn identity_match_is_safe_to_auto_apply() {
        let existing_objects = vec![object("Alice", Some("object-fs"))];
        let existing_mods = vec![db_mod("Alice/Blue", Some("mod-fs"))];
        let candidate = DiskProjection {
            objects: vec![disk_object("Alicia", Some("object-fs"))],
            mods: vec![disk_mod("Alicia/Blue", Some("mod-fs"))],
        };

        let result = classify_candidate(&existing_objects, &existing_mods, &candidate, 1);

        assert_eq!(
            result.classification,
            GameModsDirectoryClassification::Matching
        );
        assert_eq!(result.filesystem_identity_match_count, 2);
        assert!(!result.requires_confirmation);
    }

    #[test]
    fn exact_relative_path_sets_match_after_cross_volume_copy() {
        let existing_objects = vec![object("Alice", Some("old-object"))];
        let existing_mods = vec![db_mod("Alice/Blue", Some("old-mod"))];
        let candidate = DiskProjection {
            objects: vec![disk_object("Alice", Some("new-object"))],
            mods: vec![disk_mod("Alice/Blue", Some("new-mod"))],
        };

        let result = classify_candidate(&existing_objects, &existing_mods, &candidate, 1);

        assert_eq!(
            result.classification,
            GameModsDirectoryClassification::Matching
        );
        assert_eq!(result.relative_path_match_count, 2);
    }

    #[test]
    fn partial_overlap_requires_different_library_confirmation() {
        let existing_objects = vec![object("Alice", None), object("Bob", None)];
        let existing_mods = vec![db_mod("Alice/Blue", None), db_mod("Bob/Red", None)];
        let candidate = DiskProjection {
            objects: vec![disk_object("Alice", None), disk_object("Carol", None)],
            mods: vec![disk_mod("Alice/Blue", None), disk_mod("Carol/Green", None)],
        };

        let result = classify_candidate(&existing_objects, &existing_mods, &candidate, 2);

        assert_eq!(
            result.classification,
            GameModsDirectoryClassification::Different
        );
        assert!(result.requires_confirmation);
    }

    #[test]
    fn one_matching_identity_does_not_match_a_mostly_different_library() {
        let existing_objects = vec![
            object("Alice", Some("object-alice")),
            object("Bob", Some("object-bob")),
        ];
        let existing_mods = vec![
            db_mod("Alice/Blue", Some("mod-blue")),
            db_mod("Bob/Red", Some("mod-red")),
        ];
        let candidate = DiskProjection {
            objects: vec![
                disk_object("Alice", Some("object-alice")),
                disk_object("Carol", Some("object-carol")),
            ],
            mods: vec![
                disk_mod("Alice/Green", Some("mod-green")),
                disk_mod("Carol/Purple", Some("mod-purple")),
            ],
        };

        let result = classify_candidate(&existing_objects, &existing_mods, &candidate, 2);

        assert_eq!(result.filesystem_identity_match_count, 1);
        assert_eq!(
            result.classification,
            GameModsDirectoryClassification::Different
        );
        assert!(result.requires_confirmation);
    }

    #[test]
    fn fingerprint_changes_when_mod_is_disabled_without_changing_its_path_key() {
        let enabled = DiskProjection {
            objects: vec![disk_object("Alice", Some("object-fs"))],
            mods: vec![disk_mod("Alice/Blue", Some("mod-fs"))],
        };
        let disabled = DiskProjection {
            objects: vec![disk_object("Alice", Some("object-fs"))],
            mods: vec![disk_mod("Alice/DISABLED Blue", Some("mod-fs"))],
        };
        let root = std::path::Path::new("C:/mods");

        assert_ne!(
            candidate_fingerprint(root, &enabled, 1),
            candidate_fingerprint(root, &disabled, 1)
        );
    }

    #[test]
    fn physical_empty_is_not_the_same_as_unrecognized_content() {
        let existing_objects = vec![object("Alice", None)];
        let existing_mods = vec![db_mod("Alice/Blue", None)];
        let candidate = DiskProjection::default();

        let empty = classify_candidate(&existing_objects, &existing_mods, &candidate, 0);
        let unrecognized = classify_candidate(&existing_objects, &existing_mods, &candidate, 3);

        assert_eq!(empty.classification, GameModsDirectoryClassification::Empty);
        assert_eq!(
            unrecognized.classification,
            GameModsDirectoryClassification::Different
        );
        assert!(empty.requires_confirmation);
        assert!(unrecognized.requires_confirmation);
    }

    #[test]
    fn candidate_is_new_library_when_database_has_no_runtime_rows() {
        let candidate = DiskProjection {
            objects: vec![disk_object("Alice", None)],
            mods: vec![disk_mod("Alice/Blue", None)],
        };

        let result = classify_candidate(&[], &[], &candidate, 1);

        assert_eq!(
            result.classification,
            GameModsDirectoryClassification::NewLibrary
        );
        assert!(!result.requires_confirmation);
    }

    #[tokio::test]
    async fn inspection_is_read_only_and_returns_a_stable_fingerprint() {
        let context = crate::test_utils::init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let terminal = temp.path().join("Alice").join("Blue Dress");
        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::write(
            terminal.join("mod.ini"),
            "[TextureOverrideAlice]\nhash = abc\n",
        )
        .expect("ini should be written");

        let before_objects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects")
            .fetch_one(&context.pool)
            .await
            .expect("object count should load");
        let before_mods: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods")
            .fetch_one(&context.pool)
            .await
            .expect("mod count should load");

        let first = inspect_game_mods_directory(&context.pool, "game-1", temp.path())
            .await
            .expect("candidate should inspect");
        let second = inspect_game_mods_directory(&context.pool, "game-1", temp.path())
            .await
            .expect("candidate should inspect again");

        assert_eq!(
            first.summary.classification,
            GameModsDirectoryClassification::NewLibrary
        );
        assert_eq!(first.fingerprint, second.fingerprint);
        assert!(!first.fingerprint.is_empty());
        assert_eq!(first.candidate_path, second.candidate_path);

        let after_objects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects")
            .fetch_one(&context.pool)
            .await
            .expect("object count should load");
        let after_mods: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods")
            .fetch_one(&context.pool)
            .await
            .expect("mod count should load");
        assert_eq!(before_objects, after_objects);
        assert_eq!(before_mods, after_mods);
    }

    #[tokio::test]
    async fn inspection_rejects_lossy_ini_decoding_before_any_apply_can_report_applied() {
        let context = crate::test_utils::init_test_db().await;
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let terminal = temp.path().join("Alice").join("Blue Dress");
        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::write(terminal.join("mod.ini"), [0xFF, 0xFE, 0xFD])
            .expect("invalid ini bytes should be written");

        let error = inspect_game_mods_directory(&context.pool, "game-1", temp.path())
            .await
            .expect_err("lossy INI decoding must abort authoritative inspection");

        assert!(
            matches!(error, crate::shared::errors::AppError::Io(message) if message.contains("encoding"))
        );
        let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects")
            .fetch_one(&context.pool)
            .await
            .expect("object count should load");
        assert_eq!(object_count, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn apply_rejects_a_candidate_changed_after_inspection_without_updating_config() {
        let context = crate::test_utils::init_test_db().await;
        let old_root = tempfile::tempdir().expect("old root should be created");
        let candidate = tempfile::tempdir().expect("candidate should be created");
        let terminal = candidate.path().join("Alice").join("Blue Dress");
        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::write(
            terminal.join("mod.ini"),
            "[TextureOverrideAlice]\nhash=abc\n",
        )
        .expect("ini should be written");
        crate::test_utils::insert_test_game(
            &context.pool,
            &crate::test_utils::TestGameFixture {
                id: "game-1",
                name: "Game",
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                path: old_root.path().to_string_lossy().as_ref(),
                mods_path: Some(old_root.path().to_string_lossy().as_ref()),
            },
        )
        .await
        .expect("game should be inserted");
        let config = crate::modules::settings::application::config::ConfigService::new_for_test(
            context.pool.clone(),
        );
        let inspection = inspect_game_mods_directory(&context.pool, "game-1", candidate.path())
            .await
            .expect("candidate should inspect");
        let state = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState::new();
        let operation_lock = crate::platform::fs::operation_lock::OperationLock::new();
        let game_lock = state.game_lock("game-1");
        let game_guard = game_lock.lock().await;
        let operation_guard = operation_lock.acquire().await.expect("operation lock");

        std::fs::create_dir_all(candidate.path().join("Bob"))
            .expect("candidate should be changed after inspection");
        let result = apply_game_mods_directory(
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                pool: &context.pool,
                config: &config,
                state: &state,
                watcher_suppressor: std::sync::Arc::new(
                    crate::modules::workspace::application::scanner::watcher::WatcherSuppressor::new(false),
                ),
                operation_lock: &operation_lock,
                progress_reporter: None,
            },
            ApplyGameModsDirectoryRequest {
                game_id: "game-1".to_string(),
                candidate_path: candidate.path().to_string_lossy().to_string(),
                expected_fingerprint: inspection.fingerprint,
                confirm_empty: false,
                different_confirmation_game_name: None,
            },
            &game_guard,
            &operation_guard,
        )
        .await;

        assert!(matches!(
            result,
            Err(crate::shared::errors::AppError::Validation(_))
        ));
        assert_eq!(
            config.mods_root_for("game-1"),
            Some(old_root.path().to_path_buf())
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn apply_restores_emmm_data_when_saving_the_new_directory_fails() {
        let source_context = crate::test_utils::init_test_db().await;
        let config_context = crate::test_utils::init_test_db().await;
        let old_root = tempfile::tempdir().expect("old root should be created");
        let candidate = tempfile::tempdir().expect("candidate should be created");
        let old_artifact = old_root.path().join(".emmm_data").join("status");
        std::fs::create_dir_all(&old_artifact).expect("runtime data directory should be created");
        std::fs::write(old_artifact.join("preset.ini"), "active=1")
            .expect("runtime artifact should be written");
        let terminal = candidate.path().join("Alice").join("Blue Dress");
        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::write(
            terminal.join("mod.ini"),
            "[TextureOverrideAlice]\nhash=abc\n",
        )
        .expect("ini should be written");
        crate::test_utils::insert_test_game(
            &config_context.pool,
            &crate::test_utils::TestGameFixture {
                id: "game-1",
                name: "Game",
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                path: old_root.path().to_string_lossy().as_ref(),
                mods_path: Some(old_root.path().to_string_lossy().as_ref()),
            },
        )
        .await
        .expect("game should be inserted");
        let config = crate::modules::settings::application::config::ConfigService::new_for_test(
            config_context.pool.clone(),
        );
        let inspection =
            inspect_game_mods_directory(&source_context.pool, "game-1", candidate.path())
                .await
                .expect("candidate should inspect");
        config_context.pool.close().await;

        let state = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState::new();
        let operation_lock = crate::platform::fs::operation_lock::OperationLock::new();
        let game_lock = state.game_lock("game-1");
        let game_guard = game_lock.lock().await;
        let operation_guard = operation_lock.acquire().await.expect("operation lock");
        let result = apply_game_mods_directory(
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                pool: &source_context.pool,
                config: &config,
                state: &state,
                watcher_suppressor: std::sync::Arc::new(
                    crate::modules::workspace::application::scanner::watcher::WatcherSuppressor::new(false),
                ),
                operation_lock: &operation_lock,
                progress_reporter: None,
            },
            ApplyGameModsDirectoryRequest {
                game_id: "game-1".to_string(),
                candidate_path: candidate.path().to_string_lossy().to_string(),
                expected_fingerprint: inspection.fingerprint,
                confirm_empty: false,
                different_confirmation_game_name: None,
            },
            &game_guard,
            &operation_guard,
        )
        .await;

        assert!(
            result.is_err(),
            "closed config database must reject the update"
        );
        assert_eq!(
            config.mods_root_for("game-1"),
            Some(old_root.path().to_path_buf()),
            "the in-memory config must remain at the old source"
        );
        assert_eq!(
            std::fs::read_to_string(old_artifact.join("preset.ini"))
                .expect("runtime artifact should be restored"),
            "active=1"
        );
        assert!(
            !candidate.path().join(".emmm_data").exists(),
            "the candidate root must not retain artifacts when config persistence fails"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn apply_new_library_updates_config_and_reconciles_database_from_disk() {
        let context = crate::test_utils::init_test_db().await;
        let old_root = tempfile::tempdir().expect("old root should be created");
        let candidate = tempfile::tempdir().expect("candidate should be created");
        let old_runtime_status = old_root.path().join(".emmm_data").join("status");
        std::fs::create_dir_all(&old_runtime_status)
            .expect("old runtime status directory should be created");
        std::fs::write(old_runtime_status.join("preset.ini"), "active=1")
            .expect("old runtime status artifact should be written");
        let terminal = candidate.path().join("Alice").join("Blue Dress");
        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::write(
            terminal.join("mod.ini"),
            "[TextureOverrideAlice]\nhash=abc\n",
        )
        .expect("ini should be written");
        crate::test_utils::insert_test_game(
            &context.pool,
            &crate::test_utils::TestGameFixture {
                id: "game-1",
                name: "Game",
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                path: old_root.path().to_string_lossy().as_ref(),
                mods_path: Some(old_root.path().to_string_lossy().as_ref()),
            },
        )
        .await
        .expect("game should be inserted");
        let config = crate::modules::settings::application::config::ConfigService::new_for_test(
            context.pool.clone(),
        );
        let state = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState::new();
        let operation_lock = crate::platform::fs::operation_lock::OperationLock::new();
        let inspection = inspect_game_mods_directory(&context.pool, "game-1", candidate.path())
            .await
            .expect("candidate should inspect");
        let game_lock = state.game_lock("game-1");
        let game_guard = game_lock.lock().await;
        let operation_guard = operation_lock.acquire().await.expect("operation lock");

        let result = apply_game_mods_directory(
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                pool: &context.pool,
                config: &config,
                state: &state,
                watcher_suppressor: std::sync::Arc::new(
                    crate::modules::workspace::application::scanner::watcher::WatcherSuppressor::new(false),
                ),
                operation_lock: &operation_lock,
                progress_reporter: None,
            },
            ApplyGameModsDirectoryRequest {
                game_id: "game-1".to_string(),
                candidate_path: candidate.path().to_string_lossy().to_string(),
                expected_fingerprint: inspection.fingerprint,
                confirm_empty: false,
                different_confirmation_game_name: None,
            },
            &game_guard,
            &operation_guard,
        )
        .await
        .expect("new library should apply");

        assert_eq!(
            result.reconcile.status,
            crate::modules::reconciliation::application::disk_reconcile::types::DiskReconcileStatus::Applied
        );
        assert_eq!(
            config.mods_root_for("game-1"),
            Some(result.game.mod_path.clone())
        );
        let mod_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = ?")
            .bind("game-1")
            .fetch_one(&context.pool)
            .await
            .expect("mod count should load");
        assert_eq!(mod_count, 1);
        assert!(
            !old_root.path().join(".emmm_data").exists(),
            "old generated EMMM data should be moved"
        );
        assert_eq!(
            std::fs::read_to_string(
                candidate
                    .path()
                    .join(".emmm_data")
                    .join("status")
                    .join("preset.ini"),
            )
            .expect("moved runtime status artifact should be readable"),
            "active=1"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn apply_defers_overlay_sync_until_the_watcher_is_restarted() {
        let context = crate::test_utils::init_test_db().await;
        let old_root = tempfile::tempdir().expect("old root should be created");
        let candidate = tempfile::tempdir().expect("candidate should be created");
        let terminal = candidate.path().join("Alice").join("Blue Dress");
        std::fs::create_dir_all(&terminal).expect("terminal folder should be created");
        std::fs::write(
            terminal.join("mod.ini"),
            "[TextureOverrideAlice]\nhash=abc\n",
        )
        .expect("ini should be written");
        crate::test_utils::insert_test_game(
            &context.pool,
            &crate::test_utils::TestGameFixture {
                id: "game-1",
                name: "Game",
                game_type: crate::modules::games::domain::models::GameType::GIMI,
                path: old_root.path().to_string_lossy().as_ref(),
                mods_path: Some(old_root.path().to_string_lossy().as_ref()),
            },
        )
        .await
        .expect("game should be inserted");
        let config = crate::modules::settings::application::config::ConfigService::new_for_test(
            context.pool.clone(),
        );
        let inspection = inspect_game_mods_directory(&context.pool, "game-1", candidate.path())
            .await
            .expect("candidate should inspect");
        let state = crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileState::new();
        let operation_lock = crate::platform::fs::operation_lock::OperationLock::new();
        let game_lock = state.game_lock("game-1");
        let game_guard = game_lock.lock().await;
        let operation_guard = operation_lock.acquire().await.expect("operation lock");

        let apply_result = apply_game_mods_directory(
            crate::modules::reconciliation::application::disk_reconcile::orchestrator::DiskReconcileContext {
                pool: &context.pool,
                config: &config,
                state: &state,
                watcher_suppressor: std::sync::Arc::new(
                    crate::modules::workspace::application::scanner::watcher::WatcherSuppressor::new(false),
                ),
                operation_lock: &operation_lock,
                progress_reporter: None,
            },
            ApplyGameModsDirectoryRequest {
                game_id: "game-1".to_string(),
                candidate_path: candidate.path().to_string_lossy().to_string(),
                expected_fingerprint: inspection.fingerprint,
                confirm_empty: false,
                different_confirmation_game_name: None,
            },
            &game_guard,
            &operation_guard,
        )
        .await;

        let object_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = 'game-1'")
                .fetch_one(&context.pool)
                .await
                .expect("object count should load");
        let mod_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mods WHERE game_id = 'game-1'")
                .fetch_one(&context.pool)
                .await
                .expect("mod count should load");
        assert_eq!(object_count, 1, "projection should already be committed");
        assert_eq!(mod_count, 1, "projection should already be committed");

        let configured_root = config.mods_root_for("game-1");
        assert_eq!(
            configured_root,
            Some(
                candidate
                    .path()
                    .canonicalize()
                    .expect("canonical candidate")
            ),
            "projection committed (objects={object_count}, mods={mod_count}) but source recovery rolled the config back after runtime effects failed: {apply_result:?}"
        );
        let reconcile = apply_result.expect("source change should apply").reconcile;
        assert!(reconcile.status.applied());
        assert!(
            !reconcile.pending_runtime_effects.overlay_refresh,
            "the Tauri command restarts the watcher before requesting the overlay sync"
        );
    }
}
