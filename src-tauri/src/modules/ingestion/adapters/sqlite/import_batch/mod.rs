use crate::modules::ingestion::application::import_batch::types::{
    AnalysisResult, CanonicalSuggestion, CategorySuggestion, ConfidenceTier, DestinationSuggestion,
    ImportBatch, ImportBatchStatus, ImportContentKind, ImportDecision, ImportDiagnostic,
    ImportFlow, ImportItem, ImportItemStatus, ImportMatchStatus, ImportPackageShape,
    ImportSourceKind, MatchEvidence, PayloadManifestSummary, ReviewGate, ReviewReasonCode,
    SourceFingerprint, StableCategory, TargetComparison, TargetComparisonOutcome, TargetMode,
};
use sqlx::{Row, SqliteConnection, SqlitePool};
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct CreateImportBatchRecord {
    pub id: String,
    pub game_id: String,
    pub flow: ImportFlow,
    pub target_mode: TargetMode,
    pub target_object_id: Option<String>,
    pub target_subpath: Option<String>,
    pub source_archive_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewImportItemRecord {
    pub id: String,
    pub source_kind: ImportSourceKind,
    pub source_path: String,
    pub staging_path: Option<String>,
    pub planned_name: String,
}

#[derive(Debug, Clone)]
pub struct StagedRootRecord {
    pub id: String,
    pub staging_path: String,
    pub planned_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModInboxHistoryRow {
    pub source_group_id: String,
    pub source_path: String,
    pub source_kind: ImportSourceKind,
    pub processed_source_path: Option<String>,
    pub source_processed_at: String,
    pub source_deleted_at: Option<String>,
    pub destination_object_id: Option<String>,
    pub object_name: Option<String>,
    pub placed_path: String,
    pub planned_name: String,
    pub status: ImportItemStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveModInboxSource {
    pub source_path: String,
    pub batch_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModInboxSourceProcessingState {
    pub processed_source_path: Option<String>,
    pub source_processed_at: Option<String>,
}

fn decode_error(message: String) -> sqlx::Error {
    sqlx::Error::Decode(message.into())
}

fn parse_json<T: serde::de::DeserializeOwned>(raw: &str, field: &str) -> Result<T, sqlx::Error> {
    serde_json::from_str(raw)
        .map_err(|error| decode_error(format!("invalid import_jobs.{field}: {error}")))
}

pub async fn create_batch(
    db: &SqlitePool,
    batch: &CreateImportBatchRecord,
    items: &[NewImportItemRecord],
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    insert_batch_records(&mut tx, batch, items).await?;
    tx.commit().await
}

async fn insert_batch_records(
    conn: &mut SqliteConnection,
    batch: &CreateImportBatchRecord,
    items: &[NewImportItemRecord],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO import_batches
         (id, game_id, flow, target_mode, target_object_id, target_subpath, source_archive_path, status)
         VALUES (?, ?, ?, ?, ?, ?, ?, 'draft')",
    )
    .bind(&batch.id)
    .bind(&batch.game_id)
    .bind(batch.flow.as_str())
    .bind(batch.target_mode.as_str())
    .bind(&batch.target_object_id)
    .bind(&batch.target_subpath)
    .bind(&batch.source_archive_path)
    .execute(&mut *conn)
    .await?;

    for (source_order, item) in items.iter().enumerate() {
        sqlx::query(
            "INSERT INTO import_jobs
             (id, batch_id, game_id, archive_path, source_kind, source_path, source_group_id,
              staging_path, planned_name, source_order, root_order, status, match_confidence,
              confidence_tier, decision)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 'discovered', 0, 'no_match', 'pending')",
        )
        .bind(&item.id)
        .bind(&batch.id)
        .bind(&batch.game_id)
        .bind(&item.source_path)
        .bind(item.source_kind.as_str())
        .bind(&item.source_path)
        .bind(&item.id)
        .bind(&item.staging_path)
        .bind(&item.planned_name)
        .bind(i64::try_from(source_order).map_err(|_| {
            decode_error("source order exceeds supported integer range".to_string())
        })?)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Creates a Mod Inbox batch while holding SQLite's write reservation.
///
/// The active-source check and inserts share one `BEGIN IMMEDIATE` transaction,
/// so two callers cannot reserve the same source from stale snapshots.
pub async fn create_mod_inbox_batch_if_sources_available(
    db: &SqlitePool,
    batch: &CreateImportBatchRecord,
    items: &[NewImportItemRecord],
) -> Result<Option<ActiveModInboxSource>, sqlx::Error> {
    let mut conn = db.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;
    let operation = async {
        for item in items {
            let conflict = sqlx::query(
                "SELECT j.source_path, b.id AS batch_id
                 FROM import_jobs j
                 JOIN import_batches b ON b.id = j.batch_id
                 WHERE b.game_id = ? AND b.flow = 'ready_to_move'
                   AND b.status NOT IN ('done', 'failed', 'cancelled')
                   AND j.source_path = ? COLLATE NOCASE
                 LIMIT 1",
            )
            .bind(&batch.game_id)
            .bind(&item.source_path)
            .fetch_optional(&mut *conn)
            .await?;
            if let Some(row) = conflict {
                return Ok(Some(ActiveModInboxSource {
                    source_path: row.try_get("source_path")?,
                    batch_id: row.try_get("batch_id")?,
                }));
            }
        }
        insert_batch_records(&mut conn, batch, items).await?;
        Ok(None)
    }
    .await;

    match operation {
        Ok(None) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
            Ok(None)
        }
        Ok(Some(conflict)) => {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            Ok(Some(conflict))
        }
        Err(error) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(error)
        }
    }
}

pub async fn transition_item_status(
    db: &SqlitePool,
    item_id: &str,
    expected: ImportItemStatus,
    next: ImportItemStatus,
) -> Result<bool, sqlx::Error> {
    if !expected.can_transition_to(next) {
        return Ok(false);
    }
    let result = sqlx::query(
        "UPDATE import_jobs SET status = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status = ?",
    )
    .bind(next.as_str())
    .bind(item_id)
    .bind(expected.as_str())
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn replace_archive_item_with_roots(
    db: &SqlitePool,
    item_id: &str,
    roots: &[StagedRootRecord],
) -> Result<bool, sqlx::Error> {
    if roots.is_empty() {
        return Ok(false);
    }
    let mut tx = db.begin().await?;
    let Some(source) = sqlx::query(
        "SELECT batch_id, game_id, download_id, source_kind, source_path, source_group_id,
                archive_path, source_order
         FROM import_jobs WHERE id = ? AND status = 'discovered'",
    )
    .bind(item_id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(false);
    };
    let batch_id: String = source.try_get("batch_id")?;
    let game_id: Option<String> = source.try_get("game_id")?;
    let download_id: Option<String> = source.try_get("download_id")?;
    let source_kind: String = source.try_get("source_kind")?;
    let source_path: Option<String> = source.try_get("source_path")?;
    let source_group_id: Option<String> = source.try_get("source_group_id")?;
    let archive_path: Option<String> = source.try_get("archive_path")?;
    let source_order: i64 = source.try_get("source_order")?;

    sqlx::query(
        "UPDATE import_jobs
         SET staging_path = ?, planned_name = ?, root_order = 0, status = 'staged', updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status = 'discovered'",
    )
    .bind(&roots[0].staging_path)
    .bind(&roots[0].planned_name)
    .bind(item_id)
    .execute(&mut *tx)
    .await?;

    for (root_order, root) in roots[1..].iter().enumerate() {
        sqlx::query(
            "INSERT INTO import_jobs
             (id, batch_id, download_id, game_id, archive_path, source_kind, source_path,
              source_group_id, staging_path, planned_name, status, match_confidence,
              confidence_tier, decision, source_order, root_order)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'staged', 0, 'no_match', 'pending', ?, ?)",
        )
        .bind(&root.id)
        .bind(&batch_id)
        .bind(&download_id)
        .bind(&game_id)
        .bind(&archive_path)
        .bind(&source_kind)
        .bind(&source_path)
        .bind(&source_group_id)
        .bind(&root.staging_path)
        .bind(&root.planned_name)
        .bind(source_order)
        .bind(
            i64::try_from(root_order + 1).map_err(|_| {
                decode_error("root order exceeds supported integer range".to_string())
            })?,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(true)
}

pub async fn get_item(db: &SqlitePool, item_id: &str) -> Result<Option<ImportItem>, sqlx::Error> {
    let batch_id: Option<String> =
        sqlx::query_scalar("SELECT batch_id FROM import_jobs WHERE id = ?")
            .bind(item_id)
            .fetch_optional(db)
            .await?
            .flatten();
    let Some(batch_id) = batch_id else {
        return Ok(None);
    };
    Ok(get_batch(db, &batch_id)
        .await?
        .and_then(|batch| batch.items.into_iter().find(|item| item.id == item_id)))
}

pub async fn list_batches(
    db: &SqlitePool,
    game_id: Option<&str>,
) -> Result<Vec<ImportBatch>, sqlx::Error> {
    let ids = if let Some(game_id) = game_id {
        sqlx::query_scalar::<_, String>(
            "SELECT id FROM import_batches WHERE game_id = ? ORDER BY updated_at DESC LIMIT 100",
        )
        .bind(game_id)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_scalar::<_, String>(
            "SELECT id FROM import_batches ORDER BY updated_at DESC LIMIT 100",
        )
        .fetch_all(db)
        .await?
    };
    let mut batches = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(batch) = get_batch(db, &id).await? {
            batches.push(batch);
        }
    }
    Ok(batches)
}

pub async fn list_mod_inbox_history_rows(
    db: &SqlitePool,
    game_id: &str,
) -> Result<Vec<ModInboxHistoryRow>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT COALESCE(j.source_group_id, j.id) AS source_group_id,
                j.source_path, j.source_kind, j.processed_source_path,
                j.source_processed_at, j.source_deleted_at,
                COALESCE(m.object_id, j.destination_object_id, j.match_object_id)
                    AS destination_object_id,
                o.name AS object_name, COALESCE(m.folder_path, j.placed_path) AS placed_path,
                j.planned_name, j.status
         FROM import_jobs j
         JOIN import_batches b ON b.id = j.batch_id
         LEFT JOIN mods m ON m.id = j.destination_mod_id
         LEFT JOIN objects o ON o.id = COALESCE(m.object_id, j.destination_object_id, j.match_object_id)
         WHERE b.game_id = ? AND b.flow = 'ready_to_move'
           AND j.source_processed_at IS NOT NULL AND j.placed_path IS NOT NULL
         ORDER BY j.source_processed_at DESC, source_group_id, j.created_at, j.id",
    )
    .bind(game_id)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(ModInboxHistoryRow {
                source_group_id: row.try_get("source_group_id")?,
                source_path: row.try_get("source_path")?,
                source_kind: ImportSourceKind::from_str(row.try_get::<&str, _>("source_kind")?)
                    .map_err(decode_error)?,
                processed_source_path: row.try_get("processed_source_path")?,
                source_processed_at: row.try_get("source_processed_at")?,
                source_deleted_at: row.try_get("source_deleted_at")?,
                destination_object_id: row.try_get("destination_object_id")?,
                object_name: row.try_get("object_name")?,
                placed_path: row.try_get("placed_path")?,
                planned_name: row.try_get("planned_name")?,
                status: ImportItemStatus::from_str(row.try_get::<&str, _>("status")?)
                    .map_err(decode_error)?,
            })
        })
        .collect()
}

pub async fn list_active_mod_inbox_sources(
    db: &SqlitePool,
    game_id: &str,
) -> Result<Vec<ActiveModInboxSource>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT j.source_path, b.id AS batch_id
         FROM import_jobs j
         JOIN import_batches b ON b.id = j.batch_id
         WHERE b.game_id = ? AND b.flow = 'ready_to_move'
           AND b.status NOT IN ('done', 'failed', 'cancelled')
         GROUP BY j.source_path, b.id
         ORDER BY b.updated_at DESC",
    )
    .bind(game_id)
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(ActiveModInboxSource {
                source_path: row.try_get("source_path")?,
                batch_id: row.try_get("batch_id")?,
            })
        })
        .collect()
}

/// SQLite permits a limited number of bind parameters per statement. Keep the
/// game id plus selected source paths safely below the commonly configured 999.
const ACTIVE_SOURCE_PATH_BATCH_SIZE: usize = 900;

pub async fn list_active_mod_inbox_sources_for_paths(
    db: &SqlitePool,
    game_id: &str,
    source_paths: &[String],
) -> Result<Vec<ActiveModInboxSource>, sqlx::Error> {
    let mut sources = Vec::new();
    for path_chunk in source_paths.chunks(ACTIVE_SOURCE_PATH_BATCH_SIZE) {
        let placeholders = std::iter::repeat("?")
            .take(path_chunk.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT j.source_path, b.id AS batch_id
             FROM import_jobs j
             JOIN import_batches b ON b.id = j.batch_id
             WHERE b.game_id = ? AND b.flow = 'ready_to_move'
               AND b.status NOT IN ('done', 'failed', 'cancelled')
               AND j.source_path IN ({placeholders})
             GROUP BY j.source_path, b.id
             ORDER BY b.updated_at DESC"
        );
        let mut query = sqlx::query(&sql).bind(game_id);
        for path in path_chunk {
            query = query.bind(path);
        }
        let rows = query.fetch_all(db).await?;
        for row in rows {
            sources.push(ActiveModInboxSource {
                source_path: row.try_get("source_path")?,
                batch_id: row.try_get("batch_id")?,
            });
        }
    }
    Ok(sources)
}

#[cfg(test)]
mod active_source_tests {
    use super::*;

    #[tokio::test]
    async fn selected_source_queries_are_chunked_below_the_sqlite_parameter_limit() {
        let context = crate::test_utils::init_test_db().await;
        let source_paths = (0..901)
            .map(|index| format!("C:/Inbox/Mod {index}"))
            .collect::<Vec<_>>();

        let sources = list_active_mod_inbox_sources_for_paths(
            &context.pool,
            "game-without-sources",
            &source_paths,
        )
        .await
        .unwrap();

        assert!(sources.is_empty());
    }
}

#[cfg(test)]
pub async fn mark_mod_inbox_source_processed(
    db: &SqlitePool,
    batch_id: &str,
    source_path: &str,
    processed_source_path: Option<&str>,
) -> Result<u64, sqlx::Error> {
    complete_mod_inbox_source_processing(db, batch_id, source_path, processed_source_path).await
}

pub async fn plan_mod_inbox_source_processing(
    db: &SqlitePool,
    batch_id: &str,
    source_path: &str,
    processed_source_path: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET processed_source_path = ?,
             updated_at = CURRENT_TIMESTAMP
         WHERE batch_id = ? AND source_path = ? AND source_processed_at IS NULL",
    )
    .bind(processed_source_path)
    .bind(batch_id)
    .bind(source_path)
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

pub async fn complete_mod_inbox_source_processing(
    db: &SqlitePool,
    batch_id: &str,
    source_path: &str,
    processed_source_path: Option<&str>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET processed_source_path = COALESCE(?, processed_source_path),
             source_processed_at = COALESCE(source_processed_at, CURRENT_TIMESTAMP),
             updated_at = CURRENT_TIMESTAMP
         WHERE batch_id = ? AND source_path = ?",
    )
    .bind(processed_source_path)
    .bind(batch_id)
    .bind(source_path)
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

pub async fn get_mod_inbox_source_processing_state(
    db: &SqlitePool,
    batch_id: &str,
    source_path: &str,
) -> Result<Option<ModInboxSourceProcessingState>, sqlx::Error> {
    sqlx::query(
        "SELECT MAX(processed_source_path) AS processed_source_path,
                MAX(source_processed_at) AS source_processed_at
         FROM import_jobs WHERE batch_id = ? AND source_path = ?",
    )
    .bind(batch_id)
    .bind(source_path)
    .fetch_optional(db)
    .await?
    .map(|row| {
        Ok(ModInboxSourceProcessingState {
            processed_source_path: row.try_get("processed_source_path")?,
            source_processed_at: row.try_get("source_processed_at")?,
        })
    })
    .transpose()
}

pub async fn mark_mod_inbox_source_deleted(
    db: &SqlitePool,
    game_id: &str,
    source_group_id: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET source_deleted_at = COALESCE(source_deleted_at, CURRENT_TIMESTAMP),
             updated_at = CURRENT_TIMESTAMP
         WHERE game_id = ? AND source_group_id = ?
           AND processed_source_path IS NOT NULL
           AND batch_id IN (SELECT id FROM import_batches WHERE flow = 'ready_to_move')",
    )
    .bind(game_id)
    .bind(source_group_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

pub async fn set_batch_status(
    db: &SqlitePool,
    batch_id: &str,
    status: ImportBatchStatus,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE import_batches SET status = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(batch_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn has_batch_review_started(
    db: &SqlitePool,
    batch_id: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM import_batches
            WHERE id = ? AND review_started_at IS NOT NULL
        )",
    )
    .bind(batch_id)
    .fetch_one(db)
    .await
}

pub async fn mark_batch_review_started(
    db: &SqlitePool,
    batch_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_batches
         SET review_started_at = COALESCE(review_started_at, CURRENT_TIMESTAMP)
         WHERE id = ?",
    )
    .bind(batch_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn get_stored_inspection(
    db: &SqlitePool,
    item_id: &str,
) -> Result<
    Option<crate::modules::catalog::application::match_engine::types::SourceInspection>,
    sqlx::Error,
> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT source_inspection FROM import_jobs WHERE id = ?")
            .bind(item_id)
            .fetch_optional(db)
            .await?
            .flatten();
    raw.map(|value| parse_json(&value, "source_inspection"))
        .transpose()
}

pub async fn store_classification(
    db: &SqlitePool,
    input: &crate::modules::ingestion::application::import_batch::types::SetImportItemClassificationInput,
) -> Result<bool, sqlx::Error> {
    let metadata = serde_json::to_string(&input.metadata).map_err(|error| {
        decode_error(format!("could not encode classification metadata: {error}"))
    })?;
    let result = sqlx::query(
        "UPDATE import_jobs
         SET match_category = ?, match_sub_category = ?, classification_metadata = ?,
             canonical_suggestions_json = '[]', destination_suggestions_json = '[]',
             match_entry_key = NULL, match_alias_name = NULL, match_object_id = NULL,
             destination_object_id = NULL, destination_path = NULL, decision = 'pending',
             status = 'awaiting_destination', updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status = 'awaiting_category'",
    )
    .bind(input.category.as_str())
    .bind(&input.sub_category)
    .bind(metadata)
    .bind(&input.item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn apply_analysis_result(
    db: &SqlitePool,
    item_id: &str,
    analysis: &AnalysisResult,
) -> Result<bool, sqlx::Error> {
    let inspection_json = serde_json::to_string(&analysis.inspection)
        .map_err(|error| decode_error(format!("could not encode source inspection: {error}")))?;
    let fingerprint_json = serde_json::to_string(&analysis.inspection.fingerprint)
        .map_err(|error| decode_error(format!("could not encode source fingerprint: {error}")))?;
    let categories_json = serde_json::to_string(&analysis.category_suggestions)
        .map_err(|error| decode_error(format!("could not encode category suggestions: {error}")))?;
    let metadata_json =
        serde_json::to_string(&analysis.classification_metadata).map_err(|error| {
            decode_error(format!("could not encode classification metadata: {error}"))
        })?;
    let manifest_json = serde_json::to_string(&analysis.payload_manifest)
        .map_err(|error| decode_error(format!("could not encode payload manifest: {error}")))?;
    let canonical_json =
        serde_json::to_string(&analysis.canonical_suggestions).map_err(|error| {
            decode_error(format!("could not encode canonical suggestions: {error}"))
        })?;
    // Every object is scored once during analysis, but a missing entry is
    // semantically a zero-score result in the UI. Persisting those entries
    // duplicates the full object library for every import item.
    let stored_destinations = analysis
        .destination_suggestions
        .iter()
        .filter(|suggestion| suggestion.confidence_percentage > 0 || suggestion.object_id.is_none())
        .collect::<Vec<_>>();
    let destination_json = serde_json::to_string(&stored_destinations).map_err(|error| {
        decode_error(format!("could not encode destination suggestions: {error}"))
    })?;
    let evidence_json = serde_json::to_string(&analysis.evidence)
        .map_err(|error| decode_error(format!("could not encode match evidence: {error}")))?;
    let diagnostics_json = serde_json::to_string(&analysis.diagnostics)
        .map_err(|error| decode_error(format!("could not encode import diagnostics: {error}")))?;
    let review_gate_json = serde_json::to_string(&analysis.review_gate)
        .map_err(|error| decode_error(format!("could not encode review gate: {error}")))?;
    let target_comparison_json = analysis
        .target_comparison
        .as_ref()
        .map(|comparison| serde_json::to_string(comparison))
        .transpose()
        .map_err(|error| decode_error(format!("could not encode target comparison: {error}")))?;
    let identity_match_status = if !analysis.review_gate.is_empty() {
        ImportMatchStatus::NeedsReview
    } else {
        analysis
            .canonical_suggestions
            .first()
            .map(|suggestion| suggestion.match_status)
            .unwrap_or(ImportMatchStatus::NoMatch)
    };
    let default_destination = analysis
        .canonical_suggestions
        .first()
        .filter(|suggestion| {
            analysis.review_gate.is_empty()
                && suggestion.match_status
                    == crate::modules::ingestion::application::import_batch::types::ImportMatchStatus::AutoMatched
        })
        .and_then(|_| stored_destinations.first().copied());
    let (confidence, tier) = analysis
        .canonical_suggestions
        .first()
        .map(|suggestion| {
            (
                f64::from(suggestion.confidence_percentage),
                suggestion.confidence_tier,
            )
        })
        .unwrap_or((0.0, ConfidenceTier::NoMatch));
    let (
        mut decision,
        mut status,
        mut destination_object_id,
        mut destination_path,
        mut canonical_entry_key,
    ) = if let Some(destination) = default_destination {
        let kind = serde_json::to_value(&destination.kind)
            .map_err(|error| decode_error(format!("could not encode destination kind: {error}")))?;
        let decision = match kind.as_str() {
            Some("specific_target") => "keep_specific_target",
            Some("existing_object") => "reallocate",
            Some("create_canonical") => "create_canonical",
            _ => "confirm",
        };
        (
            decision,
            "ready",
            destination.object_id.as_deref(),
            Some(destination.target_path.as_str()),
            destination.canonical_entry_key.as_deref(),
        )
    } else {
        ("pending", "awaiting_destination", None, None, None)
    };
    if let Some(comparison) = &analysis.target_comparison {
        match comparison.outcome {
            TargetComparisonOutcome::AlreadyInstalled => {
                decision = "skip";
                status = "skipped";
                destination_object_id = None;
                destination_path = None;
                canonical_entry_key = None;
            }
            TargetComparisonOutcome::TargetHasAdditionalFiles
            | TargetComparisonOutcome::SameNameDifferentContent
            | TargetComparisonOutcome::Incomplete => {
                decision = "pending";
                status = "awaiting_destination";
                destination_object_id = None;
                destination_path = None;
                canonical_entry_key = None;
            }
        }
    }
    let mut tx = db.begin().await?;
    let result = sqlx::query(
        "UPDATE import_jobs
             SET source_inspection = ?, source_fingerprint = ?, category_suggestions_json = ?,
                 match_category = ?, match_sub_category = ?, classification_metadata = ?, source_metadata_json = ?,
             payload_manifest_json = ?, canonical_suggestions_json = ?,
             destination_suggestions_json = ?, evidence_json = ?, diagnostics_json = ?,
             review_gate_json = ?, target_comparison_json = ?, content_kind = ?, package_shape = ?,
             match_confidence = ?, confidence_tier = ?, decision = ?,
             identity_match_status = ?, analysis_revision = analysis_revision + 1,
             analysis_ack_revision = NULL,
             match_entry_key = ?, match_alias_name = NULL, match_object_id = ?,
             destination_object_id = ?, destination_path = ?, placed_path = NULL,
             result = NULL, error_msg = NULL, status = ?,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('staged', 'awaiting_category', 'awaiting_destination', 'ready', 'skipped')",
    )
    .bind(inspection_json)
    .bind(fingerprint_json)
    .bind(categories_json)
    .bind(analysis.selected_category.as_str())
    .bind(&analysis.selected_sub_category)
    .bind(metadata_json)
    .bind(serde_json::to_string(&analysis.source_metadata).map_err(|error| {
        decode_error(format!("could not encode source metadata: {error}"))
    })?)
    .bind(manifest_json)
    .bind(canonical_json)
    .bind(destination_json)
    .bind(evidence_json)
    .bind(diagnostics_json)
    .bind(review_gate_json)
    .bind(target_comparison_json)
    .bind(analysis.content_kind.as_str())
    .bind(analysis.package_shape.as_str())
    .bind(confidence)
    .bind(tier.as_str())
    .bind(decision)
    .bind(identity_match_status.as_str())
    .bind(canonical_entry_key)
    .bind(destination_object_id)
    .bind(destination_object_id)
    .bind(destination_path)
    .bind(status)
    .bind(item_id)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() != 1 {
        tx.rollback().await?;
        return Ok(false);
    }
    tx.commit().await?;
    Ok(true)
}

pub async fn store_match_suggestions(
    db: &SqlitePool,
    item_id: &str,
    canonical: &[CanonicalSuggestion],
    destinations: &[DestinationSuggestion],
    evidence: &[MatchEvidence],
    review_gate: &ReviewGate,
) -> Result<bool, sqlx::Error> {
    let canonical_json = serde_json::to_string(canonical).map_err(|error| {
        decode_error(format!("could not encode canonical suggestions: {error}"))
    })?;
    let stored_destinations = destinations
        .iter()
        .filter(|suggestion| suggestion.confidence_percentage > 0 || suggestion.object_id.is_none())
        .collect::<Vec<_>>();
    let destination_json = serde_json::to_string(&stored_destinations).map_err(|error| {
        decode_error(format!("could not encode destination suggestions: {error}"))
    })?;
    let evidence_json = serde_json::to_string(evidence)
        .map_err(|error| decode_error(format!("could not encode match evidence: {error}")))?;
    let review_gate_json = serde_json::to_string(review_gate)
        .map_err(|error| decode_error(format!("could not encode review gate: {error}")))?;
    let identity_match_status = if review_gate.is_empty() {
        canonical
            .first()
            .map(|suggestion| suggestion.match_status)
            .unwrap_or(ImportMatchStatus::NoMatch)
    } else {
        ImportMatchStatus::NeedsReview
    };
    let default_destination = canonical
        .first()
        .filter(|suggestion| {
            review_gate.is_empty() && suggestion.match_status == ImportMatchStatus::AutoMatched
        })
        .and_then(|_| stored_destinations.first().copied());
    let (confidence, tier) = canonical
        .first()
        .map(|suggestion| {
            (
                f64::from(suggestion.confidence_percentage),
                suggestion.confidence_tier,
            )
        })
        .unwrap_or((0.0, ConfidenceTier::NoMatch));
    let (decision, status, destination_object_id, destination_path, canonical_entry_key) =
        match default_destination {
            Some(destination) => {
                let decision = match destination.kind {
                    crate::modules::ingestion::application::import_batch::types::DestinationKind::SpecificTarget => "keep_specific_target",
                    crate::modules::ingestion::application::import_batch::types::DestinationKind::ExistingObject => "reallocate",
                    crate::modules::ingestion::application::import_batch::types::DestinationKind::CreateCanonical => "create_canonical",
                };
                (
                    decision,
                    "ready",
                    destination.object_id.as_deref(),
                    Some(destination.target_path.as_str()),
                    destination.canonical_entry_key.as_deref(),
                )
            }
            None => ("pending", "awaiting_destination", None, None, None),
        };
    let result = sqlx::query(
        "UPDATE import_jobs
         SET canonical_suggestions_json = ?, destination_suggestions_json = ?, evidence_json = ?,
             review_gate_json = ?, match_confidence = ?, confidence_tier = ?, decision = ?,
             identity_match_status = ?, analysis_revision = analysis_revision + 1,
             analysis_ack_revision = NULL, match_entry_key = ?, match_alias_name = NULL,
             match_object_id = ?, destination_object_id = ?, destination_path = ?, placed_path = NULL,
             result = NULL, error_msg = NULL, status = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('awaiting_destination', 'ready', 'skipped')",
    )
    .bind(canonical_json)
    .bind(destination_json)
    .bind(evidence_json)
    .bind(review_gate_json)
    .bind(confidence)
    .bind(tier.as_str())
    .bind(decision)
    .bind(identity_match_status.as_str())
    .bind(canonical_entry_key)
    .bind(destination_object_id)
    .bind(destination_object_id)
    .bind(destination_path)
    .bind(status)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn rename_planned_item(
    db: &SqlitePool,
    item_id: &str,
    planned_name: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET planned_name = ?, canonical_suggestions_json = '[]', destination_suggestions_json = '[]',
             match_confidence = 0, confidence_tier = 'no_match', identity_match_status = 'needs_review',
             decision = 'pending', analysis_revision = analysis_revision + 1,
             analysis_ack_revision = NULL, target_comparison_json = NULL,
             review_gate_json = '{\"reasons\":[]}',
             match_entry_key = NULL, match_alias_name = NULL, destination_object_id = NULL,
             match_object_id = NULL, destination_path = NULL, placed_path = NULL,
             status = CASE WHEN match_category IS NULL THEN 'awaiting_category' ELSE 'awaiting_destination' END,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('discovered', 'staged', 'awaiting_category',
                                      'awaiting_destination', 'ready', 'skipped', 'failed')",
    )
    .bind(planned_name)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn store_decision(
    db: &SqlitePool,
    input: &crate::modules::ingestion::application::import_batch::types::SetImportItemDecisionInput,
    confidence: u8,
    tier: ConfidenceTier,
) -> Result<bool, sqlx::Error> {
    let next_status = if input.decision == ImportDecision::Skip {
        ImportItemStatus::Skipped
    } else {
        ImportItemStatus::Ready
    };
    let result = sqlx::query(
        "UPDATE import_jobs
         SET decision = ?, destination_object_id = ?, destination_path = ?,
             match_entry_key = ?, match_alias_name = ?, match_confidence = ?, confidence_tier = ?,
             analysis_ack_revision = analysis_revision,
             status = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('awaiting_destination', 'ready', 'skipped')",
    )
    .bind(input.decision.as_str())
    .bind(&input.destination_object_id)
    .bind(&input.destination_path)
    .bind(&input.canonical_entry_key)
    .bind(&input.matched_alias)
    .bind(f64::from(confidence))
    .bind(tier.as_str())
    .bind(next_status.as_str())
    .bind(&input.item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn store_archive_hash(
    db: &SqlitePool,
    item_id: &str,
    archive_sha256: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs SET archive_sha256 = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('discovered', 'staged', 'awaiting_destination', 'ready', 'skipped')",
    )
    .bind(archive_sha256)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn mark_archive_duplicate(
    db: &SqlitePool,
    item_id: &str,
    representative_item_id: &str,
    archive_sha256: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET archive_sha256 = ?, duplicate_of_item_id = ?, decision = 'skip', status = 'skipped',
             result = 'duplicate_archive', error_msg = NULL, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status = 'discovered'",
    )
    .bind(archive_sha256)
    .bind(representative_item_id)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
pub async fn store_payload_manifest(
    db: &SqlitePool,
    item_id: &str,
    manifest: &crate::modules::ingestion::application::import_batch::payload_manifest::PayloadManifest,
) -> Result<bool, sqlx::Error> {
    let raw = serde_json::to_string(manifest)
        .map_err(|error| decode_error(format!("could not encode payload manifest: {error}")))?;
    let result = sqlx::query(
        "UPDATE import_jobs
         SET payload_manifest_json = ?, target_comparison_json = NULL,
             analysis_revision = analysis_revision + 1, analysis_ack_revision = NULL,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('staged', 'awaiting_destination', 'ready', 'skipped')",
    )
    .bind(raw)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn append_item_diagnostic(
    db: &SqlitePool,
    item_id: &str,
    diagnostic: &ImportDiagnostic,
) -> Result<bool, sqlx::Error> {
    let Some(existing) = sqlx::query(
        "SELECT diagnostics_json, review_gate_json FROM import_jobs
         WHERE id = ? AND status IN ('staged', 'awaiting_category', 'awaiting_destination', 'ready')",
    )
    .bind(item_id)
    .fetch_optional(db)
    .await?
    else {
        return Ok(false);
    };
    let existing_diagnostics: String = existing.try_get("diagnostics_json")?;
    let existing_review_gate: String = existing.try_get("review_gate_json")?;
    let mut diagnostics: Vec<ImportDiagnostic> =
        parse_json(&existing_diagnostics, "diagnostics_json")?;
    if diagnostics.iter().any(|current| {
        current.code == diagnostic.code
            && current.stage == diagnostic.stage
            && current.member_path == diagnostic.member_path
    }) {
        return Ok(true);
    }
    diagnostics.push(diagnostic.clone());
    let raw = serde_json::to_string(&diagnostics)
        .map_err(|error| decode_error(format!("could not encode import diagnostic: {error}")))?;
    let mut review_gate: ReviewGate = parse_json(&existing_review_gate, "review_gate_json")?;
    review_gate.add(
        ReviewReasonCode::IncompleteInspection,
        Some(diagnostic.code.clone()),
    );
    let review_gate_json = serde_json::to_string(&review_gate)
        .map_err(|error| decode_error(format!("could not encode review gate: {error}")))?;
    let result = sqlx::query(
        "UPDATE import_jobs
         SET diagnostics_json = ?, review_gate_json = ?, identity_match_status = 'needs_review',
             analysis_revision = analysis_revision + 1, analysis_ack_revision = NULL,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('staged', 'awaiting_category', 'awaiting_destination', 'ready')",
    )
    .bind(raw)
    .bind(review_gate_json)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn mark_payload_duplicate(
    db: &SqlitePool,
    item_id: &str,
    representative_item_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET duplicate_of_item_id = ?, decision = 'skip', status = 'skipped',
             result = 'duplicate_payload', error_msg = NULL, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('awaiting_destination', 'ready')",
    )
    .bind(representative_item_id)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn apply_target_comparison(
    db: &SqlitePool,
    item_id: &str,
    comparison: &TargetComparison,
    review_gate: &ReviewGate,
) -> Result<bool, sqlx::Error> {
    let comparison_json = serde_json::to_string(comparison)
        .map_err(|error| decode_error(format!("could not encode target comparison: {error}")))?;
    let review_gate_json = serde_json::to_string(review_gate)
        .map_err(|error| decode_error(format!("could not encode review gate: {error}")))?;
    let (decision, status, result) = match comparison.outcome {
        TargetComparisonOutcome::AlreadyInstalled => ("skip", "skipped", "already_installed"),
        TargetComparisonOutcome::TargetHasAdditionalFiles
        | TargetComparisonOutcome::SameNameDifferentContent
        | TargetComparisonOutcome::Incomplete => {
            ("pending", "awaiting_destination", "target_conflict")
        }
    };
    let identity_match_status = if review_gate.is_empty() {
        None
    } else {
        Some("needs_review")
    };
    let result = sqlx::query(
        "UPDATE import_jobs
         SET target_comparison_json = ?, review_gate_json = ?, decision = ?, status = ?, result = ?,
             identity_match_status = COALESCE(?, identity_match_status),
             destination_object_id = CASE WHEN ? THEN NULL ELSE destination_object_id END,
             destination_path = CASE WHEN ? THEN NULL ELSE destination_path END,
             placed_path = NULL,
             analysis_revision = analysis_revision + 1, analysis_ack_revision = NULL,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('awaiting_destination', 'ready', 'skipped')",
    )
    .bind(comparison_json)
    .bind(review_gate_json)
    .bind(decision)
    .bind(status)
    .bind(result)
    .bind(identity_match_status)
    .bind(comparison.outcome != TargetComparisonOutcome::Incomplete)
    .bind(comparison.outcome != TargetComparisonOutcome::Incomplete)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn get_payload_manifest(
    db: &SqlitePool,
    item_id: &str,
) -> Result<
    Option<crate::modules::ingestion::application::import_batch::payload_manifest::PayloadManifest>,
    sqlx::Error,
> {
    let raw = sqlx::query_scalar::<_, Option<String>>(
        "SELECT payload_manifest_json FROM import_jobs WHERE id = ?",
    )
    .bind(item_id)
    .fetch_optional(db)
    .await?
    .flatten();
    raw.map(|raw| {
        parse_json::<
            crate::modules::ingestion::application::import_batch::payload_manifest::PayloadManifest,
        >(&raw, "payload_manifest_json")
    })
    .transpose()
}

pub async fn store_keep_separate_decision(
    db: &SqlitePool,
    input: &crate::modules::ingestion::application::import_batch::types::SetImportItemDecisionInput,
    planned_name: &str,
    confidence: u8,
    tier: ConfidenceTier,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET planned_name = ?, decision = 'keep_separate', destination_object_id = ?,
             destination_path = ?, match_entry_key = ?, match_alias_name = ?,
             match_confidence = ?, confidence_tier = ?, status = 'ready',
             analysis_ack_revision = analysis_revision, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('awaiting_destination', 'ready', 'skipped')",
    )
    .bind(planned_name)
    .bind(&input.destination_object_id)
    .bind(&input.destination_path)
    .bind(&input.canonical_entry_key)
    .bind(&input.matched_alias)
    .bind(f64::from(confidence))
    .bind(tier.as_str())
    .bind(&input.item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn cancel_batch(db: &SqlitePool, batch_id: &str) -> Result<bool, sqlx::Error> {
    let mut tx = db.begin().await?;
    let batch = sqlx::query(
        "UPDATE import_batches SET status = 'cancelled'
         WHERE id = ? AND status NOT IN ('committing', 'done', 'cancelled')",
    )
    .bind(batch_id)
    .execute(&mut *tx)
    .await?;
    if batch.rows_affected() == 0 {
        return Ok(false);
    }
    sqlx::query(
        "UPDATE import_jobs SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP
         WHERE batch_id = ? AND status NOT IN ('done', 'cancelled')",
    )
    .bind(batch_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn set_item_failure(
    db: &SqlitePool,
    item_id: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    let diagnostics_json = serde_json::to_string(&[failure_diagnostic(message)])
        .map_err(|error| decode_error(format!("could not encode import diagnostic: {error}")))?;
    sqlx::query(
        "UPDATE import_jobs
         SET status = 'failed', error_msg = ?, diagnostics_json = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status NOT IN ('done', 'cancelled')",
    )
    .bind(message)
    .bind(diagnostics_json)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(())
}

fn failure_diagnostic(message: &str) -> ImportDiagnostic {
    use crate::modules::ingestion::application::import_batch::types::ImportDiagnosticStage;

    let normalized = message.to_ascii_lowercase();
    let (code, stage, recovery) = if normalized.contains("mod_root_too_deep") {
        (
            "root_too_deep",
            ImportDiagnosticStage::RootDiscovery,
            "Unwrap one or more wrapper folders, then retry analysis",
        )
    } else if normalized.contains("entry limit") {
        (
            "root_scan_limit",
            ImportDiagnosticStage::RootDiscovery,
            "Reduce the archive tree or inspect the source manually",
        )
    } else if normalized.contains("multipart") {
        (
            "archive_multipart",
            ImportDiagnosticStage::Staging,
            "Place every archive part beside the first part, then retry",
        )
    } else if normalized.contains("nested_archive_depth_limit")
        || normalized.contains("nested_archive_destination_collision")
    {
        (
            "nested_archive_unresolved",
            ImportDiagnosticStage::Staging,
            "Open the nested archive manually or remove the conflicting wrapper folder",
        )
    } else if normalized.contains("password") {
        (
            "archive_password",
            ImportDiagnosticStage::Staging,
            "Provide the archive password and retry",
        )
    } else if normalized.contains("unsupported") || normalized.contains("compression") {
        (
            "archive_compression_unsupported",
            ImportDiagnosticStage::Staging,
            "Extract with a compatible archiver, then import the folder",
        )
    } else if normalized.contains("no valid .ini") {
        (
            "archive_no_mod_root",
            ImportDiagnosticStage::RootDiscovery,
            "Choose the folder containing the runnable mod INI",
        )
    } else if normalized.contains("disappeared") {
        (
            "source_changed",
            ImportDiagnosticStage::Staging,
            "Restore the source or refresh the inbox",
        )
    } else {
        (
            "staging_failed",
            ImportDiagnosticStage::Staging,
            "Inspect the source and retry analysis",
        )
    };
    ImportDiagnostic {
        code: code.to_string(),
        stage,
        member_path: None,
        recovery: recovery.to_string(),
    }
}

pub async fn restore_item_after_rollback(
    db: &SqlitePool,
    item_id: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE import_jobs
         SET status = 'ready', placed_path = NULL, result = 'rolled_back',
             error_msg = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status = 'committing'",
    )
    .bind(message)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn begin_batch_commit(
    db: &SqlitePool,
    batch_id: &str,
    item_ids: &[String],
) -> Result<bool, sqlx::Error> {
    if item_ids.is_empty() {
        return Ok(false);
    }
    let mut tx = db.begin().await?;
    let batch = sqlx::query(
        "UPDATE import_batches SET status = 'committing'
         WHERE id = ?
           AND status IN ('awaiting_review', 'ready', 'partial')
           AND review_started_at IS NOT NULL",
    )
    .bind(batch_id)
    .execute(&mut *tx)
    .await?;
    if batch.rows_affected() == 0 {
        return Ok(false);
    }
    for item_id in item_ids {
        let item = sqlx::query(
            "UPDATE import_jobs SET status = 'committing', error_msg = NULL, updated_at = CURRENT_TIMESTAMP
             WHERE id = ? AND batch_id = ? AND status = 'ready'
               AND analysis_revision > 0
               AND ((identity_match_status = 'auto_matched' AND json_array_length(review_gate_json, '$.reasons') = 0)
                    OR analysis_ack_revision = analysis_revision)",
        )
        .bind(item_id)
        .bind(batch_id)
        .execute(&mut *tx)
        .await?;
        if item.rows_affected() != 1 {
            tx.rollback().await?;
            return Ok(false);
        }
    }
    tx.commit().await?;
    Ok(true)
}

pub async fn set_commit_item_state(
    db: &SqlitePool,
    item_id: &str,
    status: ImportItemStatus,
    destination_path: Option<&str>,
    result: Option<&str>,
    error: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE import_jobs
         SET status = ?, destination_path = COALESCE(?, destination_path),
             placed_path = COALESCE(?, placed_path), result = ?, error_msg = ?,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(status.as_str())
    .bind(destination_path)
    .bind(destination_path)
    .bind(result)
    .bind(error)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn bind_reconciled_destination(
    db: &SqlitePool,
    item_id: &str,
    object_id: &str,
    mod_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE import_jobs
         SET match_object_id = ?, destination_mod_id = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(object_id)
    .bind(mod_id)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn prepare_items_for_recovery(
    db: &SqlitePool,
    batch_id: &str,
    item_ids: &[String],
) -> Result<bool, sqlx::Error> {
    if item_ids.is_empty() {
        return Ok(false);
    }
    let mut tx = db.begin().await?;
    let batch = sqlx::query(
        "UPDATE import_batches SET status = 'committing', updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('partial', 'committing', 'awaiting_review', 'ready')",
    )
    .bind(batch_id)
    .execute(&mut *tx)
    .await?;
    if batch.rows_affected() != 1 {
        return Ok(false);
    }
    for item_id in item_ids {
        let item = sqlx::query(
            "UPDATE import_jobs
             SET status = CASE
                 WHEN status IN ('partial', 'reconciling') THEN 'reconciling'
                 WHEN status IN ('metadata_pending', 'finalizing_metadata') THEN 'finalizing_metadata'
                 ELSE status END,
                 error_msg = NULL, updated_at = CURRENT_TIMESTAMP
             WHERE id = ? AND batch_id = ?
               AND status IN ('partial', 'reconciling', 'metadata_pending', 'finalizing_metadata')",
        )
        .bind(item_id)
        .bind(batch_id)
        .execute(&mut *tx)
        .await?;
        if item.rows_affected() != 1 {
            tx.rollback().await?;
            return Ok(false);
        }
    }
    tx.commit().await?;
    Ok(true)
}

/// Makes every crash-interrupted import phase visible and resumable again.
///
/// A Mod Inbox source is deliberately left `partial/archive_pending` when
/// its children reached `done` before source finalization/history completed.
/// The commit coordinator can then idempotently finish the inbox operation.
pub async fn recover_interrupted_batch_states(db: &SqlitePool) -> Result<u64, sqlx::Error> {
    let mut tx = db.begin().await?;
    let metadata = sqlx::query(
        "UPDATE import_jobs SET status = 'metadata_pending',
                result = 'metadata_pending',
                error_msg = COALESCE(error_msg, 'Interrupted while finalizing metadata'),
                updated_at = CURRENT_TIMESTAMP
         WHERE status = 'finalizing_metadata'",
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let completed_metadata = sqlx::query(
        "UPDATE import_jobs
         SET error_msg = NULL, result = COALESCE(result, 'done'), updated_at = CURRENT_TIMESTAMP
         WHERE status = 'done' AND error_msg = 'Interrupted while finalizing metadata'",
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let archive_pending = sqlx::query(
        "UPDATE import_jobs
         SET status = 'partial', result = 'archive_pending',
             error_msg = COALESCE(error_msg, 'ReadyToMove archive finalization was interrupted'),
             updated_at = CURRENT_TIMESTAMP
         WHERE status = 'done'
           AND batch_id IN (
             SELECT id FROM import_batches
             WHERE flow = 'ready_to_move' AND status = 'committing'
           )",
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let analyzing = sqlx::query(
        "UPDATE import_batches SET status = 'partial', updated_at = CURRENT_TIMESTAMP
         WHERE status = 'analyzing'",
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let metadata_batches = sqlx::query(
        "UPDATE import_batches SET status = 'partial', updated_at = CURRENT_TIMESTAMP
         WHERE status = 'committing'
           AND id IN (
             SELECT DISTINCT batch_id FROM import_jobs
             WHERE status IN ('metadata_pending', 'partial')
           )",
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let completed = sqlx::query(
        "UPDATE import_batches SET status = 'done', updated_at = CURRENT_TIMESTAMP
         WHERE status = 'committing'
           AND EXISTS (SELECT 1 FROM import_jobs WHERE batch_id = import_batches.id)
           AND NOT EXISTS (
             SELECT 1 FROM import_jobs
             WHERE batch_id = import_batches.id AND status <> 'done'
           )",
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok(metadata + completed_metadata + archive_pending + analyzing + metadata_batches + completed)
}

pub async fn reset_failed_item_for_staging(
    db: &SqlitePool,
    item_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET status = 'discovered', staging_path = NULL, error_msg = NULL,
             result = NULL, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status = 'failed'",
    )
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn mark_ready_to_move_archive_pending(
    db: &SqlitePool,
    batch_id: &str,
    message: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET status = 'partial', result = 'archive_pending', error_msg = ?,
             updated_at = CURRENT_TIMESTAMP
         WHERE batch_id = ? AND staging_path IS NOT NULL
           AND (status = 'done' OR (status = 'partial' AND result = 'archive_pending'))",
    )
    .bind(message)
    .bind(batch_id)
    .execute(db)
    .await?;
    set_batch_status(db, batch_id, ImportBatchStatus::Partial).await?;
    Ok(result.rows_affected())
}

pub async fn complete_ready_to_move_archive_pending(
    db: &SqlitePool,
    batch_id: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE import_jobs
         SET status = 'done', result = 'done', error_msg = NULL,
             updated_at = CURRENT_TIMESTAMP
         WHERE batch_id = ? AND status = 'partial' AND result = 'archive_pending'",
    )
    .bind(batch_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_terminal_batch_ids_for_staging_cleanup(
    db: &SqlitePool,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM import_batches WHERE status IN ('done', 'cancelled') ORDER BY updated_at",
    )
    .fetch_all(db)
    .await
}

/// Returns paths that must survive startup staging cleanup because a resumable
/// import batch still references them.
pub async fn list_active_staging_paths(db: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT j.staging_path
         FROM import_jobs j
         JOIN import_batches b ON b.id = j.batch_id
         WHERE b.status NOT IN ('done', 'cancelled')
           AND j.staging_path IS NOT NULL
           AND j.staging_path <> ''",
    )
    .fetch_all(db)
    .await
}

pub async fn finish_batch_from_items(
    db: &SqlitePool,
    batch_id: &str,
) -> Result<ImportBatchStatus, sqlx::Error> {
    let statuses =
        sqlx::query_scalar::<_, String>("SELECT status FROM import_jobs WHERE batch_id = ?")
            .bind(batch_id)
            .fetch_all(db)
            .await?;
    let all_done = !statuses.is_empty() && statuses.iter().all(|status| status == "done");
    let has_incomplete_terminal = statuses
        .iter()
        .any(|status| matches!(status.as_str(), "failed" | "metadata_pending" | "partial"))
        || (!statuses.is_empty()
            && statuses
                .iter()
                .all(|status| matches!(status.as_str(), "done" | "skipped" | "cancelled")));
    let status = if all_done {
        ImportBatchStatus::Done
    } else if has_incomplete_terminal {
        ImportBatchStatus::Partial
    } else {
        ImportBatchStatus::AwaitingReview
    };
    set_batch_status(db, batch_id, status).await?;
    Ok(status)
}

pub async fn get_batch(
    db: &SqlitePool,
    batch_id: &str,
) -> Result<Option<ImportBatch>, sqlx::Error> {
    let Some(row) = sqlx::query(
        "SELECT id, game_id, flow, target_mode, target_object_id, target_subpath,
                source_archive_path, status, created_at, updated_at
         FROM import_batches WHERE id = ?",
    )
    .bind(batch_id)
    .fetch_optional(db)
    .await?
    else {
        return Ok(None);
    };

    let item_rows = sqlx::query(
        "SELECT id, batch_id, source_kind, source_path, archive_path, staging_path, planned_name,
                 status, match_category, match_sub_category, classification_metadata, source_metadata_json,
                category_suggestions_json, canonical_suggestions_json, destination_suggestions_json,
                match_entry_key, match_alias_name, destination_object_id, match_object_id,
                destination_path, placed_path, match_confidence, confidence_tier, evidence_json,
                decision, source_fingerprint, archive_sha256, payload_manifest_json,
                duplicate_of_item_id, target_comparison_json, analysis_revision,
                analysis_ack_revision, review_gate_json, identity_match_status, diagnostics_json,
                content_kind, package_shape, result, error_msg
         FROM import_jobs WHERE batch_id = ? ORDER BY source_order, root_order, id",
    )
    .bind(batch_id)
    .fetch_all(db)
    .await?;

    let mut items = Vec::with_capacity(item_rows.len());
    for item_row in item_rows {
        items.push(map_item_row(item_row)?);
    }

    Ok(Some(ImportBatch {
        id: row.try_get("id")?,
        game_id: row.try_get("game_id")?,
        flow: ImportFlow::from_str(row.try_get::<&str, _>("flow")?).map_err(decode_error)?,
        target_mode: TargetMode::from_str(row.try_get::<&str, _>("target_mode")?)
            .map_err(decode_error)?,
        target_object_id: row.try_get("target_object_id")?,
        target_subpath: row.try_get("target_subpath")?,
        status: ImportBatchStatus::from_str(row.try_get::<&str, _>("status")?)
            .map_err(decode_error)?,
        source_archive_path: row.try_get("source_archive_path")?,
        items,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    }))
}

fn map_item_row(row: sqlx::sqlite::SqliteRow) -> Result<ImportItem, sqlx::Error> {
    let source_path = row
        .try_get::<Option<String>, _>("source_path")?
        .or(row.try_get::<Option<String>, _>("archive_path")?)
        .unwrap_or_default();
    let planned_name = row
        .try_get::<Option<String>, _>("planned_name")?
        .or_else(|| {
            Path::new(&source_path)
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let confidence = row
        .try_get::<Option<f64>, _>("match_confidence")?
        .unwrap_or(0.0);
    let confidence_percentage = if confidence <= 1.0 {
        (confidence * 100.0).round().clamp(0.0, 100.0) as u8
    } else {
        confidence.round().clamp(0.0, 100.0) as u8
    };
    let category = row
        .try_get::<Option<String>, _>("match_category")?
        .map(|value| StableCategory::from_str(&value).map_err(decode_error))
        .transpose()?;
    let fingerprint = row
        .try_get::<Option<String>, _>("source_fingerprint")?
        .map(|raw| parse_json::<SourceFingerprint>(&raw, "source_fingerprint"))
        .transpose()?;
    let archive_sha256 = row.try_get::<Option<String>, _>("archive_sha256")?;
    let payload_manifest = row
        .try_get::<Option<String>, _>("payload_manifest_json")?
        .map(|raw| {
            parse_json::<crate::modules::ingestion::application::import_batch::payload_manifest::PayloadManifest>(
                &raw,
                "payload_manifest_json",
            )
            .map(|manifest| PayloadManifestSummary {
                version: manifest.version,
                file_count: manifest.file_count,
                total_size_bytes: manifest.total_size_bytes,
                content_sha256: manifest.content_sha256,
            })
        })
        .transpose()?;
    let duplicate_of_item_id = row.try_get::<Option<String>, _>("duplicate_of_item_id")?;
    let target_comparison = row
        .try_get::<Option<String>, _>("target_comparison_json")?
        .map(|raw| parse_json::<TargetComparison>(&raw, "target_comparison_json"))
        .transpose()?;
    let analysis_revision = u64::try_from(row.try_get::<i64, _>("analysis_revision")?)
        .map_err(|_| decode_error("invalid negative analysis_revision".to_string()))?;
    let analysis_ack_revision = row
        .try_get::<Option<i64>, _>("analysis_ack_revision")?
        .map(|value| {
            u64::try_from(value)
                .map_err(|_| decode_error("invalid negative analysis_ack_revision".to_string()))
        })
        .transpose()?;
    let review_gate = parse_json::<ReviewGate>(
        row.try_get::<&str, _>("review_gate_json")?,
        "review_gate_json",
    )?;
    let identity_match_status =
        ImportMatchStatus::from_str(row.try_get::<&str, _>("identity_match_status")?)
            .map_err(decode_error)?;
    let diagnostics = parse_json::<Vec<ImportDiagnostic>>(
        row.try_get::<&str, _>("diagnostics_json")?,
        "diagnostics_json",
    )?;
    let content_kind = ImportContentKind::from_str(row.try_get::<&str, _>("content_kind")?)
        .map_err(decode_error)?;
    let package_shape = ImportPackageShape::from_str(row.try_get::<&str, _>("package_shape")?)
        .map_err(decode_error)?;
    let stored_tier = ConfidenceTier::from_str(row.try_get::<&str, _>("confidence_tier")?)
        .unwrap_or_else(|_| ConfidenceTier::from_percentage(confidence_percentage));

    Ok(ImportItem {
        id: row.try_get("id")?,
        batch_id: row.try_get("batch_id")?,
        source_kind: ImportSourceKind::from_str(row.try_get::<&str, _>("source_kind")?)
            .map_err(decode_error)?,
        source_path,
        staging_path: row.try_get("staging_path")?,
        planned_name,
        status: ImportItemStatus::from_str(row.try_get::<&str, _>("status")?)
            .map_err(decode_error)?,
        match_category: category,
        sub_category: row.try_get("match_sub_category")?,
        classification_metadata: parse_json(
            row.try_get::<&str, _>("classification_metadata")?,
            "classification_metadata",
        )?,
        source_metadata: parse_json(
            row.try_get::<&str, _>("source_metadata_json")?,
            "source_metadata_json",
        )?,
        category_suggestions: parse_json::<Vec<CategorySuggestion>>(
            row.try_get::<&str, _>("category_suggestions_json")?,
            "category_suggestions_json",
        )?,
        canonical_suggestions: parse_json::<Vec<CanonicalSuggestion>>(
            row.try_get::<&str, _>("canonical_suggestions_json")?,
            "canonical_suggestions_json",
        )?,
        destination_suggestions: parse_json::<Vec<DestinationSuggestion>>(
            row.try_get::<&str, _>("destination_suggestions_json")?,
            "destination_suggestions_json",
        )?,
        selected_entry_key: row.try_get("match_entry_key")?,
        selected_alias_name: row.try_get("match_alias_name")?,
        destination_object_id: row
            .try_get::<Option<String>, _>("destination_object_id")?
            .or(row.try_get::<Option<String>, _>("match_object_id")?),
        destination_path: row
            .try_get::<Option<String>, _>("destination_path")?
            .or(row.try_get::<Option<String>, _>("placed_path")?),
        confidence_percentage,
        confidence_tier: stored_tier,
        identity_match_status,
        evidence: parse_json::<Vec<MatchEvidence>>(
            row.try_get::<&str, _>("evidence_json")?,
            "evidence_json",
        )?,
        decision: ImportDecision::from_str(row.try_get::<&str, _>("decision")?)
            .map_err(decode_error)?,
        fingerprint,
        archive_sha256,
        payload_manifest,
        duplicate_of_item_id,
        target_comparison,
        analysis_revision,
        analysis_ack_revision,
        review_gate,
        diagnostics,
        content_kind,
        package_shape,
        result: row.try_get("result")?,
        error: row.try_get("error_msg")?,
    })
}

#[cfg(test)]
mod tests;
