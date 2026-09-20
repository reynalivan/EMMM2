use super::staging::stage_import_batch_sources_with_options;
use super::target_manifest_index::TargetManifestIndexState;
use super::types::{
    AnalysisResult, ImportBatch, ImportBatchStatus, ImportContentKind, ImportDecision,
    ImportItemStatus, ImportPackageShape, SetImportItemDecisionInput, StableCategory,
};
use crate::modules::catalog::application::match_engine::classification::classify_source_with_content;
use crate::modules::catalog::application::match_engine::inspection::{
    inspect_source_with_content, InspectionRequest,
};
use crate::modules::ingestion::adapters::sqlite::import_batch;
use crate::modules::library::application::mods::archive::StagingExtractOptions;
use crate::modules::matching::application::deep_matcher::analysis::content::PreparedTokenFilters;
use crate::modules::matching::application::deep_matcher::MasterDb;
use crate::shared::errors::AppError;
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tauri::Manager;

pub async fn analyze_import_batch_for_app(
    app: &tauri::AppHandle,
    db: &SqlitePool,
    batch_id: &str,
) -> Result<ImportBatch, AppError> {
    analyze_import_batch_for_app_with_options(app, db, batch_id, StagingExtractOptions::default())
        .await
}

pub async fn analyze_import_batch_for_app_with_options(
    app: &tauri::AppHandle,
    db: &SqlitePool,
    batch_id: &str,
    options: StagingExtractOptions,
) -> Result<ImportBatch, AppError> {
    let batch = import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    let game_type = crate::modules::games::adapters::sqlite::game::get_game_type(db, &batch.game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{}'", batch.game_id)))?
        as i32;
    let master_db =
        crate::modules::workspace::application::scanner::master_db::get_cached(app, game_type)
            .await?;
    let resource_dir = app.path().resource_dir().map_err(AppError::from)?;
    let schema = crate::modules::games::application::game::schema_loader::load_schema(
        &resource_dir,
        game_type,
    );
    let filters = crate::modules::workspace::application::scanner::master_db::ini_filters(
        Some(&resource_dir),
        game_type,
    );
    let staging_root = app
        .path()
        .app_data_dir()
        .map_err(AppError::from)?
        .join("import-staging");

    let target_manifest_index = app.state::<TargetManifestIndexState>().inner().clone();
    analyze_import_batch_with_options_and_index(
        db,
        batch_id,
        &staging_root,
        &master_db,
        &filters,
        &schema.match_extensions,
        options,
        &target_manifest_index,
    )
    .await
}

pub async fn analyze_import_batch(
    db: &SqlitePool,
    batch_id: &str,
    staging_root: &Path,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
    match_extensions: &[String],
) -> Result<ImportBatch, AppError> {
    let target_manifest_index = TargetManifestIndexState::new();
    analyze_import_batch_with_options_and_index(
        db,
        batch_id,
        staging_root,
        master_db,
        ini_filters,
        match_extensions,
        StagingExtractOptions::default(),
        &target_manifest_index,
    )
    .await
}

pub async fn analyze_import_batch_with_options(
    db: &SqlitePool,
    batch_id: &str,
    staging_root: &Path,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
    match_extensions: &[String],
    options: StagingExtractOptions,
) -> Result<ImportBatch, AppError> {
    let target_manifest_index = TargetManifestIndexState::new();
    analyze_import_batch_with_options_and_index(
        db,
        batch_id,
        staging_root,
        master_db,
        ini_filters,
        match_extensions,
        options,
        &target_manifest_index,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn analyze_import_batch_with_options_and_index(
    db: &SqlitePool,
    batch_id: &str,
    staging_root: &Path,
    master_db: &MasterDb,
    ini_filters: &PreparedTokenFilters,
    match_extensions: &[String],
    options: StagingExtractOptions,
    target_manifest_index: &TargetManifestIndexState,
) -> Result<ImportBatch, AppError> {
    target_manifest_index.clear_batch(batch_id);
    let mut batch = import_batch::get_batch(db, batch_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Import batch '{batch_id}'")))?;
    if matches!(
        batch.status,
        ImportBatchStatus::Draft | ImportBatchStatus::Failed | ImportBatchStatus::Partial
    ) {
        batch =
            stage_import_batch_sources_with_options(db, batch_id, staging_root, &options).await?;
    }
    if batch.status != ImportBatchStatus::AwaitingReview {
        return Err(AppError::Validation(format!(
            "Import batch '{batch_id}' cannot be analyzed from status {:?}",
            batch.status
        )));
    }

    let mut payload_representatives = BTreeMap::<String, String>::new();
    let mut payload_duplicates = Vec::<(String, String)>::new();
    for item in batch
        .items
        .iter()
        .filter(|item| item.status == ImportItemStatus::Staged)
    {
        if super::is_cancelled(&options.cancel_token) {
            return Err(AppError::Cancelled);
        }
        let analysis_path = item
            .staging_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&item.source_path));
        let manifest_path = analysis_path.clone();
        let cancellation = options.cancel_token.clone();
        let payload_manifest = tokio::task::spawn_blocking(move || {
            super::payload_manifest::build_validated_import_payload_manifest(
                &manifest_path,
                cancellation.as_deref(),
            )
        })
        .await??;
        if let Some(representative_item_id) =
            payload_representatives.get(&payload_manifest.content_sha256)
        {
            payload_duplicates.push((item.id.clone(), representative_item_id.clone()));
        } else {
            payload_representatives
                .insert(payload_manifest.content_sha256.clone(), item.id.clone());
        }
        let inspected = inspect_source_with_content(&InspectionRequest {
            source_path: analysis_path.clone(),
            planned_name: Some(item.planned_name.clone()),
            match_extensions: match_extensions.to_vec(),
        })?;
        let mut signal_cache =
            crate::modules::matching::application::deep_matcher::state::signal_cache::SignalCache::new();
        let categories = classify_source_with_content(
            &analysis_path,
            &item.planned_name,
            master_db,
            ini_filters,
            &inspected.content,
            &mut signal_cache,
        );
        let (content_kind, package_shape) = classify_package_content(
            &analysis_path,
            &inspected.inspection,
            categories.first().map(|suggestion| suggestion.category),
        );
        let source_metadata =
            super::gamebanana::enrich_source_metadata(db, &batch.game_id, item).await;
        let source_evidence = super::gamebanana::source_metadata_evidence(&source_metadata);
        if super::is_cancelled(&options.cancel_token) {
            return Err(AppError::Cancelled);
        }
        let selected_category = categories.first();
        let category = selected_category
            .map(|suggestion| suggestion.category)
            .unwrap_or(StableCategory::Other);
        let sub_category = selected_category.and_then(|suggestion| suggestion.sub_category.clone());
        let classification_metadata = selected_category
            .map(|suggestion| suggestion.metadata.clone())
            .unwrap_or_else(|| serde_json::json!({}));
        let mut analysis_item = item.clone();
        analysis_item.content_kind = content_kind;
        analysis_item.package_shape = package_shape;
        let mut suggestions = super::coordinator::build_match_suggestions_with_prepared_content(
            db,
            &analysis_item,
            &batch,
            &inspected.inspection,
            category,
            &classification_metadata,
            content_kind,
            master_db,
            ini_filters,
            &inspected.content,
            &mut signal_cache,
        )
        .await?;
        if let Some(evidence) = source_evidence {
            for suggestion in &mut suggestions.canonical {
                suggestion.evidence.push(evidence.clone());
            }
            suggestions.evidence.push(evidence);
        }
        let base_review_gate =
            super::coordinator::review_gate_for(&analysis_item, Some(&suggestions.canonical), None);
        let target_comparison = if base_review_gate.is_empty()
            && suggestions.canonical.first().is_some_and(|suggestion| {
                suggestion.match_status == super::types::ImportMatchStatus::AutoMatched
            }) {
            if let Some(destination) = suggestions.destinations.first() {
                let input = SetImportItemDecisionInput {
                    item_id: item.id.clone(),
                    decision: ImportDecision::Confirm,
                    destination_object_id: destination.object_id.clone(),
                    destination_path: Some(destination.target_path.clone()),
                    canonical_entry_key: destination.canonical_entry_key.clone(),
                    matched_alias: None,
                };
                super::coordinator::inspect_existing_target(
                    db,
                    target_manifest_index,
                    &analysis_item,
                    &input,
                    Some(&payload_manifest),
                )
                .await?
            } else {
                None
            }
        } else {
            None
        };
        let review_gate = super::coordinator::review_gate_for(
            &analysis_item,
            Some(&suggestions.canonical),
            target_comparison.as_ref(),
        );
        let analysis_result = AnalysisResult {
            inspection: inspected.inspection,
            category_suggestions: categories,
            selected_category: category,
            selected_sub_category: sub_category,
            classification_metadata,
            source_metadata,
            payload_manifest,
            canonical_suggestions: suggestions.canonical,
            destination_suggestions: suggestions.destinations,
            evidence: suggestions.evidence,
            content_kind,
            package_shape,
            diagnostics: analysis_item.diagnostics,
            review_gate,
            target_comparison,
        };
        if !import_batch::apply_analysis_result(db, &item.id, &analysis_result).await? {
            return Err(AppError::Validation(format!(
                "Import item '{}' changed while analysis was running",
                item.id
            )));
        }
    }

    for (item_id, representative_item_id) in payload_duplicates {
        if !import_batch::mark_payload_duplicate(db, &item_id, &representative_item_id).await? {
            return Err(AppError::Validation(format!(
                "Import item '{item_id}' changed while payload duplicate analysis was running"
            )));
        }
    }

    if super::is_cancelled(&options.cancel_token) {
        return Err(AppError::Cancelled);
    }

    import_batch::get_batch(db, batch_id).await?.ok_or_else(|| {
        AppError::Internal("Analyzed import batch could not be reloaded".to_string())
    })
}

fn classify_package_content(
    source_path: &Path,
    inspection: &crate::modules::catalog::application::match_engine::types::SourceInspection,
    category: Option<StableCategory>,
) -> (ImportContentKind, ImportPackageShape) {
    let name_evidence = std::iter::once(inspection.source_name.as_str())
        .chain(inspection.nested_names.iter().map(String::as_str))
        .chain(inspection.ini_sections.iter().map(String::as_str))
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let contains = |needle: &str| name_evidence.iter().any(|value| value.contains(needle));
    let content_kind = if [
        "zzmi",
        "wwmi",
        "srmi",
        "efmi",
        "zenless",
        "wuthering waves",
        "star rail",
    ]
    .iter()
    .any(|marker| contains(marker))
    {
        ImportContentKind::ForeignGame
    } else if [
        "timer",
        "freecam",
        "free camera",
        "utility",
        "tool",
        "fps unlock",
    ]
    .iter()
    .any(|marker| contains(marker))
    {
        ImportContentKind::Utility
    } else if ["patch", "hotfix", "fix only", "compatibility fix"]
        .iter()
        .any(|marker| contains(marker))
    {
        ImportContentKind::Patch
    } else if matches!(
        category,
        Some(StableCategory::Character | StableCategory::Weapon)
    ) {
        ImportContentKind::Skin
    } else {
        ImportContentKind::Unknown
    };
    let normalized_name = inspection.normalized_name.as_str();
    let root_count = crate::modules::library::application::mods::archive::classify::find_mod_roots(
        source_path,
        crate::modules::library::application::mods::archive::classify::MOD_ROOT_MAX_DEPTH,
    )
    .len();
    let package_shape = if root_count > 1 {
        ImportPackageShape::Bundle
    } else if ["body", "hat", "face"].contains(&normalized_name) {
        ImportPackageShape::Composite
    } else {
        ImportPackageShape::Single
    };
    (content_kind, package_shape)
}

#[cfg(test)]
mod package_classification_tests {
    use super::classify_package_content;
    use crate::modules::catalog::application::match_engine::types::SourceInspection;
    use crate::modules::ingestion::application::import_batch::types::{
        ImportContentKind, ImportPackageShape, SourceFingerprint, StableCategory,
    };

    fn inspection(name: &str) -> SourceInspection {
        SourceInspection {
            source_path: name.to_string(),
            source_name: name.to_string(),
            normalized_name: name.to_ascii_lowercase(),
            nested_names: Vec::new(),
            matching_files: Vec::new(),
            ini_sections: Vec::new(),
            evidence: Vec::new(),
            fingerprint: SourceFingerprint {
                path: name.to_string(),
                modified_unix_ms: "0".to_string(),
                size_bytes: "0".to_string(),
                file_count: 0,
            },
        }
    }

    #[test]
    fn utility_and_foreign_game_do_not_become_character_skins() {
        let source = tempfile::tempdir().unwrap();
        assert_eq!(
            classify_package_content(
                source.path(),
                &inspection("GIMI Timer Utility"),
                Some(StableCategory::Character)
            ),
            (ImportContentKind::Utility, ImportPackageShape::Single)
        );
        assert_eq!(
            classify_package_content(
                source.path(),
                &inspection("ZZMI Nicole"),
                Some(StableCategory::Character)
            ),
            (ImportContentKind::ForeignGame, ImportPackageShape::Single)
        );
        assert_eq!(
            classify_package_content(
                source.path(),
                &inspection("body"),
                Some(StableCategory::Character)
            ),
            (ImportContentKind::Skin, ImportPackageShape::Composite)
        );
    }

    #[test]
    fn roots_held_together_are_classified_as_a_bundle() {
        let source = tempfile::tempdir().unwrap();
        for name in ["body", "face"] {
            let root = source.path().join(name);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join("merged.ini"), "[TextureOverride]\nhash = abc\n").unwrap();
        }

        assert_eq!(
            classify_package_content(
                source.path(),
                &inspection("package"),
                Some(StableCategory::Character)
            ),
            (ImportContentKind::Skin, ImportPackageShape::Bundle)
        );
    }
}
