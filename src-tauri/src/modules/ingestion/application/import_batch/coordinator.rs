use super::types::{
    ConfidenceTier, CreateImportBatchInput, ImportBatch, ImportBatchStatus, ImportDecision,
    ImportFlow, ImportItem, ImportSourceKind, RenameImportItemInput,
    SetImportItemClassificationInput, SetImportItemDecisionInput, StableCategory, TargetMode,
};
use crate::modules::ingestion::adapters::sqlite::import_batch::{
    self, CreateImportBatchRecord, NewImportItemRecord,
};
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportLibraryReadiness {
    pub batch_id: String,
    pub items: Vec<
        crate::modules::catalog::application::objects::classification_batch::ObjectClassificationPreviewItem,
    >,
    pub high_count: u32,
    pub medium_count: u32,
    pub review_started: bool,
}

pub async fn create_import_batch(
    db: &SqlitePool,
    input: CreateImportBatchInput,
) -> Result<ImportBatch, AppError> {
    let (batch, items) = prepare_import_batch(db, input).await?;
    let batch_id = batch.id.clone();
    import_batch::create_batch(db, &batch, &items).await?;
    reload_created_batch(db, &batch_id).await
}

pub async fn create_mod_inbox_import_batch(
    db: &SqlitePool,
    input: CreateImportBatchInput,
) -> Result<ImportBatch, AppError> {
    let (batch, items) = prepare_import_batch(db, input).await?;
    let batch_id = batch.id.clone();
    if let Some(conflict) =
        import_batch::create_mod_inbox_batch_if_sources_available(db, &batch, &items).await?
    {
        return Err(AppError::Validation(format!(
            "Mod Inbox source '{}' already belongs to active batch '{}'",
            conflict.source_path, conflict.batch_id
        )));
    }
    reload_created_batch(db, &batch_id).await
}

async fn prepare_import_batch(
    db: &SqlitePool,
    input: CreateImportBatchInput,
) -> Result<(CreateImportBatchRecord, Vec<NewImportItemRecord>), AppError> {
    validate_create_input(db, &input).await?;
    let workspace_roots = configured_workspace_roots(db).await?;
    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut items = Vec::with_capacity(input.sources.len());
    for source in &input.sources {
        let canonical = Path::new(&source.path).canonicalize().map_err(|error| {
            AppError::Validation(format!(
                "Import source '{}' is unavailable: {error}",
                source.path
            ))
        })?;
        reject_workspace_source(&canonical, &workspace_roots)?;
        let source_kind = source
            .source_kind
            .unwrap_or_else(|| infer_source_kind(&canonical, input.flow));
        validate_source_kind(&canonical, source_kind)?;
        let planned_name = planned_name_for_source(&canonical, source_kind)?;
        items.push(NewImportItemRecord {
            id: uuid::Uuid::new_v4().to_string(),
            source_kind,
            source_path: canonical.to_string_lossy().into_owned(),
            staging_path: None,
            planned_name,
        });
    }
    Ok((
        CreateImportBatchRecord {
            id: batch_id,
            game_id: input.game_id,
            flow: input.flow,
            target_mode: input.target_mode,
            target_object_id: input.target_object_id,
            target_subpath: normalized_optional(input.target_subpath),
            source_archive_path: None,
        },
        items,
    ))
}

async fn reload_created_batch(db: &SqlitePool, batch_id: &str) -> Result<ImportBatch, AppError> {
    import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::Internal("Created import batch could not be reloaded".to_string()))
}

pub async fn set_import_item_classification(
    db: &SqlitePool,
    mut input: SetImportItemClassificationInput,
) -> Result<ImportItem, AppError> {
    if !input.metadata.is_object() {
        return Err(AppError::Validation(
            "classification metadata must be a JSON object".to_string(),
        ));
    }
    input.sub_category = normalized_optional(input.sub_category);
    if !import_batch::store_classification(db, &input).await? {
        return Err(AppError::Validation(format!(
            "Import item '{}' must be in awaiting_category state",
            input.item_id
        )));
    }
    require_item(db, &input.item_id).await
}

pub async fn rename_import_item_plan(
    db: &SqlitePool,
    mut input: RenameImportItemInput,
) -> Result<ImportItem, AppError> {
    input.planned_name = crate::modules::library::application::mods::core_ops::standardize_prefix(
        input.planned_name.trim(),
        false,
    );
    crate::modules::library::application::mods::core_ops::validate_folder_name_component(
        &input.planned_name,
    )?;
    if !import_batch::rename_planned_item(db, &input.item_id, &input.planned_name).await? {
        return Err(AppError::Validation(format!(
            "Import item '{}' can no longer be renamed",
            input.item_id
        )));
    }
    require_item(db, &input.item_id).await
}

pub async fn set_import_item_decision(
    db: &SqlitePool,
    mut input: SetImportItemDecisionInput,
) -> Result<ImportItem, AppError> {
    if input.decision == ImportDecision::Pending {
        return Err(AppError::Validation(
            "A pending import decision cannot be committed".to_string(),
        ));
    }
    let item = require_item(db, &input.item_id).await?;
    if input.decision == ImportDecision::Skip {
        input.destination_object_id = None;
        input.destination_path = None;
        input.canonical_entry_key = None;
        input.matched_alias = None;
    } else if matches!(
        input.decision,
        ImportDecision::Reallocate | ImportDecision::KeepSpecificTarget
    ) {
        let object_id = input.destination_object_id.as_deref().ok_or_else(|| {
            AppError::Validation("Manual target decisions require an existing object".to_string())
        })?;
        let batch = import_batch::get_batch(db, &item.batch_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", item.batch_id)))?;
        let object =
            crate::modules::catalog::adapters::sqlite::object::get_game_object_by_id(db, object_id)
                .await?
                .filter(|object| object.game_id == batch.game_id)
                .ok_or_else(|| {
                    AppError::Validation(
                        "Selected manual target is not part of the import batch game".to_string(),
                    )
                })?;
        if input.decision == ImportDecision::KeepSpecificTarget
            && batch.target_object_id.as_deref() != Some(object_id)
        {
            return Err(AppError::Validation(
                "Keep-specific-target can only select the batch's original target".to_string(),
            ));
        }
        let mods_root =
            crate::modules::games::adapters::sqlite::game::get_mod_path(db, &batch.game_id)
                .await?
                .ok_or_else(|| {
                    AppError::Validation("Game has no configured mods path".to_string())
                })?;
        input.destination_path = Some(
            Path::new(&mods_root)
                .join(&object.folder_path)
                .to_string_lossy()
                .into_owned(),
        );
        input.canonical_entry_key = None;
        input.matched_alias = None;
    } else {
        let selected = item.destination_suggestions.iter().find(|suggestion| {
            input
                .destination_object_id
                .as_ref()
                .is_some_and(|id| suggestion.object_id.as_ref() == Some(id))
                || input
                    .canonical_entry_key
                    .as_ref()
                    .is_some_and(|key| suggestion.canonical_entry_key.as_ref() == Some(key))
        });
        let Some(selected) = selected else {
            return Err(AppError::Validation(
                "Selected destination is not part of the latest backend suggestions".to_string(),
            ));
        };
        input.destination_object_id = selected.object_id.clone();
        input.destination_path = Some(selected.target_path.clone());
        input.canonical_entry_key = selected.canonical_entry_key.clone();
    }
    let (confidence, tier) = if input.decision == ImportDecision::Skip {
        (0, super::types::ConfidenceTier::NoMatch)
    } else {
        item.destination_suggestions
            .iter()
            .find(|suggestion| {
                input
                    .destination_object_id
                    .as_ref()
                    .is_some_and(|id| suggestion.object_id.as_ref() == Some(id))
                    || input
                        .destination_path
                        .as_ref()
                        .is_some_and(|path| suggestion.target_path == *path)
                    || input
                        .canonical_entry_key
                        .as_ref()
                        .is_some_and(|key| suggestion.canonical_entry_key.as_ref() == Some(key))
            })
            .map(|suggestion| {
                (suggestion.confidence_percentage, suggestion.confidence_tier.clone())
            })
            .unwrap_or((0, super::types::ConfidenceTier::NoMatch))
    };
    if !import_batch::store_decision(db, &input, confidence, tier).await? {
        return Err(AppError::Validation(format!(
            "Import item '{}' is not awaiting a destination decision",
            input.item_id
        )));
    }
    require_item(db, &input.item_id).await
}

pub async fn refresh_import_item_suggestions(
    db: &SqlitePool,
    item_id: &str,
    master_db: &crate::modules::matching::application::deep_matcher::MasterDb,
    ini_filters: &crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters,
) -> Result<ImportItem, AppError> {
    let item = require_item(db, item_id).await?;
    if !item.status.can_refresh_object_suggestions() {
        return Err(AppError::Validation(format!(
            "Import item '{item_id}' must have a category decision before object matching"
        )));
    }
    let category = item.match_category.ok_or_else(|| {
        AppError::Validation(format!(
            "Import item '{item_id}' has no confirmed stable category"
        ))
    })?;
    let inspection = import_batch::get_stored_inspection(db, item_id)
        .await?
        .ok_or_else(|| {
            AppError::Validation(format!("Import item '{item_id}' has no source inspection"))
        })?;
    let mut canonical = crate::modules::catalog::application::match_engine::canonical_match::match_canonical_objects(
        Path::new(&inspection.source_path),
        &item.planned_name,
        category,
        master_db,
        ini_filters,
    );
    rerank_with_metadata(&mut canonical, &item.classification_metadata, master_db);

    let batch = import_batch::get_batch(db, &item.batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{}'", item.batch_id)))?;
    let mods_root = crate::modules::games::adapters::sqlite::game::get_mod_path(db, &batch.game_id)
        .await?
        .ok_or_else(|| AppError::Validation("Game has no configured mods path".to_string()))?;
    let page = crate::modules::catalog::adapters::sqlite::object::get_filtered_objects(
        db,
        &crate::modules::catalog::domain::objects::ObjectFilter {
            game_id: batch.game_id.clone(),
            search_query: None,
            object_type: None,
            meta_filters: None,
            sort_by: None,
            status_filter: None,
        },
    )
    .await?;
    let existing = page
        .objects
        .into_iter()
        .map(|object| {
            let category =
                StableCategory::from_str(&object.object_type).unwrap_or(StableCategory::Other);
            let folder_name = Path::new(&object.folder_path)
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| object.name.clone());
            let aliases = object.matched_alias_name.into_iter().collect();
            crate::modules::catalog::application::match_engine::destination::ExistingDestination {
                object_id: object.id,
                name: object.name,
                folder_name,
                canonical_entry_key: object.matched_entry_key,
                aliases,
                category,
            }
        })
        .collect::<Vec<_>>();
    let specific_target = batch
        .target_object_id
        .as_ref()
        .and_then(|id| existing.iter().find(|target| &target.object_id == id));
    let canonical_identity = canonical.first().and_then(|suggestion| {
        master_db.entries.iter().find_map(|entry| {
            let key =
                crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key(
                    &entry.name,
                );
            (key == suggestion.entry_key).then(|| {
                crate::modules::catalog::application::match_engine::types::CanonicalIdentity {
                    entry_key: key,
                    name: entry.name.clone(),
                    entry_kind: entry.entry_kind,
                }
            })
        })
    });
    let destinations = crate::modules::catalog::application::match_engine::destination::resolve_all_destination_candidates(
        crate::modules::catalog::application::match_engine::destination::DestinationContext {
            source_name: &item.planned_name,
            category,
            specific_target,
            existing: &existing,
            canonical: canonical_identity.as_ref(),
            mods_root: &mods_root,
            // `Other` means the identity category is not clarified yet. Keep
            // exact folder/alias evidence available across existing objects.
            enforce_category: category != StableCategory::Other,
        },
    );
    let evidence = canonical
        .first()
        .map(|suggestion| suggestion.evidence.clone())
        .unwrap_or_else(|| inspection.evidence.clone());
    if !import_batch::store_match_suggestions(db, item_id, &canonical, &destinations, &evidence)
        .await?
    {
        return Err(AppError::Validation(format!(
            "Import item '{item_id}' changed while suggestions were refreshed"
        )));
    }
    require_item(db, item_id).await
}

pub async fn preview_import_library_readiness(
    db: &SqlitePool,
    batch_id: &str,
    master_db: &crate::modules::matching::application::deep_matcher::MasterDb,
    ini_filters: &crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters,
    match_extensions: &[String],
) -> Result<ImportLibraryReadiness, AppError> {
    let batch = import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    let review_started = import_batch::has_batch_review_started(db, batch_id).await?;
    if review_started {
        return Ok(ImportLibraryReadiness {
            batch_id: batch_id.to_string(),
            items: Vec::new(),
            high_count: 0,
            medium_count: 0,
            review_started: true,
        });
    }
    if matches!(batch.status, ImportBatchStatus::Draft | ImportBatchStatus::Analyzing) {
        return Err(AppError::Validation(
            "Import source analysis must finish before checking Object Library readiness"
                .to_string(),
        ));
    }

    let relevant_ids = batch
        .items
        .iter()
        .flat_map(|item| item.destination_suggestions.iter())
        .filter(|suggestion| {
            matches!(
                suggestion.confidence_tier,
                ConfidenceTier::High | ConfidenceTier::Medium
            )
        })
        .filter_map(|suggestion| suggestion.object_id.clone())
        .collect::<BTreeSet<_>>();
    if relevant_ids.is_empty() {
        return Ok(ImportLibraryReadiness {
            batch_id: batch_id.to_string(),
            items: Vec::new(),
            high_count: 0,
            medium_count: 0,
            review_started: false,
        });
    }

    let page = crate::modules::catalog::adapters::sqlite::object::get_filtered_objects(
        db,
        &crate::modules::catalog::domain::objects::ObjectFilter {
            game_id: batch.game_id.clone(),
            search_query: None,
            object_type: None,
            meta_filters: None,
            sort_by: None,
            status_filter: None,
        },
    )
    .await?;
    let object_ids = page
        .objects
        .into_iter()
        .filter(|object| relevant_ids.contains(&object.id))
        .filter(|object| object.matched_entry_key.is_none())
        .filter(|object| object.matched_source.as_deref() != Some("classification_wizard_custom"))
        .map(|object| object.id)
        .collect::<Vec<_>>();
    if object_ids.is_empty() {
        return Ok(ImportLibraryReadiness {
            batch_id: batch_id.to_string(),
            items: Vec::new(),
            high_count: 0,
            medium_count: 0,
            review_started: false,
        });
    }

    use crate::modules::catalog::application::objects::classification_batch::{
        ObjectClassificationDraft, PreviewObjectClassificationBatchInput,
    };
    let category_preview =
        crate::modules::catalog::application::objects::classification_batch::preview_object_classification_batch(
            db,
            &PreviewObjectClassificationBatchInput {
                game_id: batch.game_id.clone(),
                object_ids: object_ids.clone(),
                drafts: Vec::new(),
            },
            master_db,
            ini_filters,
            match_extensions,
        )
        .await?;
    let drafts = category_preview
        .iter()
        .map(|item| {
            let suggestion = item.category_suggestions.first();
            ObjectClassificationDraft {
                object_id: item.object_id.clone(),
                category: suggestion
                    .map(|value| value.category)
                    .unwrap_or(StableCategory::Other),
                sub_category: suggestion.and_then(|value| value.sub_category.clone()),
                metadata: suggestion
                    .map(|value| value.metadata.clone())
                    .unwrap_or_else(|| serde_json::json!({})),
            }
        })
        .collect();
    let mut items =
        crate::modules::catalog::application::objects::classification_batch::preview_object_classification_batch(
            db,
            &PreviewObjectClassificationBatchInput {
                game_id: batch.game_id,
                object_ids,
                drafts,
            },
            master_db,
            ini_filters,
            match_extensions,
        )
        .await?;
    items.retain(|item| {
        item.canonical_suggestions.first().is_some_and(|suggestion| {
            suggestion.confidence_tier == ConfidenceTier::High
                || suggestion.confidence_tier == ConfidenceTier::Medium
        })
    });
    items.sort_by(|left, right| {
        right
            .canonical_suggestions
            .first()
            .map(|suggestion| suggestion.confidence_percentage)
            .unwrap_or(0)
            .cmp(
                &left
                    .canonical_suggestions
                    .first()
                    .map(|suggestion| suggestion.confidence_percentage)
                    .unwrap_or(0),
            )
            .then_with(|| left.object_name.cmp(&right.object_name))
    });
    let high_count = items
        .iter()
        .filter(|item| {
            item.canonical_suggestions
                .first()
                .is_some_and(|suggestion| suggestion.confidence_tier == ConfidenceTier::High)
        })
        .count() as u32;
    let medium_count = items.len() as u32 - high_count;
    Ok(ImportLibraryReadiness {
        batch_id: batch_id.to_string(),
        items,
        high_count,
        medium_count,
        review_started: false,
    })
}

pub async fn refresh_import_batch_matches(
    db: &SqlitePool,
    batch_id: &str,
    master_db: &crate::modules::matching::application::deep_matcher::MasterDb,
    ini_filters: &crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters,
) -> Result<ImportBatch, AppError> {
    if import_batch::has_batch_review_started(db, batch_id).await? {
        return Err(AppError::Validation(
            "Import matches can only be refreshed before review starts".to_string(),
        ));
    }
    let batch = import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    let item_ids = batch
        .items
        .iter()
        .filter(|item| item.status.can_refresh_object_suggestions())
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    for item_id in item_ids {
        refresh_import_item_suggestions(db, &item_id, master_db, ini_filters).await?;
    }
    import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::Internal("Refreshed import batch could not be reloaded".to_string()))
}

async fn configured_workspace_roots(db: &SqlitePool) -> Result<Vec<std::path::PathBuf>, AppError> {
    let games = crate::modules::games::adapters::sqlite::game::get_all_games(db).await?;
    Ok(games
        .into_iter()
        .filter_map(|game| {
            let configured = game
                .mods_path
                .filter(|path| !path.trim().is_empty())
                .unwrap_or(game.path);
            Path::new(&configured).canonicalize().ok()
        })
        .collect())
}

fn reject_workspace_source(
    source: &Path,
    workspace_roots: &[std::path::PathBuf],
) -> Result<(), AppError> {
    if workspace_roots
        .iter()
        .any(|root| source.starts_with(root) || root.starts_with(source))
    {
        return Err(AppError::Security(format!(
            "Import source '{}' overlaps a configured game workspace; use relocation instead",
            source.display()
        )));
    }
    Ok(())
}

fn rerank_with_metadata(
    suggestions: &mut [crate::modules::ingestion::application::import_batch::types::CanonicalSuggestion],
    user_metadata: &serde_json::Value,
    master_db: &crate::modules::matching::application::deep_matcher::MasterDb,
) {
    let Some(user_fields) = user_metadata.as_object() else {
        return;
    };
    if user_fields.is_empty() {
        return;
    }
    for suggestion in suggestions.iter_mut() {
        let Some(entry) = master_db.entries.iter().find(|entry| {
            crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key(
                &entry.name,
            ) == suggestion.entry_key
        }) else {
            continue;
        };
        let Some(db_fields) = entry
            .metadata
            .as_ref()
            .and_then(serde_json::Value::as_object)
        else {
            continue;
        };
        let delta = user_fields.iter().fold(0_i16, |delta, (key, user_value)| {
            let Some(db_value) = db_fields.get(key) else {
                return delta;
            };
            if db_value == user_value {
                delta + 5
            } else {
                delta - 5
            }
        });
        suggestion.confidence_percentage =
            (i16::from(suggestion.confidence_percentage) + delta).clamp(0, 100) as u8;
        suggestion.confidence_tier =
            crate::modules::ingestion::application::import_batch::types::ConfidenceTier::from_percentage(
                suggestion.confidence_percentage,
            );
    }
    suggestions.sort_by(|left, right| {
        right
            .confidence_percentage
            .cmp(&left.confidence_percentage)
            .then_with(|| left.name.cmp(&right.name))
    });
}

async fn validate_create_input(
    db: &SqlitePool,
    input: &CreateImportBatchInput,
) -> Result<(), AppError> {
    if input.sources.is_empty() {
        return Err(AppError::Validation(
            "At least one import source is required".to_string(),
        ));
    }
    if input.sources.len() > 500 {
        return Err(AppError::Validation(
            "One import batch can contain at most 500 sources".to_string(),
        ));
    }
    if crate::modules::games::adapters::sqlite::game::get_mod_path(db, &input.game_id)
        .await?
        .is_none()
    {
        return Err(AppError::NotFound(format!(
            "Game '{}' is not configured",
            input.game_id
        )));
    }
    match (
        input.flow,
        input.target_mode,
        input.target_object_id.as_ref(),
    ) {
        (ImportFlow::SpecificImport, TargetMode::Specific, Some(_)) => Ok(()),
        (ImportFlow::SpecificImport, _, _) => Err(AppError::Validation(
            "Specific import requires target_mode specific and a target object".to_string(),
        )),
        (_, TargetMode::Specific, None) => Err(AppError::Validation(
            "Specific target mode requires a target object".to_string(),
        )),
        _ => Ok(()),
    }
}

fn infer_source_kind(path: &Path, flow: ImportFlow) -> ImportSourceKind {
    if flow == ImportFlow::ReadyToMove {
        return ImportSourceKind::ReadyToMove;
    }
    if flow == ImportFlow::Browser {
        return ImportSourceKind::BrowserDownload;
    }
    if path.is_file() {
        ImportSourceKind::ArchiveRoot
    } else {
        ImportSourceKind::Folder
    }
}

fn validate_source_kind(path: &Path, kind: ImportSourceKind) -> Result<(), AppError> {
    match kind {
        ImportSourceKind::Folder if path.is_dir() => Ok(()),
        ImportSourceKind::ReadyToMove if path.is_dir() => Ok(()),
        ImportSourceKind::ReadyToMove if path.is_file() => {
            if crate::modules::library::application::mods::archive::ArchiveFormat::detect(path)
                .is_some()
            {
                Ok(())
            } else {
                Err(AppError::Validation(format!(
                    "Unsupported ReadyToMove archive: {}",
                    path.display()
                )))
            }
        }
        ImportSourceKind::ArchiveRoot | ImportSourceKind::BrowserDownload if path.is_file() => {
            if crate::modules::library::application::mods::archive::ArchiveFormat::detect(path)
                .is_some()
            {
                Ok(())
            } else {
                Err(AppError::Validation(format!(
                    "Unsupported archive source: {}",
                    path.display()
                )))
            }
        }
        _ => Err(AppError::Validation(format!(
            "Import source kind does not match path: {}",
            path.display()
        ))),
    }
}

fn planned_name_for_source(path: &Path, kind: ImportSourceKind) -> Result<String, AppError> {
    let name = if path.is_file()
        && matches!(
            kind,
            ImportSourceKind::ArchiveRoot
                | ImportSourceKind::BrowserDownload
                | ImportSourceKind::ReadyToMove
        ) {
        path.file_stem()
    } else {
        path.file_name()
    }
    .map(|value| value.to_string_lossy().into_owned())
    .unwrap_or_default();
    crate::modules::library::application::mods::core_ops::validate_folder_name_component(&name)?;
    Ok(crate::modules::library::application::mods::core_ops::standardize_prefix(&name, false))
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}

async fn require_item(db: &SqlitePool, item_id: &str) -> Result<ImportItem, AppError> {
    import_batch::get_item(db, item_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import item '{item_id}'")))
}
