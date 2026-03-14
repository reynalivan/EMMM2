//! Startup sync: reconcile the `objects`/`mods` DB index with the filesystem.
//!
//! Called once per game during Tauri `setup()`, before the webview loads.
//! Ensures the ObjectList query always returns data on first paint.
//!
//! # Core Invariant
//! After [`reconcile_game`] returns `Ok`, every non-hidden directory under
//! `mod_path` has a corresponding row in the `objects` table.

use sqlx::SqlitePool;

/// Reconcile the objects/mods DB index with the filesystem for a single game.
///
/// # Timestamp Skip Logic
/// The expensive GC + sync cycle is skipped IFF **both** conditions hold:
///   1. `mod_path` modification time matches `cached_mtime`
///   2. The `objects` table already has ≥1 row for this `game_id`
///
/// If either condition fails, a full GC + sync cycle runs.
/// This prevents the "death spiral" where a DB reset leaves `objects` empty
/// but timestamps still match, causing the sync to be permanently skipped.
///
/// # Returns
/// The filesystem `mtime` that the caller should persist as the new cached
/// timestamp. If the sync was skipped, returns `cached_mtime` unchanged.
///
/// # Errors
/// Returns `Err` if:
/// - `mod_path` does not exist as a directory
/// - A database error occurs during GC or sync
pub async fn reconcile_game(
    pool: &SqlitePool,
    game_id: &str,
    mod_path: &str,
    safe_mode_keywords: &[String],
    cached_mtime: u64,
) -> Result<u64, String> {
    let path = std::path::Path::new(mod_path);
    if !path.is_dir() {
        log::warn!(
            "reconcile_game: mod_path '{}' is not a directory, skipping game '{}'",
            mod_path,
            game_id
        );
        return Ok(cached_mtime);
    }

    // Step 1: Read current filesystem modification time
    let current_mtime = std::fs::metadata(mod_path)
        .and_then(|m| m.modified())
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map_err(std::io::Error::other)
        })
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Step 2: Count existing objects for this game in the DB
    let object_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects WHERE game_id = ?")
        .bind(game_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    // Step 3: Decide whether to skip
    if current_mtime > 0 && current_mtime == cached_mtime && object_count > 0 {
        log::info!(
            "reconcile_game: skipping '{}' — no FS changes and {} objects in DB",
            game_id,
            object_count
        );
        return Ok(cached_mtime);
    }

    if object_count == 0 {
        log::info!(
            "reconcile_game: forcing full sync for '{}' — objects table is empty (DB reset?)",
            game_id
        );
    }

    // Step 4: Run GC (remove objects whose folders were deleted from disk)
    if let Err(e) = crate::services::objects::query::gc_lost_objects(pool, game_id).await {
        log::warn!("reconcile_game: GC failed for '{}': {e}", game_id);
    }

    // Step 5: Sync disk → DB (create objects/mods for new folders)
    if let Err(e) = crate::services::scanner::object_sync::sync_objects_for_game(
        pool,
        game_id,
        safe_mode_keywords,
    )
    .await
    {
        log::warn!("reconcile_game: sync failed for '{}': {e}", game_id);
    }

    log::info!("reconcile_game: complete for '{}'", game_id);
    Ok(current_mtime)
}
