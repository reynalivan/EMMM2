use crate::modules::catalog::application::match_engine::canonical_match::{
    match_canonical_objects_with_prepared_content, prepare_canonical_match_db,
};
use crate::modules::catalog::application::match_engine::inspection::{
    inspect_source_with_content, InspectionRequest,
};
use crate::modules::catalog::application::objects::classification::{
    CanonicalClassificationMatch, ObjectClassificationInput,
};
use crate::modules::ingestion::application::import_batch::types::{
    CanonicalSuggestion, SourceFingerprint, StableCategory,
};
use crate::shared::errors::AppError;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

struct PreparedClassification {
    input: ObjectClassificationInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalClassificationCatalogEntry {
    pub entry_key: String,
    pub name: String,
    pub category: StableCategory,
    pub metadata: serde_json::Value,
    pub thumbnail_path: Option<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ObjectClassificationDecision {
    #[serde(rename_all = "camelCase")]
    Canonical { entry_key: String },
    #[serde(rename_all = "camelCase")]
    Manual {
        category: StableCategory,
        sub_category: Option<String>,
        metadata: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewObjectClassificationBatchInput {
    pub game_id: String,
    pub object_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ObjectClassificationPreviewItem {
    pub object_id: String,
    pub object_name: String,
    pub source_path: String,
    pub current_category: String,
    pub canonical_suggestions: Vec<CanonicalSuggestion>,
    pub fingerprint: SourceFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyObjectClassificationItem {
    pub object_id: String,
    pub decision: ObjectClassificationDecision,
    pub fingerprint: SourceFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyObjectClassificationBatchInput {
    pub game_id: String,
    pub items: Vec<ApplyObjectClassificationItem>,
    #[serde(default)]
    pub disable_after_apply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyObjectClassificationBatchResult {
    pub applied: u32,
    pub child_mods_updated: u64,
    pub aliases_changed: bool,
    pub disabled_objects: u32,
    pub disable_warning: Option<String>,
}

pub fn list_canonical_classification_catalog(
    master_db: &crate::modules::matching::application::deep_matcher::MasterDb,
) -> Vec<CanonicalClassificationCatalogEntry> {
    let mut entries = master_db
        .entries
        .iter()
        .filter(|entry| {
            entry.entry_kind == crate::modules::matching::application::deep_matcher::EntryKind::Canonical
        })
        .filter_map(|entry| {
            let category = StableCategory::from_str(&entry.object_type).ok()?;
            let metadata = entry.metadata.clone().unwrap_or_else(|| serde_json::json!({}));
            if !metadata.is_object() {
                return None;
            }
            let mut aliases = entry.aliases.clone();
            for skin in &entry.custom_skins {
                aliases.push(skin.name.clone());
                aliases.extend(skin.aliases.clone());
            }
            aliases.sort_by_cached_key(|value| value.to_lowercase());
            aliases.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
            Some(CanonicalClassificationCatalogEntry {
                entry_key: crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key(
                    &entry.name,
                ),
                name: entry.name.clone(),
                category,
                metadata,
                thumbnail_path: entry.thumbnail_path.clone(),
                aliases,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by_cached_key(|entry| entry.name.to_lowercase());
    entries
}

pub async fn preview_object_classification_batch(
    db: &SqlitePool,
    input: &PreviewObjectClassificationBatchInput,
    master_db: &crate::modules::matching::application::deep_matcher::MasterDb,
    filters: &crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters,
    match_extensions: &[String],
) -> Result<Vec<ObjectClassificationPreviewItem>, AppError> {
    if input.object_ids.is_empty() {
        return Err(AppError::Validation(
            "Select at least one existing object to classify".to_string(),
        ));
    }
    let mods_root = crate::modules::games::adapters::sqlite::game::get_mod_path(db, &input.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))?;
    let canonical_db = prepare_canonical_match_db(master_db);
    let objects_by_id = crate::modules::catalog::adapters::sqlite::object::get_game_objects_by_ids(
        db,
        &input.game_id,
        &input.object_ids,
    )
    .await?
    .into_iter()
    .map(|object| (object.id.clone(), object))
    .collect::<HashMap<_, _>>();
    let mut result = Vec::with_capacity(input.object_ids.len());

    for object_id in &input.object_ids {
        let object = objects_by_id
            .get(object_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Object '{object_id}'")))?;
        let source = Path::new(&mods_root).join(&object.folder_path);
        let inspected = inspect_source_with_content(&InspectionRequest {
            source_path: source.clone(),
            planned_name: Some(object.name.clone()),
            match_extensions: match_extensions.to_vec(),
        })?;
        let canonical_suggestions = match_canonical_objects_with_prepared_content(
            &source,
            &object.name,
            &canonical_db,
            filters,
            &inspected.content,
        );
        result.push(ObjectClassificationPreviewItem {
            object_id: object.id,
            object_name: object.name,
            source_path: source.to_string_lossy().into_owned(),
            current_category: object.object_type,
            canonical_suggestions,
            fingerprint: inspected.inspection.fingerprint,
        });
    }
    Ok(result)
}

pub async fn apply_object_classification_batch(
    db: &SqlitePool,
    input: ApplyObjectClassificationBatchInput,
    master_db: &crate::modules::matching::application::deep_matcher::MasterDb,
    filters: &crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters,
    match_extensions: &[String],
) -> Result<ApplyObjectClassificationBatchResult, AppError> {
    if input.items.is_empty() {
        return Err(AppError::Validation(
            "Select at least one classification decision".to_string(),
        ));
    }
    let unique_ids = input
        .items
        .iter()
        .map(|item| item.object_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if unique_ids.len() != input.items.len() {
        return Err(AppError::Validation(
            "Classification object IDs must be unique".to_string(),
        ));
    }
    let mods_root = crate::modules::games::adapters::sqlite::game::get_mod_path(db, &input.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))?;
    let canonical_db = prepare_canonical_match_db(master_db);
    let object_ids = input
        .items
        .iter()
        .map(|item| item.object_id.clone())
        .collect::<Vec<_>>();
    let objects_by_id = crate::modules::catalog::adapters::sqlite::object::get_game_objects_by_ids(
        db,
        &input.game_id,
        &object_ids,
    )
    .await?
    .into_iter()
    .map(|object| (object.id.clone(), object))
    .collect::<HashMap<_, _>>();
    let mut prepared = Vec::with_capacity(input.items.len());

    for item in input.items {
        let object = objects_by_id
            .get(&item.object_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Object '{}'", item.object_id)))?;
        let source = Path::new(&mods_root).join(&object.folder_path);
        let inspected = inspect_source_with_content(&InspectionRequest {
            source_path: source.clone(),
            planned_name: Some(object.name.clone()),
            match_extensions: match_extensions.to_vec(),
        })?;
        if inspected.inspection.fingerprint != item.fingerprint {
            return Err(AppError::Validation(format!(
                "stale_preview: object '{}' changed after classification preview",
                object.name
            )));
        }
        let classification = match item.decision {
            ObjectClassificationDecision::Canonical { entry_key } => {
                canonical_classification_input(
                    master_db,
                    &canonical_db,
                    filters,
                    &source,
                    &object.name,
                    &inspected.content,
                    input.game_id.clone(),
                    item.object_id,
                    entry_key,
                )?
            }
            ObjectClassificationDecision::Manual {
                category,
                sub_category,
                metadata,
            } => ObjectClassificationInput {
                game_id: input.game_id.clone(),
                object_id: item.object_id,
                category: category.as_str().to_string(),
                subcategory: sub_category,
                metadata,
                canonical_match: None,
                confirmed_source_alias: None,
            },
        };
        prepared.push(PreparedClassification {
            input: classification,
        });
    }

    let mut tx = db.begin().await?;
    let mut applied = 0_u32;
    let mut child_mods_updated = 0_u64;
    let mut aliases_changed = false;
    for prepared in prepared {
        let result =
            super::classification::apply_object_classification_tx(&mut tx, prepared.input).await?;
        applied += 1;
        child_mods_updated += result.child_mods_updated;
        aliases_changed |= result.aliases_changed;
    }
    tx.commit().await?;

    Ok(ApplyObjectClassificationBatchResult {
        applied,
        child_mods_updated,
        aliases_changed,
        disabled_objects: 0,
        disable_warning: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn canonical_classification_input(
    master_db: &crate::modules::matching::application::deep_matcher::MasterDb,
    canonical_db: &crate::modules::matching::application::deep_matcher::MasterDb,
    filters: &crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters,
    source: &Path,
    object_name: &str,
    content: &crate::modules::workspace::application::scanner::core::walker::FolderContent,
    game_id: String,
    object_id: String,
    entry_key: String,
) -> Result<ObjectClassificationInput, AppError> {
    let entry = master_db
        .entries
        .iter()
        .find(|entry| {
            crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key(
                &entry.name,
            ) == entry_key
        })
        .ok_or_else(|| {
            AppError::Validation(format!(
                "Canonical entry '{entry_key}' does not exist in the active game database"
            ))
        })?;
    if entry.entry_kind != crate::modules::matching::application::deep_matcher::EntryKind::Canonical
    {
        return Err(AppError::Validation(format!(
            "Canonical entry '{entry_key}' is taxonomy-only"
        )));
    }
    let category = StableCategory::from_str(&entry.object_type).map_err(|_| {
        AppError::Validation(format!(
            "Canonical entry '{entry_key}' has unsupported category '{}'",
            entry.object_type
        ))
    })?;
    let metadata = entry
        .metadata
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    if !metadata.is_object() {
        return Err(AppError::Validation(format!(
            "Canonical entry '{entry_key}' has invalid metadata"
        )));
    }
    let matched = match_canonical_objects_with_prepared_content(
        source,
        object_name,
        canonical_db,
        filters,
        content,
    )
    .into_iter()
    .find(|candidate| candidate.entry_key == entry_key);
    let canonical_match = CanonicalClassificationMatch {
        entry_key,
        alias_name: matched
            .as_ref()
            .and_then(|candidate| candidate.matched_alias.clone()),
        confidence: matched
            .as_ref()
            .map(|candidate| f64::from(candidate.confidence_percentage) / 100.0),
        reason: Some(if matched.is_some() {
            "Confirmed canonical matcher recommendation".to_string()
        } else {
            "Confirmed canonical catalog selection".to_string()
        }),
        source: if matched.is_some() {
            "classification_wizard_matcher".to_string()
        } else {
            "classification_wizard_manual".to_string()
        },
    };
    Ok(ObjectClassificationInput {
        game_id,
        object_id,
        category: category.as_str().to_string(),
        subcategory: None,
        metadata,
        canonical_match: Some(canonical_match),
        confirmed_source_alias: Some(object_name.to_string()),
    })
}
