use std::path::Path;

use crate::domain::workspace::{
    WorkspaceRuntime, WorkspaceSelection, WorkspaceSourceState, WorkspaceSourceStatus,
    WorkspaceViewModel, WorkspaceViewModelInput,
};
use crate::services::explorer::helpers::apply_runtime_corridor_filter_to_response;
use crate::services::explorer::listing::list_mod_folders_for_game;
use crate::services::objects::query::get_filtered_objects_with_conflict_check;
use crate::services::workspace_read_model::explorer_mapper::{
    empty_workspace_explorer, map_workspace_explorer,
};
use crate::services::workspace_read_model::object_mapper::map_workspace_objects;
use crate::services::workspace_read_model::preview_builder::{
    build_preview, clear_preview_selection_for_corridor_mismatch, empty_workspace_preview,
};
use crate::services::workspace_read_model::selection::{
    build_current_path, resolve_unavailable_workspace_selection, resolve_workspace_selection,
    ResolvedWorkspaceSelection,
};

async fn load_game_mods_path(pool: &sqlx::SqlitePool, game_id: &str) -> Result<String, String> {
    sqlx::query_scalar::<_, String>("SELECT mods_path FROM games WHERE id = ?")
        .bind(game_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Game '{}' has no mods_path", game_id))
}

fn available_source_state() -> WorkspaceSourceState {
    WorkspaceSourceState {
        status: WorkspaceSourceStatus::Available,
        message: None,
    }
}

fn unavailable_source_state(mods_path: &str) -> WorkspaceSourceState {
    WorkspaceSourceState {
        status: WorkspaceSourceStatus::Unavailable,
        message: Some(format!("Mods root is unavailable: {mods_path}")),
    }
}

fn build_workspace_selection(
    resolved_selection: &ResolvedWorkspaceSelection,
    preview_selected_path: Option<String>,
) -> WorkspaceSelection {
    WorkspaceSelection {
        selected_object_folder_path: resolved_selection.selected_object_folder_path.clone(),
        explorer_sub_path: resolved_selection.explorer_sub_path.clone(),
        selected_mod_path: preview_selected_path,
        current_path: build_current_path(
            resolved_selection.selected_object_folder_path.as_deref(),
            resolved_selection.explorer_sub_path.as_deref(),
        ),
        reconciliation_status: resolved_selection.reconciliation_status,
        reconciliation_reason: resolved_selection.reconciliation_reason,
        affected_paths: resolved_selection.affected_paths.clone(),
    }
}

pub async fn get_workspace_view_model(
    pool: &sqlx::SqlitePool,
    input: WorkspaceViewModelInput,
) -> Result<WorkspaceViewModel, String> {
    let game_id = input.filter.game_id.clone();
    let safe_mode = input.filter.safe_mode;
    let mods_path = load_game_mods_path(pool, &game_id).await?;
    let objects = get_filtered_objects_with_conflict_check(pool, &input.filter)
        .await?
        .objects;

    if !Path::new(&mods_path).is_dir() {
        let resolved_selection = resolve_unavailable_workspace_selection(&input);
        let preview = empty_workspace_preview();
        let selection =
            build_workspace_selection(&resolved_selection, preview.selected_path.clone());

        return Ok(WorkspaceViewModel {
            objects: map_workspace_objects(objects, &mods_path, false),
            explorer: empty_workspace_explorer(),
            preview,
            selection,
            runtime: WorkspaceRuntime {
                game_id,
                safe_mode,
                source_state: unavailable_source_state(&mods_path),
            },
        });
    }

    let mut resolved_selection = resolve_workspace_selection(&mods_path, &input);
    let raw_explorer = list_mod_folders_for_game(
        pool,
        &game_id,
        mods_path.clone(),
        resolved_selection.explorer_sub_path.clone(),
    )
    .await?;
    let explorer = map_workspace_explorer(apply_runtime_corridor_filter_to_response(
        raw_explorer,
        safe_mode,
    ));
    let preview = build_preview(
        &explorer,
        resolved_selection.explorer_sub_path.as_deref(),
        &mods_path,
        resolved_selection.selected_mod_path.as_deref(),
        safe_mode,
    );
    clear_preview_selection_for_corridor_mismatch(&mut resolved_selection, &preview);
    let selection = build_workspace_selection(&resolved_selection, preview.selected_path.clone());

    Ok(WorkspaceViewModel {
        objects: map_workspace_objects(objects, &mods_path, true),
        explorer,
        preview,
        selection,
        runtime: WorkspaceRuntime {
            game_id,
            safe_mode,
            source_state: available_source_state(),
        },
    })
}

#[cfg(test)]
#[path = "workspace_service_tests/mod.rs"]
mod tests;
