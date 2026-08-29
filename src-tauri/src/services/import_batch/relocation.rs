use super::types::{DestinationSuggestion, StableCategory};
use crate::domain::errors::AppError;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRelocationBatchInput {
    pub game_id: String,
    pub source_paths: Vec<String>,
    pub current_object_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RelocationPreviewItem {
    pub source_path: String,
    pub source_name: String,
    pub category: StableCategory,
    pub suggestions: Vec<DestinationSuggestion>,
}

pub async fn preview_relocation_batch(
    db: &SqlitePool,
    input: PreviewRelocationBatchInput,
) -> Result<Vec<RelocationPreviewItem>, AppError> {
    if input.source_paths.is_empty() {
        return Err(AppError::Validation(
            "Select at least one folder to relocate".to_string(),
        ));
    }
    let mods_root = crate::repo::game_repo::get_mod_path(db, &input.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))?;
    let page = crate::repo::object_repo::get_filtered_objects(
        db,
        &crate::domain::objects::ObjectFilter {
            game_id: input.game_id.clone(),
            search_query: None,
            object_type: None,
            meta_filters: None,
            sort_by: None,
            status_filter: None,
        },
    )
    .await?;
    let category = input
        .current_object_id
        .as_ref()
        .and_then(|id| page.objects.iter().find(|object| &object.id == id))
        .and_then(|object| StableCategory::from_str(&object.object_type).ok())
        .unwrap_or(StableCategory::Other);
    let existing = page
        .objects
        .into_iter()
        .filter(|object| input.current_object_id.as_ref() != Some(&object.id))
        .map(|object| {
            let object_category =
                StableCategory::from_str(&object.object_type).unwrap_or(StableCategory::Other);
            let folder_name = Path::new(&object.folder_path)
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| object.name.clone());
            crate::services::match_engine::destination::ExistingDestination {
                object_id: object.id,
                name: object.name,
                folder_name,
                canonical_entry_key: object.matched_entry_key,
                aliases: object.matched_alias_name.into_iter().collect(),
                category: object_category,
            }
        })
        .collect::<Vec<_>>();
    Ok(input
        .source_paths
        .into_iter()
        .map(|source_path| {
            let source_name = Path::new(&source_path)
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| source_path.clone());
            let suggestions =
                crate::services::match_engine::destination::resolve_destination_candidates(
                    crate::services::match_engine::destination::DestinationContext {
                        source_name: &source_name,
                        category,
                        specific_target: None,
                        existing: &existing,
                        canonical: None,
                        mods_root: &mods_root,
                        enforce_category: false,
                    },
                );
            RelocationPreviewItem {
                source_path,
                source_name,
                category,
                suggestions,
            }
        })
        .collect())
}
