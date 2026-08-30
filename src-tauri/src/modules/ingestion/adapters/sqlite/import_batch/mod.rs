use crate::modules::ingestion::application::import_batch::types::{
    CanonicalSuggestion, CategorySuggestion, ConfidenceTier, DestinationSuggestion, ImportBatch,
    ImportBatchStatus, ImportDecision, ImportFlow, ImportItem, ImportItemStatus, ImportSourceKind,
    MatchEvidence, SourceFingerprint, StableCategory, TargetMode,
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

    for item in items {
        sqlx::query(
            "INSERT INTO import_jobs
             (id, batch_id, game_id, archive_path, source_kind, source_path, source_group_id,
              staging_path, planned_name, status, match_confidence, confidence_tier, decision)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'discovered', 0, 'no_match', 'pending')",
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

pub async fn attach_download_id(
    db: &SqlitePool,
    item_id: &str,
    download_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE import_jobs SET download_id = ? WHERE id = ?")
        .bind(download_id)
        .bind(item_id)
        .execute(db)
        .await?;
    Ok(())
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
                archive_path
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

    sqlx::query(
        "UPDATE import_jobs
         SET staging_path = ?, planned_name = ?, status = 'staged', updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status = 'discovered'",
    )
    .bind(&roots[0].staging_path)
    .bind(&roots[0].planned_name)
    .bind(item_id)
    .execute(&mut *tx)
    .await?;

    for root in &roots[1..] {
        sqlx::query(
            "INSERT INTO import_jobs
             (id, batch_id, download_id, game_id, archive_path, source_kind, source_path,
              source_group_id, staging_path, planned_name, status, match_confidence,
              confidence_tier, decision)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'staged', 0, 'no_match', 'pending')",
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

pub async fn store_inspection(
    db: &SqlitePool,
    item_id: &str,
    inspection: &crate::modules::catalog::application::match_engine::types::SourceInspection,
    category_suggestions: &[CategorySuggestion],
) -> Result<bool, sqlx::Error> {
    let inspection_json = serde_json::to_string(inspection)
        .map_err(|error| decode_error(format!("could not encode source inspection: {error}")))?;
    let fingerprint_json = serde_json::to_string(&inspection.fingerprint)
        .map_err(|error| decode_error(format!("could not encode source fingerprint: {error}")))?;
    let evidence_json = serde_json::to_string(&inspection.evidence)
        .map_err(|error| decode_error(format!("could not encode match evidence: {error}")))?;
    let categories_json = serde_json::to_string(category_suggestions)
        .map_err(|error| decode_error(format!("could not encode category suggestions: {error}")))?;
    let result = sqlx::query(
        "UPDATE import_jobs
         SET source_inspection = ?, source_fingerprint = ?, evidence_json = ?,
             category_suggestions_json = ?, status = 'awaiting_category', updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('discovered', 'staged', 'failed')",
    )
    .bind(inspection_json)
    .bind(fingerprint_json)
    .bind(evidence_json)
    .bind(categories_json)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn get_stored_inspection(
    db: &SqlitePool,
    item_id: &str,
) -> Result<Option<crate::modules::catalog::application::match_engine::types::SourceInspection>, sqlx::Error> {
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

pub async fn store_match_suggestions(
    db: &SqlitePool,
    item_id: &str,
    canonical: &[CanonicalSuggestion],
    destinations: &[DestinationSuggestion],
    evidence: &[MatchEvidence],
) -> Result<bool, sqlx::Error> {
    let canonical_json = serde_json::to_string(canonical).map_err(|error| {
        decode_error(format!("could not encode canonical suggestions: {error}"))
    })?;
    let destination_json = serde_json::to_string(destinations).map_err(|error| {
        decode_error(format!("could not encode destination suggestions: {error}"))
    })?;
    let evidence_json = serde_json::to_string(evidence)
        .map_err(|error| decode_error(format!("could not encode match evidence: {error}")))?;
    let (confidence, tier) = canonical
        .first()
        .map(|suggestion| {
            (
                f64::from(suggestion.confidence_percentage),
                suggestion.confidence_tier,
            )
        })
        .unwrap_or((0.0, ConfidenceTier::NoMatch));
    let result = sqlx::query(
        "UPDATE import_jobs
         SET canonical_suggestions_json = ?, destination_suggestions_json = ?, evidence_json = ?,
             match_confidence = ?, confidence_tier = ?, decision = 'pending',
             match_entry_key = NULL, match_alias_name = NULL, match_object_id = NULL,
             destination_object_id = NULL, destination_path = NULL, placed_path = NULL,
             result = NULL, error_msg = NULL, status = 'awaiting_destination',
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('awaiting_destination', 'ready', 'skipped')",
    )
    .bind(canonical_json)
    .bind(destination_json)
    .bind(evidence_json)
    .bind(confidence)
    .bind(tier.as_str())
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
             match_confidence = 0, confidence_tier = 'no_match', decision = 'pending',
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
) -> Result<bool, sqlx::Error> {
    let next_status = if input.decision == ImportDecision::Skip {
        ImportItemStatus::Skipped
    } else {
        ImportItemStatus::Ready
    };
    let result = sqlx::query(
        "UPDATE import_jobs
         SET decision = ?, destination_object_id = ?, destination_path = ?,
             match_entry_key = ?, match_alias_name = ?, status = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status IN ('awaiting_destination', 'ready', 'skipped')",
    )
    .bind(input.decision.as_str())
    .bind(&input.destination_object_id)
    .bind(&input.destination_path)
    .bind(&input.canonical_entry_key)
    .bind(&input.matched_alias)
    .bind(next_status.as_str())
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
    sqlx::query(
        "UPDATE import_jobs SET status = 'failed', error_msg = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status NOT IN ('done', 'cancelled')",
    )
    .bind(message)
    .bind(item_id)
    .execute(db)
    .await?;
    Ok(())
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
         WHERE id = ? AND status IN ('awaiting_review', 'ready', 'partial')",
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
             WHERE id = ? AND batch_id = ? AND status = 'ready'",
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

pub async fn bind_reconciled_object_id(
    db: &SqlitePool,
    item_id: &str,
    object_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE import_jobs SET match_object_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(object_id)
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
    Ok(metadata + archive_pending + analyzing + metadata_batches + completed)
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
                status, match_category, match_sub_category, classification_metadata,
                category_suggestions_json, canonical_suggestions_json, destination_suggestions_json,
                match_entry_key, match_alias_name, destination_object_id, match_object_id,
                destination_path, placed_path, match_confidence, confidence_tier, evidence_json,
                decision, source_fingerprint, result, error_msg
         FROM import_jobs WHERE batch_id = ? ORDER BY created_at, id",
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
        evidence: parse_json::<Vec<MatchEvidence>>(
            row.try_get::<&str, _>("evidence_json")?,
            "evidence_json",
        )?,
        decision: ImportDecision::from_str(row.try_get::<&str, _>("decision")?)
            .map_err(decode_error)?,
        fingerprint,
        result: row.try_get("result")?,
        error: row.try_get("error_msg")?,
    })
}

#[cfg(test)]
mod tests;
