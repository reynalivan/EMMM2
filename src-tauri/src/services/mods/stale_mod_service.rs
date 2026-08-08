//! Stale mod path healing service.
//!
//! Resolves the filesystem path for a mod that belongs to a given object,
//! cleaning up stale DB rows where the folder no longer exists on disk.

use std::path::Path;

/// Resolve the filesystem folder path for a mod associated with `object_id`.
///
/// - If the path exists on disk: returns `Some(absolute path)`.
/// - If the path is in the DB but the folder is gone: deletes the stale row, returns `None`.
/// - If no mod row exists: returns `None`.
///
/// The existence check has to resolve against `mods_root` before it means
/// anything. Testing the stored value directly resolved it against the process
/// working directory, so every row looked gone — and this function's answer to
/// "gone" is to delete it. A read-only action like revealing a folder in
/// Explorer was quietly dropping index rows, taking the favourite flag and the
/// manual safe-mode classification with them.
pub async fn resolve_mod_path_for_object(
    pool: &sqlx::SqlitePool,
    object_id: &str,
    mods_root: &Path,
) -> Option<String> {
    let (mod_id, stored_path) = crate::repo::mod_repo::get_mod_by_object_id(pool, object_id)
        .await
        .ok()??;

    let path = stored_path.resolve(mods_root);
    if path.exists() {
        return Some(path.to_string_lossy().to_string());
    }

    // Filesystem is source of truth — delete the stale row
    let _ = crate::repo::mod_repo::delete_mod_by_id(pool, &mod_id).await;

    log::warn!(
        "Deleted stale mod {} (folder gone): {}",
        mod_id,
        stored_path
    );
    None
}
