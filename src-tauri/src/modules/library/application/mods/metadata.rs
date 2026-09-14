use crate::modules::catalog::domain::objects::RandomizerMode;
use crate::modules::games::domain::models::ItemStatus;
use crate::modules::settings::application::config::ConfigService;
use crate::platform::fs::guard::ValidatedPath;
use crate::platform::images::thumbnail_cache::ThumbnailCache;
use crate::shared::errors::AppError;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Set the category for the mod's owning Object and every sibling mod.
///
/// A mod inherits its Object category; allowing a terminal folder to carry a
/// contradictory `object_type` makes category filters and randomizer disagree.
pub async fn set_mod_category(
    pool: &SqlitePool,
    game_id: &str,
    canonical_path: &ValidatedPath,
    category: &str,
) -> Result<(), AppError> {
    let folder_path_str = canonical_path.to_string_lossy();

    let exists = crate::modules::library::adapters::sqlite::mods::get_mod_id_and_object_id_by_path(
        pool,
        &folder_path_str,
        game_id,
    )
    .await?;

    let Some((_mod_id, Some(object_id))) = exists else {
        return Err(AppError::NotFound(
            "Mod must belong to an Object before its category can be changed. Please sync first."
                .to_string(),
        ));
    };

    crate::modules::catalog::application::objects::mutate::set_object_and_mods_category(
        pool, game_id, &object_id, category,
    )
    .await?;

    Ok(())
}

/// Update the thumbnail for a mod folder.
/// Copies the source image to `preview.png` (or keeps extension) in the mod folder.
/// Invalidates cache.
pub fn update_mod_thumbnail(
    target_dir: &ValidatedPath,
    source_path: &str,
) -> Result<String, AppError> {
    let source_path_obj = Path::new(source_path);
    if !source_path_obj.exists() || !source_path_obj.is_file() {
        return Err(AppError::NotFound(format!(
            "Source file does not exist: {source_path}"
        )));
    }

    let source_bytes = std::fs::read(source_path_obj)?;
    image::load_from_memory(&source_bytes).map_err(|error| {
        AppError::Metadata(crate::shared::errors::MetadataError::Validation(format!(
            "Invalid thumbnail image: {error}"
        )))
    })?;

    // Determine the new thumbnail path within the mod folder
    let new_thumbnail_name = source_path_obj
        .file_name()
        .ok_or_else(|| AppError::Validation("Invalid source file name".to_string()))?
        .to_string_lossy()
        .to_string();
    let new_thumbnail_path = target_dir.join(&new_thumbnail_name);

    crate::platform::fs::atomic_file::atomic_write(&new_thumbnail_path, &source_bytes)?;

    // Invalidate cache for this mod's thumbnail
    ThumbnailCache::invalidate(&new_thumbnail_path);
    ThumbnailCache::invalidate_folder(&target_dir.to_string_lossy());

    Ok(new_thumbnail_path.to_string_lossy().to_string())
}

pub async fn toggle_mod_safe(
    pool: &SqlitePool,
    game_id: &str,
    full_path: &ValidatedPath,
    safe: bool,
) -> Result<(), AppError> {
    let game_mod_path = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found or has no mods_path".to_string()))?;

    let base = std::path::Path::new(&game_mod_path);
    let rel_path = full_path
        .strip_prefix(base)
        .unwrap_or(full_path)
        .to_string_lossy()
        .to_string();

    let info_path = full_path.join("info.json");
    let previous = match std::fs::read(&info_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let update = crate::modules::library::application::mods::info_json::ModInfoUpdate {
        is_safe: Some(safe),
        ..Default::default()
    };
    crate::modules::library::application::mods::info_json::update_info_json(full_path, &update)?;
    if let Err(error) = crate::modules::library::adapters::sqlite::mods::set_mod_safe_by_path(
        pool, game_id, &rel_path, safe,
    )
    .await
    {
        let rollback = match previous.as_deref() {
            Some(bytes) => crate::platform::fs::atomic_file::atomic_write(&info_path, bytes),
            None if info_path.exists() => std::fs::remove_file(&info_path).map_err(AppError::from),
            None => Ok(()),
        };
        return Err(match rollback {
            Ok(()) => error.into(),
            Err(rollback_error) => AppError::Io(format!(
                "Safety database update failed ({error}); info.json rollback failed: {rollback_error}"
            )),
        });
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct RandomModProposal {
    pub object_id: String,
    pub object_name: String,
    pub object_type: Option<String>,
    pub mode: RandomizerLoadoutMode,
    pub is_safe: bool,
    pub active_mod_names: Vec<String>,
    pub mod_id: String,
    pub name: String,
    pub thumbnail_path: Option<String>,
    pub folder_path: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RandomizerLoadoutMode {
    Exclusive,
    Additive,
}

fn effective_mode(
    object_type: Option<&str>,
    override_value: Option<&str>,
) -> RandomizerLoadoutMode {
    match override_value.and_then(RandomizerMode::from_persisted) {
        Some(RandomizerMode::Exclusive) => RandomizerLoadoutMode::Exclusive,
        Some(RandomizerMode::Additive) => RandomizerLoadoutMode::Additive,
        Some(RandomizerMode::Default) | None => match object_type {
            Some("Character" | "Weapon") => RandomizerLoadoutMode::Exclusive,
            _ => RandomizerLoadoutMode::Additive,
        },
    }
}

/// Safety scope selected in the global explorer controls and enforced by both
/// randomizer generation and loadout application.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RandomizerSafetyFilter {
    All,
    Safe,
    Unsafe,
}

impl RandomizerSafetyFilter {
    pub fn allows(self, is_safe: bool) -> bool {
        match self {
            Self::All => true,
            Self::Safe => is_safe,
            Self::Unsafe => !is_safe,
        }
    }
}

/// Object categories included in one randomizer roll.
#[derive(Debug, Clone, Deserialize, Serialize, specta::Type, PartialEq, Eq)]
pub struct RandomizerScope {
    pub categories:
        Vec<crate::modules::ingestion::application::import_batch::types::StableCategory>,
    pub include_unclassified: bool,
}

impl RandomizerScope {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.categories.is_empty() && !self.include_unclassified {
            return Err(AppError::Validation(
                "Select at least one randomizer scope".to_string(),
            ));
        }

        if self
            .categories
            .iter()
            .enumerate()
            .any(|(index, category)| self.categories[..index].contains(category))
        {
            return Err(AppError::Validation(
                "A randomizer scope cannot contain duplicate categories".to_string(),
            ));
        }

        Ok(())
    }

    fn category_names(&self) -> Vec<&'static str> {
        self.categories
            .iter()
            .map(|category| category.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SuggestRandomModsInput {
    pub game_id: String,
    pub safety_filter: RandomizerSafetyFilter,
    pub scope: RandomizerScope,
    /// The last three selected ids per Object in the current modal session.
    pub recent_mod_ids_by_object: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub excluded_object_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct RandomizedLoadoutBackupInput {
    pub collection_name: String,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct ApplyRandomizedLoadoutInput {
    pub game_id: String,
    pub mod_ids: Vec<String>,
    pub safety_filter: RandomizerSafetyFilter,
    pub scope: RandomizerScope,
    pub backup: Option<RandomizedLoadoutBackupInput>,
    pub preview_fingerprint: String,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct PreviewRandomizedLoadoutInput {
    pub game_id: String,
    pub mod_ids: Vec<String>,
    pub safety_filter: RandomizerSafetyFilter,
    pub scope: RandomizerScope,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RandomizedLoadoutPreviewItem {
    pub object_id: String,
    pub object_name: String,
    pub object_type: Option<String>,
    pub mode: RandomizerLoadoutMode,
    pub selected_mod_name: String,
    pub selected_mod_id: String,
    pub active_mod_names: Vec<String>,
    pub disable_count: u32,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RandomizedLoadoutPreview {
    pub fingerprint: String,
    pub items: Vec<RandomizedLoadoutPreviewItem>,
    pub enable_count: u32,
    pub disable_count: u32,
    pub unsafe_mod_names: Vec<String>,
    pub runtime_conflicts:
        Vec<crate::modules::workspace::application::scanner::conflict::ConflictInfo>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RandomizedLoadoutBackupResult {
    pub collection_id: String,
    pub collection_name: String,
    pub reused: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ApplyRandomizedLoadoutResult {
    pub impact: crate::modules::workspace::domain::workspace::WorkspaceImpact,
    pub backup: Option<RandomizedLoadoutBackupResult>,
    pub sync_warning:
        Option<crate::modules::reconciliation::application::disk_reconcile::types::CommittedMutationSyncWarning>,
    pub history_warning: Option<String>,
}

fn path_has_hidden_segment(path: &str) -> bool {
    path.split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .any(|segment| segment.starts_with('.'))
}

fn path_is_effectively_disabled(path: &str) -> bool {
    path.split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .any(crate::modules::workspace::domain::normalizer::is_disabled_folder)
}

fn is_randomizer_candidate(status: ItemStatus, folder_path: &str) -> bool {
    if path_has_hidden_segment(folder_path) {
        return false;
    }

    // Reconciliation stores a terminal folder's own status, while a disabled
    // parent can make an otherwise enabled row inactive. Treat both forms as
    // eligible so randomizer proposals represent the runtime state on disk.
    status == ItemStatus::Disabled || path_is_effectively_disabled(folder_path)
}

pub async fn suggest_random_mods(
    pool: &SqlitePool,
    input: &SuggestRandomModsInput,
) -> Result<Vec<RandomModProposal>, AppError> {
    use rand::seq::SliceRandom;

    input.scope.validate()?;
    let persisted_history =
        crate::modules::library::adapters::sqlite::mods::get_recent_randomizer_history(
            pool,
            &input.game_id,
        )
        .await?;
    let excluded = input
        .excluded_object_ids
        .iter()
        .collect::<std::collections::HashSet<_>>();
    let category_names = input.scope.category_names();
    let candidates = crate::modules::library::adapters::sqlite::mods::get_randomizer_candidates(
        pool,
        &input.game_id,
        &category_names,
        input.scope.include_unclassified,
    )
    .await?;
    let mods_root =
        crate::modules::games::adapters::sqlite::game::get_mod_path(pool, &input.game_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Game not found or has no mods path".to_string()))?;
    let mut candidates_by_object = BTreeMap::<String, Vec<_>>::new();

    for candidate in candidates {
        if is_randomizer_candidate(candidate.status, &candidate.folder_path)
            && input.safety_filter.allows(candidate.is_safe)
            && !excluded.contains(&candidate.object_id)
        {
            candidates_by_object
                .entry(candidate.object_id.clone())
                .or_default()
                .push(candidate);
        }
    }

    let selected = {
        let mut rng = rand::thread_rng();
        candidates_by_object
            .into_iter()
            .filter_map(|(object_id, candidates)| {
                let recent = input
                    .recent_mod_ids_by_object
                    .get(&object_id)
                    .into_iter()
                    .flatten()
                    .chain(persisted_history.get(&object_id).into_iter().flatten())
                    .collect::<std::collections::HashSet<_>>();
                let alternatives = candidates
                    .iter()
                    .filter(|candidate| !recent.contains(&candidate.id))
                    .collect::<Vec<_>>();
                let selection_pool = if alternatives.is_empty() {
                    candidates.iter().collect::<Vec<_>>()
                } else {
                    alternatives
                };
                selection_pool
                    .choose(&mut rng)
                    .map(|candidate| (object_id, (*candidate).clone()))
            })
            .collect::<Vec<_>>()
    };

    let mut proposals = Vec::with_capacity(selected.len());
    for (object_id, candidate) in selected {
        let stored_path = Path::new(&candidate.folder_path);
        let absolute_path = if stored_path.is_absolute() {
            stored_path.to_path_buf()
        } else {
            Path::new(&mods_root).join(stored_path)
        };
        let thumbnail_path =
            ThumbnailCache::resolve(&input.game_id, &absolute_path.to_string_lossy())
                .await
                .ok()
                .flatten();
        proposals.push(RandomModProposal {
            active_mod_names:
                crate::modules::library::adapters::sqlite::mods::get_effectively_enabled_mod_names(
                    pool,
                    &input.game_id,
                    &object_id,
                )
                .await?,
            mode: effective_mode(
                candidate.object_type.as_deref(),
                candidate.randomizer_mode.as_deref(),
            ),
            is_safe: candidate.is_safe,
            object_id,
            object_name: candidate.object_name,
            object_type: candidate.object_type,
            mod_id: candidate.id,
            name: candidate.actual_name,
            thumbnail_path,
            folder_path: candidate.folder_path,
        });
    }

    Ok(proposals)
}

async fn validated_randomizer_candidates(
    pool: &SqlitePool,
    game_id: &str,
    mod_ids: &[String],
    safety_filter: RandomizerSafetyFilter,
    scope: &RandomizerScope,
) -> Result<Vec<crate::modules::library::adapters::sqlite::mods::RandomizerModCandidate>, AppError>
{
    scope.validate()?;
    if mod_ids.is_empty() {
        return Err(AppError::Validation(
            "Select at least one random mod before applying".to_string(),
        ));
    }
    let unique_ids = mod_ids.iter().collect::<std::collections::HashSet<_>>();
    if unique_ids.len() != mod_ids.len() {
        return Err(AppError::Validation(
            "A randomized loadout cannot contain the same mod twice".to_string(),
        ));
    }
    let category_names = scope.category_names();
    let candidates =
        crate::modules::library::adapters::sqlite::mods::get_randomizer_candidates_by_ids(
            pool,
            game_id,
            mod_ids,
            &category_names,
            scope.include_unclassified,
        )
        .await?;
    if candidates.len() != mod_ids.len() {
        return Err(AppError::Validation(
            "One or more selected mods are no longer eligible for this game".to_string(),
        ));
    }
    let mut seen_objects = std::collections::HashSet::new();
    for candidate in &candidates {
        if !is_randomizer_candidate(candidate.status, &candidate.folder_path)
            || !safety_filter.allows(candidate.is_safe)
        {
            return Err(AppError::Validation(format!(
                "Selected mod '{}' no longer matches the randomizer filter",
                candidate.actual_name
            )));
        }
        if !seen_objects.insert(&candidate.object_id) {
            return Err(AppError::Validation(
                "A randomized loadout can select only one mod per Object".to_string(),
            ));
        }
    }
    Ok(candidates)
}

pub async fn preview_randomized_loadout(
    config: &ConfigService,
    pool: &SqlitePool,
    input: &PreviewRandomizedLoadoutInput,
) -> Result<RandomizedLoadoutPreview, AppError> {
    let candidates = validated_randomizer_candidates(
        pool,
        &input.game_id,
        &input.mod_ids,
        input.safety_filter,
        &input.scope,
    )
    .await?;
    let target_paths = candidates
        .iter()
        .map(|candidate| candidate.folder_path.clone())
        .collect::<Vec<_>>();
    let exclusive_object_ids = candidates
        .iter()
        .filter(|candidate| {
            effective_mode(
                candidate.object_type.as_deref(),
                candidate.randomizer_mode.as_deref(),
            ) == RandomizerLoadoutMode::Exclusive
        })
        .map(|candidate| candidate.object_id.clone())
        .collect::<std::collections::HashSet<_>>();
    let planned = crate::modules::workspace::application::workspace::switch::prepare_randomized_loadout_switch(
        config,
        pool,
        &input.game_id,
        &target_paths,
        &exclusive_object_ids,
    )
    .await?;
    let mods_root =
        crate::modules::games::adapters::sqlite::game::get_mod_path(pool, &input.game_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Game {} has no mods path", input.game_id))
            })?;
    let mut projected_paths =
        crate::modules::library::adapters::sqlite::mods::get_enabled_mods_paths(
            pool,
            &input.game_id,
        )
        .await?;
    let mut items = Vec::with_capacity(candidates.len());
    let mut unsafe_mod_names = Vec::new();
    let mut fingerprint_parts = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let active_mod_names =
            crate::modules::library::adapters::sqlite::mods::get_effectively_enabled_mod_names(
                pool,
                &input.game_id,
                &candidate.object_id,
            )
            .await?;
        let mode = effective_mode(
            candidate.object_type.as_deref(),
            candidate.randomizer_mode.as_deref(),
        );
        let disable_count = if mode == RandomizerLoadoutMode::Exclusive {
            active_mod_names.len() as u32
        } else {
            0
        };
        if mode == RandomizerLoadoutMode::Exclusive {
            let siblings =
                crate::modules::library::adapters::sqlite::mods::get_enabled_siblings_paths(
                    pool,
                    &candidate.object_id,
                    &input.game_id,
                    Some(&candidate.id),
                )
                .await?;
            projected_paths
                .retain(|path| !siblings.iter().any(|sibling| sibling == path.as_stored()));
        }
        if !projected_paths
            .iter()
            .any(|path| path.as_stored() == candidate.folder_path)
        {
            projected_paths.push(
                crate::modules::system::domain::mod_path::ModFolderPath::from_stored(
                    candidate.folder_path.clone(),
                ),
            );
        }
        if !candidate.is_safe {
            unsafe_mod_names.push(candidate.actual_name.clone());
        }
        items.push(RandomizedLoadoutPreviewItem {
            object_id: candidate.object_id,
            object_name: candidate.object_name,
            object_type: candidate.object_type,
            mode,
            selected_mod_name: candidate.actual_name,
            selected_mod_id: candidate.id,
            active_mod_names,
            disable_count,
        });
    }
    fingerprint_parts.extend(planned.journal_steps().into_iter().map(
        |(sequence, old_path, new_path)| {
            format!(
                "{sequence}:{}:{}",
                old_path.to_string_lossy(),
                new_path.to_string_lossy()
            )
        },
    ));
    fingerprint_parts.push(format!("{:?}:{:?}", input.safety_filter, input.scope));
    fingerprint_parts.sort();
    let fingerprint = blake3::hash(fingerprint_parts.join("\u{1e}").as_bytes())
        .to_hex()
        .to_string();
    let runtime_conflicts = conflicts_for_enabled_paths(Path::new(&mods_root), &projected_paths);
    let disable_count = items.iter().map(|item| item.disable_count).sum();
    Ok(RandomizedLoadoutPreview {
        fingerprint,
        enable_count: items.len() as u32,
        disable_count,
        items,
        unsafe_mod_names,
        runtime_conflicts,
    })
}

/// Re-resolves a browser proposal immediately before a randomizer mutation.
/// Client-owned ids are never trusted as paths or as proof of eligibility.
pub async fn validate_randomized_loadout(
    pool: &SqlitePool,
    input: &ApplyRandomizedLoadoutInput,
) -> Result<Vec<String>, AppError> {
    Ok(validated_randomizer_candidates(
        pool,
        &input.game_id,
        &input.mod_ids,
        input.safety_filter,
        &input.scope,
    )
    .await?
    .into_iter()
    .map(|candidate| candidate.folder_path)
    .collect())
}

pub async fn get_active_mod_conflicts(
    pool: &SqlitePool,
    game_id: &str,
) -> Result<Vec<crate::modules::workspace::application::scanner::conflict::ConflictInfo>, AppError>
{
    let mods_path = crate::modules::games::adapters::sqlite::game::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} has no mods path")))?;
    let rows =
        crate::modules::library::adapters::sqlite::mods::get_enabled_mods_paths(pool, game_id)
            .await?;

    Ok(conflicts_for_enabled_paths(Path::new(&mods_path), &rows))
}

/// Conflict detection over an enabled-mod path list the caller already has.
///
/// Post-apply needs both the conflicts and the same path list for its harvest;
/// without this it issued the identical query twice.
///
/// `enabled_paths` are `mods.folder_path` values, which disk reconcile writes
/// relative to the mods root. This used to test them with `Path::exists`
/// directly: a relative path resolves against the process working directory,
/// so every such row failed the check and was skipped, and the whole feature
/// reported "no conflicts" without ever reading a file. `join` handles both
/// conventions -- an absolute argument replaces the base -- which matters
/// while the scanner commit still writes absolute paths.
pub fn conflicts_for_enabled_paths(
    mods_root: &Path,
    enabled_paths: &[crate::modules::system::domain::mod_path::ModFolderPath],
) -> Vec<crate::modules::workspace::application::scanner::conflict::ConflictInfo> {
    let mut ini_files: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    let mut mod_roots = Vec::new();
    for stored in enabled_paths {
        let path = stored.resolve(mods_root);
        if !path.exists() {
            continue;
        }
        mod_roots.push(path.clone());
        let discovered =
            crate::modules::workspace::application::scanner::conflict::discover_runtime_ini_files(
                &path,
            );
        for ini in discovered {
            ini_files.push((path.clone(), ini));
        }
    }

    crate::modules::workspace::application::scanner::conflict::detect_conflicts_with_roots(
        &ini_files, &mod_roots,
    )
}

#[cfg(test)]
#[path = "tests/metadata_conflict_tests.rs"]
mod metadata_conflict_tests;
