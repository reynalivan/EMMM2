//! Orchestrates the two-phase commit of confirmed scan results.

use crate::domain::errors::ScannerError;
use crate::repo::mod_repo;
use crate::services::scanner::sync::helpers::ensure_game_exists;
use crate::services::scanner::sync::types::SyncResult;

use super::execute::execute_entries;
use super::linking::link_disk_to_db;
use super::request::{CommitCtx, CommitScanRequest};
use super::temp_move::prepare_disk_entries;

/// Phase 2: Commit user-confirmed scan results to DB (Two-Phase Diffing).
pub async fn commit_scan_results(
    request: CommitScanRequest<'_>,
) -> Result<SyncResult, ScannerError> {
    let pool = request.pool;
    let game_id = request.game_id;
    let game_name = request.game_name;
    let game_type = request.game_type;
    let mods_path = request.mods_path;
    let items = request.items;
    let resource_dir = request.resource_dir;
    let _ = resource_dir; // reserved for future thumbnail resolution

    let ctx = CommitCtx {
        game_id,
        mods_path,
        safe_mode_keywords: request.safe_mode_keywords,
        preserve_existing_mappings: request.preserve_existing_mappings,
    };

    let mut tx = pool.begin().await?;

    ensure_game_exists(&mut tx, game_id, game_name, game_type, mods_path).await?;

    let mut new_objects_count = 0;
    let (disk_entries, collisions) =
        prepare_disk_entries(&mut tx, &ctx, items, &mut new_objects_count).await?;

    let total = disk_entries.len();

    // Fetch snapshot of DB state
    let db_mods = mod_repo::get_all_mods_sync_info_tx(&mut tx, game_id).await?;

    let disk_to_db = link_disk_to_db(&disk_entries, &db_mods, std::path::Path::new(ctx.mods_path));

    let (new_mods_count, updated_mods_count) = execute_entries(
        &mut tx,
        &ctx,
        disk_entries,
        &db_mods,
        &disk_to_db,
        &mut new_objects_count,
    )
    .await?;

    // Scan commit is an explicit import/scan commit, not passive filesystem repair.
    // Disk Reconcile owns cleanup of DB rows whose folders disappeared outside this flow.
    let deleted_mods_count = 0;

    crate::repo::object_repo::delete_ghost_objects_gc(&mut tx, game_id).await?;

    tx.commit().await?;

    let temp_dir_path = std::path::Path::new(mods_path).join(".emmm_temp");
    if temp_dir_path.exists() {
        let _ = std::fs::remove_dir(&temp_dir_path);
    }

    Ok(SyncResult {
        total_scanned: total,
        new_mods: new_mods_count,
        updated_mods: updated_mods_count,
        deleted_mods: deleted_mods_count,
        new_objects: new_objects_count,
        collisions,
    })
}
