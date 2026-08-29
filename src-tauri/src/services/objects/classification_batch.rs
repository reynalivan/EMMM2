use crate::domain::errors::AppError;
use crate::services::import_batch::types::{
    CanonicalSuggestion, CategorySuggestion, SourceFingerprint, StableCategory,
};
use crate::services::match_engine::inspection::{inspect_source, InspectionRequest};
use crate::services::objects::classification::{
    CanonicalClassificationMatch, ObjectClassificationInput,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::path::Path;

struct PreparedClassification {
    input: ObjectClassificationInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ObjectClassificationDraft {
    pub object_id: String,
    pub category: StableCategory,
    pub sub_category: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewObjectClassificationBatchInput {
    pub game_id: String,
    pub object_ids: Vec<String>,
    pub drafts: Vec<ObjectClassificationDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ObjectClassificationPreviewItem {
    pub object_id: String,
    pub object_name: String,
    pub source_path: String,
    pub current_category: String,
    pub category_suggestions: Vec<CategorySuggestion>,
    pub canonical_suggestions: Vec<CanonicalSuggestion>,
    pub fingerprint: SourceFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyObjectClassificationItem {
    pub object_id: String,
    pub category: StableCategory,
    pub sub_category: Option<String>,
    pub metadata: serde_json::Value,
    pub canonical_entry_key: Option<String>,
    pub canonical_alias: Option<String>,
    pub confidence_percentage: Option<u8>,
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

pub async fn preview_object_classification_batch(
    db: &SqlitePool,
    input: &PreviewObjectClassificationBatchInput,
    master_db: &crate::services::scanner::deep_matcher::MasterDb,
    filters: &crate::services::scanner::deep_matcher::analysis::content::PreparedTokenFilters,
    match_extensions: &[String],
) -> Result<Vec<ObjectClassificationPreviewItem>, AppError> {
    if input.object_ids.is_empty() {
        return Err(AppError::Validation(
            "Select at least one existing object to classify".to_string(),
        ));
    }
    let drafts = input
        .drafts
        .iter()
        .map(|draft| (draft.object_id.as_str(), draft))
        .collect::<BTreeMap<_, _>>();
    let mods_root = crate::repo::game::get_mod_path(db, &input.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))?;
    let mut result = Vec::with_capacity(input.object_ids.len());

    for object_id in &input.object_ids {
        let object = crate::repo::object::get_game_object_by_id(db, object_id)
            .await?
            .filter(|object| object.game_id == input.game_id)
            .ok_or_else(|| AppError::NotFound(format!("Object '{object_id}'")))?;
        let source = Path::new(&mods_root).join(&object.folder_path);
        let inspection = inspect_source(&InspectionRequest {
            source_path: source.clone(),
            planned_name: Some(object.name.clone()),
            match_extensions: match_extensions.to_vec(),
        })?;
        let categories = crate::services::match_engine::classification::classify_source(
            &source,
            &object.name,
            master_db,
            filters,
        );
        let canonical_suggestions = if let Some(draft) = drafts.get(object_id.as_str()) {
            if !draft.metadata.is_object() {
                return Err(AppError::Validation(format!(
                    "Classification metadata for object '{object_id}' must be an object"
                )));
            }
            crate::services::match_engine::canonical_match::match_canonical_objects(
                &source,
                &object.name,
                draft.category,
                master_db,
                filters,
            )
        } else {
            Vec::new()
        };
        result.push(ObjectClassificationPreviewItem {
            object_id: object.id,
            object_name: object.name,
            source_path: source.to_string_lossy().into_owned(),
            current_category: object.object_type,
            category_suggestions: categories,
            canonical_suggestions,
            fingerprint: inspection.fingerprint,
        });
    }
    Ok(result)
}

pub async fn apply_object_classification_batch(
    db: &SqlitePool,
    input: ApplyObjectClassificationBatchInput,
    master_db: &crate::services::scanner::deep_matcher::MasterDb,
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
    let mods_root = crate::repo::game::get_mod_path(db, &input.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", input.game_id)))?;
    let mut prepared = Vec::with_capacity(input.items.len());

    for item in input.items {
        let object = crate::repo::object::get_game_object_by_id(db, &item.object_id)
            .await?
            .filter(|object| object.game_id == input.game_id)
            .ok_or_else(|| AppError::NotFound(format!("Object '{}'", item.object_id)))?;
        let source = Path::new(&mods_root).join(&object.folder_path);
        let current = inspect_source(&InspectionRequest {
            source_path: source,
            planned_name: Some(object.name.clone()),
            match_extensions: match_extensions.to_vec(),
        })?;
        if current.fingerprint != item.fingerprint {
            return Err(AppError::Validation(format!(
                "stale_preview: object '{}' changed after classification preview",
                object.name
            )));
        }
        let canonical_match = validate_canonical_selection(
            master_db,
            item.category,
            item.canonical_entry_key,
            item.canonical_alias,
            item.confidence_percentage,
        )?;
        prepared.push(PreparedClassification {
            input: ObjectClassificationInput {
                game_id: input.game_id.clone(),
                object_id: item.object_id,
                category: item.category.as_str().to_string(),
                subcategory: item.sub_category,
                metadata: item.metadata,
                confirmed_source_alias: canonical_match.as_ref().map(|_| object.name.clone()),
                canonical_match,
            },
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

fn validate_canonical_selection(
    master_db: &crate::services::scanner::deep_matcher::MasterDb,
    category: StableCategory,
    entry_key: Option<String>,
    alias_name: Option<String>,
    confidence_percentage: Option<u8>,
) -> Result<Option<CanonicalClassificationMatch>, AppError> {
    let Some(entry_key) = entry_key else {
        if alias_name.is_some() || confidence_percentage.is_some() {
            return Err(AppError::Validation(
                "Canonical alias/confidence requires a canonical entry key".to_string(),
            ));
        }
        return Ok(None);
    };
    let entry = master_db
        .entries
        .iter()
        .find(|entry| {
            crate::services::scanner::sync::helpers::canonical_entry_key(&entry.name) == entry_key
        })
        .ok_or_else(|| {
            AppError::Validation(format!(
                "Canonical entry '{entry_key}' does not exist in the active game database"
            ))
        })?;
    if entry.entry_kind != crate::services::scanner::deep_matcher::EntryKind::Canonical {
        return Err(AppError::Validation(format!(
            "Canonical entry '{entry_key}' is taxonomy-only"
        )));
    }
    if entry.object_type != category.as_str() {
        return Err(AppError::Validation(format!(
            "Canonical entry '{entry_key}' belongs to category '{}', not '{}'",
            entry.object_type,
            category.as_str()
        )));
    }
    if let Some(alias) = alias_name.as_deref() {
        let recognized = entry.name.eq_ignore_ascii_case(alias)
            || entry
                .tags
                .iter()
                .any(|value| value.eq_ignore_ascii_case(alias))
            || entry.custom_skins.iter().any(|skin| {
                skin.name.eq_ignore_ascii_case(alias)
                    || skin
                        .aliases
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(alias))
            });
        if !recognized {
            return Err(AppError::Validation(format!(
                "Alias '{alias}' is not registered for canonical entry '{entry_key}'"
            )));
        }
    }
    Ok(Some(CanonicalClassificationMatch {
        entry_key,
        alias_name,
        confidence: confidence_percentage.map(|value| f64::from(value) / 100.0),
        reason: Some("Confirmed in classification wizard".to_string()),
        source: "classification_wizard".to_string(),
    }))
}
