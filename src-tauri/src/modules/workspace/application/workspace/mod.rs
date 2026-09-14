use std::path::Path;

use crate::modules::catalog::application::objects::query::get_filtered_objects_with_conflict_check;
use crate::modules::workspace::application::explorer::listing::{
    list_mod_folders_for_game_shallow_with_enrichment, list_mod_folders_for_game_with_enrichment,
    load_listing_enrichment,
};
use crate::modules::workspace::application::workspace_read_model::explorer_mapper::{
    empty_workspace_explorer, map_workspace_explorer,
};
use crate::modules::workspace::application::workspace_read_model::object_mapper::{
    map_workspace_objects, WorkspaceObjectMapping,
};
use crate::modules::workspace::application::workspace_read_model::preview_builder::read_preview;
use crate::modules::workspace::application::workspace_read_model::selection::{
    build_current_path, resolve_unavailable_workspace_selection, resolve_workspace_selection,
    ResolvedWorkspaceSelection,
};
use crate::modules::workspace::domain::workspace::{
    WorkspaceNavigationSelection, WorkspacePreviewInput, WorkspacePreviewRequestIdentity,
    WorkspacePreviewResult, WorkspaceRuntime, WorkspaceSourceState, WorkspaceSourceStatus,
    WorkspaceStructureInput, WorkspaceStructureViewModel,
};
use crate::shared::errors::AppError;

async fn load_game_mods_path(pool: &sqlx::SqlitePool, game_id: &str) -> Result<String, AppError> {
    crate::modules::games::adapters::sqlite::game::get_configured_mods_path(pool, game_id)
        .await?
        .ok_or_else(|| AppError::Internal(format!("Game '{}' has no mods_path", game_id)))
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

fn include_unregistered_roots(
    filter: &crate::modules::catalog::domain::objects::ObjectFilter,
) -> bool {
    filter.object_type.is_none()
        && filter.status_filter.is_none()
        && filter
            .meta_filters
            .as_ref()
            .is_none_or(std::collections::HashMap::is_empty)
}

fn build_navigation_selection(
    resolved_selection: &ResolvedWorkspaceSelection,
) -> WorkspaceNavigationSelection {
    WorkspaceNavigationSelection {
        selected_object_folder_path: resolved_selection.selected_object_folder_path.clone(),
        explorer_sub_path: resolved_selection.explorer_sub_path.clone(),
        current_path: build_current_path(
            resolved_selection.selected_object_folder_path.as_deref(),
            resolved_selection.explorer_sub_path.as_deref(),
        ),
        reconciliation_status: resolved_selection.reconciliation_status,
        reconciliation_reason: resolved_selection.reconciliation_reason,
        affected_paths: resolved_selection.affected_paths.clone(),
    }
}

pub async fn get_workspace_structure(
    pool: &sqlx::SqlitePool,
    input: WorkspaceStructureInput,
) -> Result<WorkspaceStructureViewModel, AppError> {
    get_workspace_structure_with_listing_mode(pool, input, false).await
}

/// A recovery pass already performs the authoritative deep disk traversal.
/// For its duration, return a shallow directory view so opening Mods is not
/// held hostage by classification and INI reads for every sibling.
pub async fn get_workspace_structure_with_listing_mode(
    pool: &sqlx::SqlitePool,
    input: WorkspaceStructureInput,
    shallow_listing: bool,
) -> Result<WorkspaceStructureViewModel, AppError> {
    let game_id = input.filter.game_id.clone();
    let mods_path = load_game_mods_path(pool, &game_id).await?;
    let objects = get_filtered_objects_with_conflict_check(pool, &input.filter)
        .await?
        .objects;

    if !Path::new(&mods_path).is_dir() {
        let resolved_selection = resolve_unavailable_workspace_selection(&input);
        let selection = build_navigation_selection(&resolved_selection);

        return Ok(WorkspaceStructureViewModel {
            objects: map_workspace_objects(WorkspaceObjectMapping {
                objects,
                root_folders: &[],
                mods_path: &mods_path,
                source_available: false,
                include_unregistered: false,
            }),
            explorer: empty_workspace_explorer(),
            selection,
            runtime: WorkspaceRuntime {
                game_id,
                source_state: unavailable_source_state(&mods_path),
                recovery_status:
                    crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus::Ready,
            },
        });
    }

    let resolved_selection = resolve_workspace_selection(&mods_path, &input);
    let listing_enrichment = load_listing_enrichment(pool, &game_id, &mods_path).await?;
    let root_listing = if shallow_listing {
        list_mod_folders_for_game_shallow_with_enrichment(
            &listing_enrichment,
            mods_path.clone(),
            None,
        )
        .await?
    } else {
        list_mod_folders_for_game_with_enrichment(&listing_enrichment, mods_path.clone(), None)
            .await?
    };
    let workspace_objects = map_workspace_objects(WorkspaceObjectMapping {
        objects,
        root_folders: &root_listing.children,
        mods_path: &mods_path,
        source_available: true,
        include_unregistered: include_unregistered_roots(&input.filter),
    });
    let raw_explorer = if resolved_selection.explorer_sub_path.is_none() {
        root_listing
    } else {
        let listing = if shallow_listing {
            list_mod_folders_for_game_shallow_with_enrichment(
                &listing_enrichment,
                mods_path.clone(),
                resolved_selection.explorer_sub_path.clone(),
            )
            .await?
        } else {
            list_mod_folders_for_game_with_enrichment(
                &listing_enrichment,
                mods_path.clone(),
                resolved_selection.explorer_sub_path.clone(),
            )
            .await?
        };
        listing
    };
    let explorer = map_workspace_explorer(raw_explorer);
    let selection = build_navigation_selection(&resolved_selection);

    Ok(WorkspaceStructureViewModel {
        objects: workspace_objects,
        explorer,
        selection,
        runtime: WorkspaceRuntime {
            game_id,
            source_state: available_source_state(),
            recovery_status:
                crate::modules::workspace::domain::workspace::WorkspaceRecoveryStatus::Ready,
        },
    })
}

pub async fn get_workspace_preview(
    pool: &sqlx::SqlitePool,
    input: WorkspacePreviewInput,
) -> Result<WorkspacePreviewResult, AppError> {
    let request_identity = WorkspacePreviewRequestIdentity {
        game_id: input.game_id.clone(),
        explorer_sub_path: input.explorer_sub_path.clone(),
        selected_mod_path: input.selected_mod_path.clone(),
    };
    let mods_path = load_game_mods_path(pool, &input.game_id).await?;
    let read = read_preview(pool, &input, &mods_path).await?;
    Ok(WorkspacePreviewResult {
        request_identity,
        context_status: read.context_status,
        preview: read.preview,
        selection: read.selection,
    })
}

#[cfg(test)]
pub async fn get_workspace_view_model(
    pool: &sqlx::SqlitePool,
    input: crate::modules::workspace::domain::workspace::WorkspaceViewModelInput,
) -> Result<crate::modules::workspace::domain::workspace::WorkspaceViewModel, AppError> {
    get_workspace_view_model_with_listing_mode(pool, input, false).await
}

#[cfg(test)]
pub async fn get_workspace_view_model_with_listing_mode(
    pool: &sqlx::SqlitePool,
    input: crate::modules::workspace::domain::workspace::WorkspaceViewModelInput,
    shallow_listing: bool,
) -> Result<crate::modules::workspace::domain::workspace::WorkspaceViewModel, AppError> {
    use crate::modules::workspace::domain::workspace::{WorkspaceSelection, WorkspaceViewModel};

    let structure_input = WorkspaceStructureInput {
        filter: input.filter.clone(),
        selected_object_folder_path: input.selected_object_folder_path.clone(),
        explorer_sub_path: input.explorer_sub_path.clone(),
    };
    let structure =
        get_workspace_structure_with_listing_mode(pool, structure_input, shallow_listing).await?;
    let preview = get_workspace_preview(
        pool,
        WorkspacePreviewInput {
            game_id: input.filter.game_id,
            explorer_sub_path: structure.selection.explorer_sub_path.clone(),
            selected_mod_path: input.selected_mod_path,
        },
    )
    .await?;
    let navigation = structure.selection;
    let preview_selection = preview.selection;
    let use_navigation_reconciliation = navigation.reconciliation_status
        != crate::modules::workspace::domain::workspace::WorkspaceSelectionReconciliationStatus::Unchanged;
    let mut affected_paths = navigation.affected_paths.clone();
    for path in &preview_selection.affected_paths {
        if !affected_paths.contains(path) {
            affected_paths.push(path.clone());
        }
    }
    let (reconciliation_status, reconciliation_reason) = if use_navigation_reconciliation {
        (
            navigation.reconciliation_status,
            navigation.reconciliation_reason,
        )
    } else {
        (
            preview_selection.reconciliation_status,
            preview_selection.reconciliation_reason,
        )
    };

    Ok(WorkspaceViewModel {
        objects: structure.objects,
        explorer: structure.explorer,
        preview: preview.preview,
        selection: WorkspaceSelection {
            selected_object_folder_path: navigation.selected_object_folder_path,
            explorer_sub_path: navigation.explorer_sub_path,
            selected_mod_path: preview_selection.selected_mod_path,
            current_path: navigation.current_path,
            reconciliation_status,
            reconciliation_reason,
            affected_paths,
        },
        runtime: structure.runtime,
    })
}

#[cfg(test)]
mod tests;

pub mod switch;
