//! Stale mod path healing service.
//!
//! Resolves the filesystem path for a mod that belongs to a given object,
//! cleaning up stale DB rows where the folder no longer exists on disk.

use std::path::Path;

/// Resolve the filesystem folder path for a mod associated with `object_id`.
///
/// - If the path exists on disk: returns `Some(path)`.
/// - If the path is in the DB but the folder is gone: deletes the stale row, returns `None`.
/// - If no mod row exists: returns `None`.
pub async fn resolve_mod_path_for_object(
    pool: &sqlx::SqlitePool,
    object_id: &str,
) -> Option<String> {
    let (mod_id, folder_path) = crate::database::mod_repo::get_mod_by_object_id(pool, object_id)
        .await
        .ok()??;

    let path = Path::new(&folder_path);
    if path.exists() {
        return Some(folder_path);
    }

    // Filesystem is source of truth — delete the stale row
    let _ = crate::database::mod_repo::delete_mod_by_id(pool, &mod_id).await;

    log::warn!(
        "Deleted stale mod {} (folder gone): {}",
        mod_id,
        folder_path
    );
    None
}
