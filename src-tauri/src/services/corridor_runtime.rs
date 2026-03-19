use crate::database::corridor_state_repo;
use crate::services::collections;
use crate::services::collections::types::{
    CollectionPreviewMod, CollectionStateKind, CorridorRuntimeSnapshot, RuntimeObjectState,
};
use crate::services::config::ConfigService;
use crate::services::corridor_constants::{
    CORRIDOR_ALL_DISABLED_LABEL, CORRIDOR_UNSAVED_PRESET_LABEL,
};
use crate::services::corridor_types::{
    CorridorPreview, CorridorPreviewMod, CorridorPreviewStateKind,
};
use crate::services::scanner::watcher::WatcherState;
use sqlx::SqlitePool;

struct ResolvedTargetPreview {
    mods: Vec<CollectionPreviewMod>,
    object_states: Vec<RuntimeObjectState>,
    state_name: Option<String>,
    state_kind: CorridorPreviewStateKind,
}

pub async fn reconcile_current_corridor(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    game_id: &str,
    is_safe: bool,
) -> Result<usize, String> {
    collections::auto_disable_auto_tagged_outside_corridor(pool, watcher_state, game_id, is_safe)
        .await
}

pub async fn reconcile_active_game_corridor(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    config: &ConfigService,
) -> Result<usize, String> {
    let settings = config.get_settings();
    let Some(active_game_id) = settings.active_game_id else {
        return Ok(0);
    };

    reconcile_current_corridor(
        pool,
        watcher_state,
        &active_game_id,
        settings.safe_mode.enabled,
    )
    .await
}

pub async fn reconcile_if_active_game_corridor(
    pool: &SqlitePool,
    watcher_state: &WatcherState,
    config: &ConfigService,
    game_id: &str,
) -> Result<usize, String> {
    let settings = config.get_settings();
    if settings.active_game_id.as_deref() != Some(game_id) {
        return Ok(0);
    }

    reconcile_current_corridor(pool, watcher_state, game_id, settings.safe_mode.enabled).await
}

pub async fn get_corridor_runtime_snapshot(
    pool: &SqlitePool,
    game_id: &str,
    is_safe: bool,
) -> Result<CorridorRuntimeSnapshot, String> {
    collections::resolve_corridor_runtime_snapshot(pool, game_id, is_safe).await
}

pub async fn preview_corridor_switch(
    pool: &SqlitePool,
    game_id: &str,
    current_safe: bool,
    target_safe: bool,
) -> Result<CorridorPreview, String> {
    let (leaving_state_result, target_preview_result) = tokio::join!(
        get_corridor_runtime_snapshot(pool, game_id, current_safe),
        resolve_target_corridor_preview(pool, game_id, target_safe),
    );

    let leaving_state = leaving_state_result?;
    let target_preview = target_preview_result?;
    let leaving_mods = map_corridor_preview_mods(leaving_state.roots);
    let target_mods = map_corridor_preview_mods(target_preview.mods);
    let leaving_state_name = leaving_state
        .state_name
        .unwrap_or_else(|| CORRIDOR_UNSAVED_PRESET_LABEL.to_string());
    let leaving_state_kind = map_state_kind(leaving_state.state_kind);

    let target_description = if target_preview.state_kind == CorridorPreviewStateKind::None {
        CORRIDOR_ALL_DISABLED_LABEL.to_string()
    } else if target_mods.is_empty() {
        format!(
            "{} ({CORRIDOR_ALL_DISABLED_LABEL})",
            target_preview
                .state_name
                .as_deref()
                .unwrap_or(CORRIDOR_ALL_DISABLED_LABEL)
        )
    } else {
        format!("Restoring {} Mods", target_mods.len())
    };

    Ok(CorridorPreview {
        leaving_mods,
        leaving_object_states: leaving_state.object_states,
        leaving_state_name,
        leaving_state_kind,
        target_mods,
        target_object_states: target_preview.object_states,
        target_state_name: target_preview.state_name,
        target_state_kind: target_preview.state_kind,
        target_description,
    })
}

async fn resolve_target_corridor_preview(
    pool: &SqlitePool,
    game_id: &str,
    target_safe: bool,
) -> Result<ResolvedTargetPreview, String> {
    let state = corridor_state_repo::get_corridor_state(pool, game_id, target_safe)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(collection_id) = state.active_collection_id.as_deref() {
        let preview =
            collections::get_collection_runtime_preview(pool, collection_id, game_id).await?;
        let state_kind = if preview.collection.is_last_unsaved {
            CorridorPreviewStateKind::Unsaved
        } else {
            CorridorPreviewStateKind::Named
        };
        let state_name = if preview.collection.is_last_unsaved {
            CORRIDOR_UNSAVED_PRESET_LABEL.to_string()
        } else {
            preview.collection.name
        };

        return Ok(ResolvedTargetPreview {
            mods: preview.roots,
            object_states: preview.object_states,
            state_name: Some(state_name),
            state_kind,
        });
    }

    if let Some(snapshot_id) = state.undo_collection_id.as_deref() {
        let preview =
            collections::get_collection_runtime_preview(pool, snapshot_id, game_id).await?;
        return Ok(ResolvedTargetPreview {
            mods: preview.roots,
            object_states: preview.object_states,
            state_name: Some(CORRIDOR_UNSAVED_PRESET_LABEL.to_string()),
            state_kind: CorridorPreviewStateKind::Unsaved,
        });
    }

    Ok(ResolvedTargetPreview {
        mods: Vec::new(),
        object_states: Vec::new(),
        state_name: None,
        state_kind: CorridorPreviewStateKind::None,
    })
}

fn map_state_kind(kind: CollectionStateKind) -> CorridorPreviewStateKind {
    match kind {
        CollectionStateKind::Named => CorridorPreviewStateKind::Named,
        CollectionStateKind::Unsaved => CorridorPreviewStateKind::Unsaved,
        CollectionStateKind::None => CorridorPreviewStateKind::None,
    }
}

fn map_corridor_preview_mods(mods: Vec<CollectionPreviewMod>) -> Vec<CorridorPreviewMod> {
    mods.into_iter()
        .map(|mod_item| CorridorPreviewMod {
            id: mod_item.id,
            actual_name: mod_item.actual_name,
            folder_path: mod_item.folder_path,
            is_safe: mod_item.is_safe,
            object_id: mod_item.object_id,
            object_name: mod_item.object_name,
            object_type: mod_item.object_type,
            node_type: mod_item
                .node_type
                .unwrap_or_else(|| "FlatModRoot".to_string()),
        })
        .collect()
}
