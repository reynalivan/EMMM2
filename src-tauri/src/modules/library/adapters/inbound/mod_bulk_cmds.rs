use crate::shared::errors::AppError;
use crate::modules::system::application::config::ConfigService;
use crate::platform::fs::operation_lock::OperationLock;
use crate::modules::library::application::mods::bulk;
use crate::modules::library::application::mods::info_json;
use crate::modules::workspace::application::scanner::watcher::WatcherState;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, State};

/// Cooperative cancel for the two bulk actions that walk the filesystem one
/// folder at a time. A single flag is enough: `OperationLock` already
/// serializes bulk runs, so two batches are never in flight together.
#[derive(Default)]
pub struct BulkCancelState(AtomicBool);

impl BulkCancelState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears a cancel left over from an earlier batch and hands back the flag.
    /// Callers already hold the operation lock, so this cannot wipe a cancel
    /// aimed at a run that is still going.
    fn begin(&self) -> &AtomicBool {
        self.0.store(false, Ordering::SeqCst);
        &self.0
    }

    fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn apply_committed_reconcile(
    result: &mut bulk::BulkResult,
    settlement: crate::modules::workspace::application::disk_reconcile::emit::CommittedReconcileSettlement,
) {
    if let Some(reconcile) = settlement.reconcile {
        result
            .collection_impact
            .merge(reconcile.collection_reference_impact);
    }
    result.sync_warning = settlement.sync_warning;
}

/// Stop the running bulk toggle/delete after the item in flight. Work already
/// done stays done — the trailing reconcile still converges the DB.
#[specta::specta]
#[tauri::command]
pub async fn bulk_cancel(cancel_state: State<'_, BulkCancelState>) -> Result<(), AppError> {
    cancel_state.cancel();
    Ok(())
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn bulk_toggle_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
    enable: bool,
) -> Result<bulk::BulkResult, AppError> {
    // Security validation for all paths
    crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;

    let lock = op_lock.acquire().await?;
    let mut result = bulk::bulk_toggle(&app, &state, paths, enable, cancel_state.begin()).await?;
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command boundary keeps the existing IPC payload stable.
pub async fn bulk_delete_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    state: tauri::State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    cancel_state: State<'_, BulkCancelState>,
    game_id: String,
    paths: Vec<String>,
) -> Result<bulk::BulkResult, AppError> {
    // Required, like `delete_mod`: it names the mods root the paths must sit
    // inside, and the game whose index rows may be pruned. Optional, it let a
    // caller skip containment entirely.
    crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;

    let lock = op_lock.acquire().await?;
    let mut result = bulk::bulk_delete(&app, &state, paths, cancel_state.begin()).await?;
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_update_info(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    paths: Vec<String>,
    update: info_json::ModInfoUpdate,
) -> Result<bulk::BulkResult, AppError> {
    if update.is_safe.is_some() {
        return Err(AppError::Validation(
            "Safety changes must use bulk_set_mod_safety".to_string(),
        ));
    }
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;
    let lock = op_lock.acquire().await?;
    let suppression = watcher
        .suppressor
        .suppress_paths(validated.iter().map(AsRef::<std::path::Path>::as_ref));
    let mut result = bulk::bulk_update_info(&validated, update).await?;
    drop(suppression);
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }

    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_set_mod_safety(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    paths: Vec<String>,
    safe: bool,
) -> Result<bulk::BulkResult, AppError> {
    let validated = crate::platform::fs::guard::validate_paths(&config, &game_id, &paths)?;
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&paths),
    )
    .await?;

    let lock = op_lock.acquire().await?;
    let resolved = bulk::resolve_safety_targets(pool.inner(), &game_id, &validated).await?;
    let suppression = watcher.suppressor.suppress_paths(
        resolved
            .targets
            .iter()
            .map(|target| std::path::Path::new(&target.disk_path)),
    );
    let mut result = bulk::bulk_set_safety(pool.inner(), &game_id, resolved, safe).await?;
    drop(suppression);
    drop(lock);

    if !result.success.is_empty() {
        let settlement = crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_toggle_favorite(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    folder_paths: Vec<String>,
    favorite: bool,
) -> Result<bulk::BulkResult, AppError> {
    let validated =
        crate::platform::fs::guard::validate_paths(&config, &game_id, &folder_paths)?;
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&folder_paths),
    )
    .await?;
    let lock = op_lock.acquire().await?;
    let suppression = watcher
        .suppressor
        .suppress_paths(validated.iter().map(AsRef::<std::path::Path>::as_ref));
    let mut result =
        bulk::bulk_toggle_favorite(&pool, game_id.clone(), folder_paths, favorite).await?;
    drop(suppression);
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[specta::specta]
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri boundary: injected states plus the bulk payload.
pub async fn bulk_pin_mods(
    app: AppHandle,
    config: State<'_, ConfigService>,
    pool: tauri::State<'_, sqlx::SqlitePool>,
    watcher: State<'_, WatcherState>,
    op_lock: State<'_, OperationLock>,
    game_id: String,
    folder_paths: Vec<String>,
    pin: bool,
) -> Result<bulk::BulkResult, AppError> {
    let validated =
        crate::platform::fs::guard::validate_paths(&config, &game_id, &folder_paths)?;
    crate::modules::workspace::application::disk_reconcile::emit::ensure_mutation_preflight_for_paths(
        &app,
        pool.inner(),
        &game_id,
        Some(&folder_paths),
    )
    .await?;
    let lock = op_lock.acquire().await?;
    let suppression = watcher
        .suppressor
        .suppress_paths(validated.iter().map(AsRef::<std::path::Path>::as_ref));
    let mut result = bulk::bulk_pin(&pool, game_id.clone(), folder_paths, pin).await?;
    drop(suppression);
    drop(lock);
    if !result.success.is_empty() {
        let settlement = crate::modules::workspace::application::disk_reconcile::emit::settle_committed_reconcile(
            crate::modules::workspace::application::disk_reconcile::emit::run_internal_disk_reconcile(
                &app,
                pool.inner(),
                &game_id,
                result.success.clone(),
            )
            .await,
        );
        apply_committed_reconcile(&mut result, settlement);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_cancel_state_is_observed_and_reset_for_the_next_batch() {
        let state = BulkCancelState::new();

        assert!(!state.begin().load(Ordering::SeqCst));
        state.cancel();
        assert!(state.0.load(Ordering::SeqCst));
        assert!(!state.begin().load(Ordering::SeqCst));
    }
}
