//! Resolving a naming conflict where "X" and "DISABLED X" both exist on disk.
//!
//! The orchestration lived in the Tauri command: it took the lock, renamed the
//! folder, wrote two repos directly and settled the runtime itself. That is
//! the same sequence `rename` performs one module over, so it belongs at the
//! same altitude.

use std::fs;
use std::path::Path;

use crate::common::normalizer::{is_disabled_folder, normalize_display_name};
use crate::common::path_key::relative_to_root;
use crate::domain::errors::AppError;
use crate::services::app::runtime_effects::{finalize_mutation, MutationOutcome};
use crate::services::config::ConfigService;
use crate::services::fs_utils::guard::ValidatedPath;
use crate::services::fs_utils::operation_lock::OpGuard;
use crate::services::scanner::watcher::{SuppressionGuard, WatcherState};

/// How to break a duplicate pair apart.
///
/// An enum rather than the `&str` the command used to match on: an unknown
/// spelling used to surface as a runtime "Unknown strategy" error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    /// Keep the enabled folder; uniquify the disabled duplicate.
    KeepEnabled,
    /// Keep the disabled folder; uniquify the enabled duplicate.
    KeepDisabled,
    /// Keep both, renaming the duplicate's base name to "<base> (copy)".
    Separate,
}

/// Rename the duplicate so the pair no longer collides, then settle the index.
///
/// Returns the duplicate's new absolute path.
pub struct ResolveConflictRequest<'a> {
    pub config: &'a ConfigService,
    pub pool: &'a sqlx::SqlitePool,
    pub state: &'a WatcherState,
    pub op_guard: &'a OpGuard,
    pub game_id: &'a str,
    /// The folder that keeps its name.
    pub keep: &'a ValidatedPath,
    /// The folder that gets renamed out of the way.
    pub duplicate: &'a ValidatedPath,
    pub strategy: ConflictStrategy,
}

pub async fn resolve_naming_conflict(
    request: ResolveConflictRequest<'_>,
) -> Result<String, AppError> {
    let ResolveConflictRequest {
        config,
        pool,
        state,
        op_guard: _,
        game_id,
        keep,
        duplicate,
        strategy,
    } = request;

    let mods_path = crate::repo::game_repo::get_mod_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Game not found".to_string()))?;
    let mods_root = Path::new(&mods_path);

    let old_rel = relative_to_root(duplicate.original(), mods_root);
    let renamed = rename_duplicate(state, keep, duplicate, strategy)?;
    let new_rel = relative_to_root(&renamed, mods_root);

    if old_rel != new_rel {
        if let Err(error) = crate::repo::mod_repo::update_mod_path_by_old_path_in_game(
            pool, game_id, &old_rel, &new_rel,
        )
        .await
        {
            log::warn!(
                "Conflict resolution left the index stale ({old_rel} -> {new_rel}): {error}"
            );
        }

        // Collection auto-healing: cascade the rename into saved collections.
        let _ = crate::services::collection_service::handle_mod_moved_or_renamed(
            pool, &old_rel, &new_rel, None,
        )
        .await;
    }

    // A resolution renames a folder, which can move it between objects, so the
    // affected set is not enumerable here.
    finalize_mutation(pool, config, game_id, MutationOutcome::full_game()).await;

    Ok(renamed)
}

/// The filesystem half: pick a free name and move the duplicate onto it.
pub fn rename_duplicate(
    state: &WatcherState,
    keep: &Path,
    duplicate: &Path,
    strategy: ConflictStrategy,
) -> Result<String, AppError> {
    if !keep.exists() {
        return Err(AppError::Io(format!(
            "Keep path does not exist: {}",
            keep.display()
        )));
    }
    if !duplicate.exists() {
        return Err(AppError::Io(format!(
            "Duplicate path does not exist: {}",
            duplicate.display()
        )));
    }

    let parent = duplicate.parent().unwrap_or_else(|| Path::new(""));
    let dup_name = duplicate.file_name().unwrap_or_default().to_string_lossy();
    let base = normalize_display_name(&dup_name);
    let is_disabled = is_disabled_folder(&dup_name);

    let new_name = match strategy {
        ConflictStrategy::KeepEnabled | ConflictStrategy::KeepDisabled => {
            find_unique_name(parent, &base, is_disabled)
        }
        ConflictStrategy::Separate => {
            find_unique_name(parent, &format!("{base} (copy)"), is_disabled)
        }
    };

    let new_path = parent.join(&new_name);
    if new_path.exists() {
        return Err(AppError::Io(format!(
            "Target already exists: {}",
            new_path.display()
        )));
    }

    {
        let _guard = SuppressionGuard::new(&state.suppressor);
        fs::rename(duplicate, &new_path)
            .map_err(|e| AppError::Io(format!("Failed to rename duplicate: {e}")))?;
    }

    log::info!(
        "Resolved conflict: '{}' → '{}'",
        dup_name,
        new_path.display()
    );

    Ok(new_path.to_string_lossy().to_string())
}

/// Find a free folder name, keeping the prefix state.
///
/// Always suffixed: the bare base name is what the *kept* folder holds, so
/// returning it unchanged would recreate the collision this is breaking.
pub(crate) fn find_unique_name(parent: &Path, base: &str, is_disabled: bool) -> String {
    let prefix = if is_disabled {
        crate::DISABLED_PREFIX
    } else {
        ""
    };

    let candidate = format!("{prefix}{base} (dup)");
    if !parent.join(&candidate).exists() {
        return candidate;
    }

    for n in 2..100 {
        let candidate = format!("{prefix}{base} (dup {n})");
        if !parent.join(&candidate).exists() {
            return candidate;
        }
    }

    // Fallback: a timestamp is collision-free enough for a 100-way pileup.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{prefix}{base} (dup {stamp})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_unique_name() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path();

        // 1st duplication
        let name1 = find_unique_name(parent, "TestMod", false);
        assert_eq!(name1, "TestMod (dup)");

        // Simulate creation of first dup
        fs::create_dir(parent.join(&name1)).unwrap();

        // 2nd duplication
        let name2 = find_unique_name(parent, "TestMod", false);
        assert_eq!(name2, "TestMod (dup 2)");

        // Disabled duplication
        let name_disabled = find_unique_name(parent, "TestMod", true);
        assert_eq!(
            name_disabled,
            format!("{}TestMod (dup)", crate::DISABLED_PREFIX)
        );
    }

    #[test]
    fn test_resolve_conflict_keep_enabled() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path();
        let watcher = WatcherState::new();

        let enabled_mod = parent.join("ConflictMod");
        let disabled_mod = parent.join(format!("{}ConflictMod", crate::DISABLED_PREFIX));

        fs::create_dir(&enabled_mod).unwrap();
        fs::create_dir(&disabled_mod).unwrap();

        let res = rename_duplicate(
            &watcher,
            &enabled_mod,
            &disabled_mod,
            ConflictStrategy::KeepEnabled,
        )
        .unwrap();

        // Strategy was keep_enabled, so disabled_mod was renamed to DISABLED ConflictMod (dup)
        assert_eq!(
            res,
            parent
                .join(format!("{}ConflictMod (dup)", crate::DISABLED_PREFIX))
                .to_string_lossy()
        );

        // Verify rename happened
        assert!(!disabled_mod.exists());
        assert!(Path::new(&res).exists());
        assert!(enabled_mod.exists()); // keep didn't move
    }

    #[test]
    fn test_resolve_conflict_separate() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path();
        let watcher = WatcherState::new();

        let enabled_mod = parent.join("ConflictMod");
        let disabled_mod = parent.join(format!("{}ConflictMod", crate::DISABLED_PREFIX));

        fs::create_dir(&enabled_mod).unwrap();
        fs::create_dir(&disabled_mod).unwrap();

        let res = rename_duplicate(
            &watcher,
            &enabled_mod,
            &disabled_mod,
            ConflictStrategy::Separate,
        )
        .unwrap();

        // Strategy was separate, so disabled duplicate was renamed to base (copy)
        assert_eq!(
            res,
            parent
                .join(format!(
                    "{}ConflictMod (copy) (dup)",
                    crate::DISABLED_PREFIX
                ))
                .to_string_lossy()
        );

        assert!(!disabled_mod.exists());
        assert!(Path::new(&res).exists());
        assert!(enabled_mod.exists());
    }
}
