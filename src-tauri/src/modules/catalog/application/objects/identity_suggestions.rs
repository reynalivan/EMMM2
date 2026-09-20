//! Background, per-game identity suggestions backed by the canonical matcher.
//!
//! Reconciliation remains disk-only. Explicit UI actions start this bounded
//! local inspection, and a single generation per game makes stale results
//! harmless when the active game or catalog changes.

use std::{collections::HashMap, path::Path, sync::Arc};

use serde::Serialize;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use tauri::{AppHandle, Manager};

use crate::modules::{
    catalog::application::match_engine::{
        canonical_match::{
            match_canonical_objects_with_prepared_content, prepare_canonical_match_db,
        },
        inspection::{inspect_source_with_content, InspectionRequest},
    },
    ingestion::application::import_batch::types::{
        ConfidenceTier, ImportMatchStatus, MatchEvidence,
    },
    workspace::application::scanner::master_db::{self, asset_pack::CatalogPack},
};
use crate::shared::errors::AppError;

const MATCHER_REVISION: i64 = 1;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ObjectIdentitySuggestionStatus {
    pub state: String,
    pub catalog_id: Option<String>,
    pub catalog_version: Option<String>,
    pub suggested_count: u64,
    pub checked_count: u64,
    pub total_count: u64,
    pub failed_count: u64,
    pub has_keyviewer_targets: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ObjectIdentitySuggestionItem {
    pub object_id: String,
    pub object_name: String,
    pub entry_key: String,
    pub entry_name: String,
    pub confidence_percentage: u8,
    pub evidence: Vec<MatchEvidence>,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ObjectIdentitySuggestionPage {
    pub items: Vec<ObjectIdentitySuggestionItem>,
    pub total: u64,
    pub next_offset: Option<u32>,
}

#[derive(Clone)]
struct JobProgress {
    generation: u64,
    state: &'static str,
    checked_count: u64,
    total_count: u64,
    failed_count: u64,
    message: Option<String>,
}

#[derive(Clone, Default)]
pub struct IdentitySuggestionState {
    jobs: Arc<tokio::sync::Mutex<HashMap<String, JobProgress>>>,
}

impl IdentitySuggestionState {
    async fn begin(&self, game_id: &str) -> (u64, bool) {
        let mut jobs = self.jobs.lock().await;
        let replaces_active_job = jobs.get(game_id).is_some_and(|job| job.state == "checking");
        let generation = jobs
            .get(game_id)
            .map_or(1, |job| job.generation.saturating_add(1));
        jobs.insert(
            game_id.to_string(),
            JobProgress {
                generation,
                state: "checking",
                checked_count: 0,
                total_count: 0,
                failed_count: 0,
                message: None,
            },
        );
        (generation, replaces_active_job)
    }

    async fn update(&self, game_id: &str, generation: u64, update: impl FnOnce(&mut JobProgress)) {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs
            .get_mut(game_id)
            .filter(|job| job.generation == generation)
        {
            update(job);
        }
    }

    async fn status(&self, game_id: &str) -> Option<JobProgress> {
        self.jobs.lock().await.get(game_id).cloned()
    }

    async fn is_current(&self, game_id: &str, generation: u64) -> bool {
        self.jobs
            .lock()
            .await
            .get(game_id)
            .is_some_and(|job| job.generation == generation && job.state == "checking")
    }
}

#[derive(Debug)]
struct CandidateObject {
    id: String,
    name: String,
    folder_path: String,
    updated_at: String,
}

pub async fn schedule(
    app: AppHandle,
    game_id: String,
    changed_roots: Option<Vec<String>>,
) -> Result<(), AppError> {
    let state = app.state::<IdentitySuggestionState>().inner().clone();
    let (generation, replaces_active_job) = state.begin(&game_id).await;
    // A second explicit check replaces the first generation. Re-reading the
    // DB is cheap because cached object revisions filter unchanged sources.
    let changed_roots = (!replaces_active_job).then_some(changed_roots).flatten();
    tauri::async_runtime::spawn(async move {
        if !state.is_current(&game_id, generation).await {
            return;
        }
        if let Err(error) = run(&app, &game_id, generation, &state, changed_roots).await {
            state
                .update(&game_id, generation, |job| {
                    job.state = "error";
                    job.message = Some(error.to_string());
                })
                .await;
            log::warn!("identity suggestion check failed for game {game_id}: {error}");
        }
    });
    Ok(())
}

pub async fn status(
    app: &AppHandle,
    pool: &SqlitePool,
    game_id: &str,
) -> Result<ObjectIdentitySuggestionStatus, AppError> {
    let game_type = crate::modules::games::adapters::sqlite::game::get_game_type(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}'")))?;
    let app_data_dir = app.path().app_data_dir()?;
    let pack = match CatalogPack::load(&app_data_dir) {
        Ok(pack) => pack,
        Err(error) if error.to_string().contains("not installed") => {
            return Ok(empty_status("catalog_missing"));
        }
        Err(error) => {
            let mut result = empty_status("error");
            result.message = Some(error.to_string());
            return Ok(result);
        }
    };
    let entries = pack.entries_for(game_type as i32)?;
    if entries.is_empty() {
        let mut result = empty_status("unsupported");
        result.catalog_id = Some(pack.id().to_string());
        result.catalog_version = Some(pack.version().to_string());
        return Ok(result);
    }
    let suggested_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM object_identity_checks checks
         JOIN objects object ON object.id = checks.object_id
         WHERE checks.game_id = ?
           AND checks.catalog_id = ?
           AND checks.catalog_version = ?
           AND checks.matcher_revision = ?
           AND checks.object_revision = object.updated_at
           AND checks.candidate_entry_key IS NOT NULL
           AND checks.match_status = 'auto_matched'
           AND checks.confidence_percentage >= 75
           AND object.matched_entry_key IS NULL
           AND NOT EXISTS (SELECT 1 FROM object_runtime_projection projection
             WHERE projection.game_id = object.game_id AND projection.object_id = object.id
               AND projection.has_naming_conflict = 1)
           AND NOT EXISTS (
             SELECT 1 FROM object_identity_dismissals dismissals
             WHERE dismissals.game_id = checks.game_id
               AND dismissals.object_id = checks.object_id
               AND dismissals.catalog_id = checks.catalog_id
               AND dismissals.candidate_entry_key = checks.candidate_entry_key
           )",
    )
    .bind(game_id)
    .bind(pack.id())
    .bind(pack.version())
    .bind(MATCHER_REVISION)
    .fetch_one(pool)
    .await?;
    let job = app
        .state::<IdentitySuggestionState>()
        .inner()
        .status(game_id)
        .await;
    let has_keyviewer_targets = !pack.keyviewer_entries_for(game_type as i32)?.is_empty();
    Ok(ObjectIdentitySuggestionStatus {
        state: job.as_ref().map_or("ready", |job| job.state).to_string(),
        catalog_id: Some(pack.id().to_string()),
        catalog_version: Some(pack.version().to_string()),
        suggested_count: suggested_count.max(0) as u64,
        checked_count: job.as_ref().map_or(0, |job| job.checked_count),
        total_count: job.as_ref().map_or(0, |job| job.total_count),
        failed_count: job.as_ref().map_or(0, |job| job.failed_count),
        has_keyviewer_targets,
        message: job.and_then(|job| job.message),
    })
}

pub async fn list(
    app: &AppHandle,
    pool: &SqlitePool,
    game_id: &str,
    offset: u32,
    limit: u32,
) -> Result<ObjectIdentitySuggestionPage, AppError> {
    let status = status(app, pool, game_id).await?;
    let (Some(catalog_id), Some(catalog_version)) = (status.catalog_id, status.catalog_version)
    else {
        return Ok(ObjectIdentitySuggestionPage {
            items: Vec::new(),
            total: 0,
            next_offset: None,
        });
    };
    let limit = limit.clamp(1, 100);
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM object_identity_checks checks JOIN objects object ON object.id = checks.object_id
         WHERE checks.game_id = ? AND checks.catalog_id = ? AND checks.catalog_version = ?
           AND checks.matcher_revision = ? AND checks.object_revision = object.updated_at
           AND checks.candidate_entry_key IS NOT NULL
           AND checks.match_status = 'auto_matched' AND checks.confidence_percentage >= 75
           AND object.matched_entry_key IS NULL
           AND NOT EXISTS (SELECT 1 FROM object_runtime_projection projection
             WHERE projection.game_id = object.game_id AND projection.object_id = object.id
               AND projection.has_naming_conflict = 1)
           AND NOT EXISTS (SELECT 1 FROM object_identity_dismissals dismissals
             WHERE dismissals.game_id = checks.game_id AND dismissals.object_id = checks.object_id
               AND dismissals.catalog_id = checks.catalog_id AND dismissals.candidate_entry_key = checks.candidate_entry_key)",
    )
    .bind(game_id).bind(&catalog_id).bind(&catalog_version).bind(MATCHER_REVISION)
    .fetch_one(pool).await?;
    let rows = sqlx::query(
        "SELECT checks.object_id, object.name AS object_name, checks.candidate_entry_key,
                checks.candidate_name, checks.confidence_percentage, checks.evidence_json, checks.thumbnail_path
         FROM object_identity_checks checks JOIN objects object ON object.id = checks.object_id
         WHERE checks.game_id = ? AND checks.catalog_id = ? AND checks.catalog_version = ?
           AND checks.matcher_revision = ? AND checks.object_revision = object.updated_at
           AND checks.candidate_entry_key IS NOT NULL
           AND checks.match_status = 'auto_matched' AND checks.confidence_percentage >= 75
           AND object.matched_entry_key IS NULL
           AND NOT EXISTS (SELECT 1 FROM object_runtime_projection projection
             WHERE projection.game_id = object.game_id AND projection.object_id = object.id
               AND projection.has_naming_conflict = 1)
           AND NOT EXISTS (SELECT 1 FROM object_identity_dismissals dismissals
             WHERE dismissals.game_id = checks.game_id AND dismissals.object_id = checks.object_id
               AND dismissals.catalog_id = checks.catalog_id AND dismissals.candidate_entry_key = checks.candidate_entry_key)
         ORDER BY checks.confidence_percentage DESC, object.name COLLATE NOCASE ASC LIMIT ? OFFSET ?",
    )
    .bind(game_id).bind(&catalog_id).bind(&catalog_version).bind(MATCHER_REVISION)
    .bind(i64::from(limit)).bind(i64::from(offset)).fetch_all(pool).await?;
    let items = rows
        .into_iter()
        .map(|row| {
            let evidence_json: String = row.try_get("evidence_json")?;
            Ok(ObjectIdentitySuggestionItem {
                object_id: row.try_get("object_id")?,
                object_name: row.try_get("object_name")?,
                entry_key: row.try_get("candidate_entry_key")?,
                entry_name: row.try_get("candidate_name")?,
                confidence_percentage: row
                    .try_get::<i64, _>("confidence_percentage")?
                    .clamp(0, 100) as u8,
                evidence: serde_json::from_str(&evidence_json).unwrap_or_default(),
                thumbnail_path: row.try_get("thumbnail_path")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    let total = total.max(0) as u64;
    let next_offset =
        (u64::from(offset) + (items.len() as u64) < total).then_some(offset + items.len() as u32);
    Ok(ObjectIdentitySuggestionPage {
        items,
        total,
        next_offset,
    })
}

pub async fn dismiss(
    app: &AppHandle,
    pool: &SqlitePool,
    game_id: &str,
    object_id: &str,
) -> Result<(), AppError> {
    let status = status(app, pool, game_id).await?;
    let (Some(catalog_id), Some(catalog_version)) = (status.catalog_id, status.catalog_version)
    else {
        return Ok(());
    };
    let candidate: Option<String> = sqlx::query_scalar(
        "SELECT candidate_entry_key FROM object_identity_checks WHERE game_id = ? AND object_id = ?
         AND catalog_id = ? AND catalog_version = ? AND matcher_revision = ?",
    )
    .bind(game_id)
    .bind(object_id)
    .bind(&catalog_id)
    .bind(&catalog_version)
    .bind(MATCHER_REVISION)
    .fetch_optional(pool)
    .await?;
    if let Some(candidate_entry_key) = candidate {
        sqlx::query("INSERT OR IGNORE INTO object_identity_dismissals (game_id, object_id, catalog_id, candidate_entry_key) VALUES (?, ?, ?, ?)")
            .bind(game_id).bind(object_id).bind(&catalog_id).bind(candidate_entry_key).execute(pool).await?;
    }
    Ok(())
}

pub async fn reset_dismissals(pool: &SqlitePool, game_id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM object_identity_dismissals WHERE game_id = ?")
        .bind(game_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn run(
    app: &AppHandle,
    game_id: &str,
    generation: u64,
    state: &IdentitySuggestionState,
    changed_roots: Option<Vec<String>>,
) -> Result<(), AppError> {
    let pool = app.state::<SqlitePool>().inner().clone();
    let game_type = crate::modules::games::adapters::sqlite::game::get_game_type(&pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}'")))?;
    let master_db = master_db::get_cached(app, game_type as i32).await?;
    if master_db.entries.is_empty() {
        state
            .update(game_id, generation, |job| job.state = "ready")
            .await;
        return Ok(());
    }
    let mods_root = crate::modules::games::adapters::sqlite::game::get_mod_path(&pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game '{game_id}' mods directory")))?;
    let resource_dir = app.path().resource_dir()?;
    let filters = master_db::ini_filters(Some(&resource_dir), game_type as i32);
    let app_data_dir = app.path().app_data_dir()?;
    let pack = CatalogPack::load(&app_data_dir)?;
    let canonical_db = prepare_canonical_match_db(&master_db);
    let roots = changed_roots
        .unwrap_or_default()
        .into_iter()
        .filter(|root| !root.trim().is_empty())
        .map(|root| crate::shared::path_key::folder_path_key(&root, None))
        .collect::<std::collections::BTreeSet<_>>();
    let mut query = QueryBuilder::<Sqlite>::new(
        "SELECT objects.id, objects.name, objects.folder_path, objects.updated_at
         FROM objects
         LEFT JOIN object_identity_checks cached
           ON cached.game_id = objects.game_id AND cached.object_id = objects.id
         WHERE objects.game_id = ",
    );
    query.push_bind(game_id);
    query.push(
        " AND objects.matched_entry_key IS NULL
          AND COALESCE(objects.matched_source, '') != 'classification_wizard_custom'
          AND NOT EXISTS (SELECT 1 FROM object_runtime_projection projection
            WHERE projection.game_id = objects.game_id AND projection.object_id = objects.id
              AND projection.has_naming_conflict = 1)",
    );
    if roots.is_empty() {
        // A full run (first install, retry, or a coalesced watcher burst)
        // needs only cache misses. A known changed root is inspected even if
        // SQLite's object revision did not change for an implementation file.
        query.push(" AND (cached.object_revision IS NULL OR cached.object_revision != objects.updated_at OR cached.catalog_id != ");
        query.push_bind(pack.id());
        query.push(" OR cached.catalog_version != ");
        query.push_bind(pack.version());
        query.push(" OR cached.matcher_revision != ");
        query.push_bind(MATCHER_REVISION);
        query.push(")");
    } else {
        query.push(" AND folder_path_key IN (");
        {
            let mut separated = query.separated(", ");
            for root in roots {
                separated.push_bind(root);
            }
        }
        query.push(")");
    }
    let rows = query.build().fetch_all(&pool).await?;
    let objects = rows
        .into_iter()
        .map(|row| {
            Ok(CandidateObject {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                folder_path: row.try_get("folder_path")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    state
        .update(game_id, generation, |job| {
            job.total_count = objects.len() as u64
        })
        .await;
    for object in objects {
        if !state.is_current(game_id, generation).await {
            return Ok(());
        }
        let source = Path::new(&mods_root).join(&object.folder_path);
        let result = inspect_source_with_content(&InspectionRequest {
            source_path: source.clone(),
            planned_name: Some(object.name.clone()),
            match_extensions: Vec::new(),
        })
        .map(|inspected| {
            let suggestions = match_canonical_objects_with_prepared_content(
                &source,
                &object.name,
                &canonical_db,
                &filters,
                &inspected.content,
            );
            (
                inspected.inspection.fingerprint,
                suggestions.into_iter().next(),
            )
        });
        match result {
            Ok((fingerprint, suggestion)) => {
                if !state.is_current(game_id, generation).await {
                    return Ok(());
                }
                let accepted = suggestion.filter(|candidate| {
                    candidate.confidence_tier == ConfidenceTier::High
                        && candidate.match_status == ImportMatchStatus::AutoMatched
                });
                let (entry_key, entry_name, confidence, match_status, evidence, thumbnail_path) =
                    if let Some(candidate) = accepted {
                        let thumbnail_path = master_db.entries.iter().find(|entry| {
                        crate::modules::workspace::application::scanner::sync::helpers::canonical_entry_key(&entry.name) == candidate.entry_key
                    }).and_then(|entry| entry.thumbnail_path.clone());
                        (
                            Some(candidate.entry_key),
                            Some(candidate.name),
                            Some(i64::from(candidate.confidence_percentage)),
                            Some("auto_matched"),
                            serde_json::to_string(&candidate.evidence)?,
                            thumbnail_path,
                        )
                    } else {
                        (None, None, None, None, "[]".to_string(), None)
                    };
                sqlx::query(
                    "INSERT INTO object_identity_checks (game_id, object_id, object_revision, source_fingerprint, catalog_id, catalog_version, matcher_revision, candidate_entry_key, candidate_name, confidence_percentage, match_status, evidence_json, thumbnail_path)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT(game_id, object_id) DO UPDATE SET object_revision = excluded.object_revision, source_fingerprint = excluded.source_fingerprint, catalog_id = excluded.catalog_id, catalog_version = excluded.catalog_version, matcher_revision = excluded.matcher_revision, candidate_entry_key = excluded.candidate_entry_key, candidate_name = excluded.candidate_name, confidence_percentage = excluded.confidence_percentage, match_status = excluded.match_status, evidence_json = excluded.evidence_json, thumbnail_path = excluded.thumbnail_path, checked_at = CURRENT_TIMESTAMP",
                ).bind(game_id).bind(&object.id).bind(&object.updated_at).bind(serde_json::to_string(&fingerprint)?)
                    .bind(pack.id()).bind(pack.version()).bind(MATCHER_REVISION).bind(entry_key).bind(entry_name).bind(confidence).bind(match_status).bind(evidence).bind(thumbnail_path)
                    .execute(&pool).await?;
            }
            Err(error) => {
                log::debug!("skipping identity check for object {}: {error}", object.id);
                state
                    .update(game_id, generation, |job| job.failed_count += 1)
                    .await;
            }
        }
        state
            .update(game_id, generation, |job| job.checked_count += 1)
            .await;
    }
    state
        .update(game_id, generation, |job| job.state = "ready")
        .await;
    Ok(())
}

fn empty_status(state: &str) -> ObjectIdentitySuggestionStatus {
    ObjectIdentitySuggestionStatus {
        state: state.to_string(),
        catalog_id: None,
        catalog_version: None,
        suggested_count: 0,
        checked_count: 0,
        total_count: 0,
        failed_count: 0,
        has_keyviewer_targets: false,
        message: None,
    }
}
