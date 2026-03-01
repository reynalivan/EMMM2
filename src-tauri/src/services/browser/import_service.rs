use chrono::Utc;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

/// DTO returned to the frontend for import queue display.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportJobDto {
    pub id: String,
    pub download_id: Option<String>,
    pub game_id: Option<String>,
    pub archive_path: String,
    pub status: String,
    pub match_category: Option<String>,
    pub match_object_id: Option<String>,
    pub match_confidence: Option<f64>,
    pub match_reason: Option<String>,
    pub placed_path: Option<String>,
    pub error_msg: Option<String>,
    pub is_duplicate: bool,
    pub created_at: String,
    pub updated_at: String,
}

// ── Queue ────────────────────────────────────────────────────────────────────

/// Enqueue a new import job and immediately spawn the pipeline.
pub async fn queue_import_job(
    db: &SqlitePool,
    app: &AppHandle,
    download_id: &str,
    session_id: Option<&str>,
    archive_path: &str,
) -> Result<String, String> {
    let job_id = Uuid::new_v4().to_string();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    // Determine game_id from session if available
    let game_id: Option<String> = match session_id {
        Some(sid) => sqlx::query_scalar!("SELECT game_id FROM download_sessions WHERE id = ?", sid)
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .flatten(),
        None => None,
    };

    sqlx::query!(
        r#"INSERT INTO import_jobs
           (id, download_id, game_id, archive_path, status, is_duplicate, created_at, updated_at)
           VALUES (?, ?, ?, ?, 'queued', 0, ?, ?)"#,
        job_id,
        download_id,
        game_id,
        archive_path,
        now,
        now
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB insert import_job failed: {e}"))?;

    // Spawn the pipeline asynchronously
    let db_c = db.clone();
    let app_c = app.clone();
    let job_id_c = job_id.clone();
    let archive = archive_path.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_pipeline(&db_c, &app_c, &job_id_c, &archive).await {
            log::error!("Import pipeline error for job {job_id_c}: {e}");
            let _ = set_job_status(&db_c, &job_id_c, "failed", Some(&e)).await;
            let _ = app_c.emit(
                "import:job-update",
                serde_json::json!({
                    "job_id": job_id_c,
                    "status": "failed",
                    "error": e,
                }),
            );
        }
    });

    Ok(job_id)
}

// ── Pipeline ─────────────────────────────────────────────────────────────────

async fn run_pipeline(
    db: &SqlitePool,
    app: &AppHandle,
    job_id: &str,
    archive_path: &str,
) -> Result<(), String> {
    let archive = PathBuf::from(archive_path);

    // -- Step 1: Validate extension is allowed --
    validate_extension(db, &archive).await?;

    // -- Step 2: Hash (BLAKE3) + dedup check --
    emit_status(app, job_id, "extracting", None);
    set_job_status(db, job_id, "extracting", None).await?;

    let hash = hash_file(&archive)?;

    // Check dedup
    let existing = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM import_jobs WHERE archive_hash = ? AND status = 'done'",
        hash
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    if existing > 0 {
        sqlx::query!(
            "UPDATE import_jobs SET is_duplicate = 1 WHERE id = ?",
            job_id
        )
        .execute(db)
        .await
        .ok();
        // Inform frontend — user must decide what to do with duplicate
        emit_status(
            app,
            job_id,
            "needs_review",
            Some(serde_json::json!({
                "reason": "duplicate",
                "archive_hash": hash,
            })),
        );
        return set_job_status(db, job_id, "needs_review", None).await;
    }

    // Store hash
    sqlx::query!(
        "UPDATE import_jobs SET archive_hash = ? WHERE id = ?",
        hash,
        job_id
    )
    .execute(db)
    .await
    .ok();

    // -- Step 3: Stage (copy to staging dir) --
    let staging_path = stage_archive(app, job_id, &archive).await?;
    let staging_path_str = staging_path.to_string_lossy().to_string();
    sqlx::query!(
        "UPDATE import_jobs SET staging_path = ? WHERE id = ?",
        staging_path_str,
        job_id
    )
    .execute(db)
    .await
    .ok();

    // -- Step 4: Extract --
    let extract_dir = staging_path.parent().unwrap().join("extracted");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extract dir: {e}"))?;

    let password: Option<&str> = None; // No password UI yet; extend later
    crate::services::mods::archive::extract_archive(&staging_path, &extract_dir, password, true)?;

    // -- Step 5: Validate (check for at least one .ini file) --
    let ini_count = walkdir::WalkDir::new(&extract_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "ini").unwrap_or(false))
        .count();

    if ini_count == 0 {
        return Err(
            "No .ini files found in archive — this does not appear to be a valid 3DMigoto mod."
                .to_string(),
        );
    }

    // -- Step 6: Deep Matcher --
    set_job_status(db, job_id, "matching", None).await?;
    emit_status(app, job_id, "matching", None);

    // Load game_id for this job
    let game_id: Option<String> =
        sqlx::query_scalar!("SELECT game_id FROM import_jobs WHERE id = ?", job_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .flatten();

    let match_result = try_deep_match(app, &extract_dir, game_id.as_deref()).await;

    // Store match result
    if let Some(ref m) = match_result {
        sqlx::query!(
            r#"UPDATE import_jobs
               SET match_category = ?, match_object_id = ?, match_confidence = ?, match_reason = ?
               WHERE id = ?"#,
            m.category,
            m.object_id,
            m.confidence,
            m.reason,
            job_id
        )
        .execute(db)
        .await
        .ok();
    }

    let confidence = match_result.as_ref().map(|m| m.confidence).unwrap_or(0.0);
    if confidence < 0.70 {
        // Needs manual review
        set_job_status(db, job_id, "needs_review", None).await?;
        emit_status(
            app,
            job_id,
            "needs_review",
            match_result.as_ref().map(|m| {
                serde_json::json!({
                    "category": m.category,
                    "object_id": m.object_id,
                    "confidence": m.confidence,
                    "reason": m.reason,
                })
            }),
        );
        return Ok(());
    }

    // -- Step 7: Place --
    place_mod(db, app, job_id, &extract_dir, &match_result.unwrap()).await
}

// ── Placement ────────────────────────────────────────────────────────────────

async fn place_mod(
    db: &SqlitePool,
    app: &AppHandle,
    job_id: &str,
    extract_dir: &Path,
    match_result: &MatchResult,
) -> Result<(), String> {
    set_job_status(db, job_id, "placing", None).await?;

    let game_id: Option<String> =
        sqlx::query_scalar!("SELECT game_id FROM import_jobs WHERE id = ?", job_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .flatten();

    let game_id = game_id.ok_or("No game_id set on import job — cannot place mod")?;

    // Fetch game mods_path (games.path = the /Mods directory)
    let mods_path: String = sqlx::query_scalar!("SELECT path FROM games WHERE id = ?", game_id)
        .fetch_one(db)
        .await
        .map_err(|e| format!("Game not found: {e}"))?;

    let category = match_result.category.as_deref().unwrap_or("Other");
    let mod_name = extract_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("ImportedMod_{}", Uuid::new_v4().as_simple()));

    let dest = PathBuf::from(&mods_path).join(category).join(format!(
        "{} {}",
        crate::DISABLED_PREFIX,
        mod_name
    ));

    // Collision guard
    let dest = resolve_collision(dest);

    std::fs::rename(extract_dir, &dest)
        .or_else(|_| {
            // cross-drive fallback
            crate::services::fs_utils::file_utils::rename_cross_drive_fallback(extract_dir, &dest)
        })
        .map_err(|e| format!("Failed to place mod: {e}"))?;

    let dest_str = dest.to_string_lossy().to_string();
    sqlx::query!(
        "UPDATE import_jobs SET placed_path = ?, status = 'done', updated_at = datetime('now') WHERE id = ?",
        dest_str, job_id
    )
    .execute(db)
    .await
    .ok();

    // Mark the linked download as imported
    if let Some(dl_id) =
        sqlx::query_scalar!("SELECT download_id FROM import_jobs WHERE id = ?", job_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .flatten()
    {
        sqlx::query!(
            "UPDATE browser_downloads SET status = 'imported' WHERE id = ?",
            dl_id
        )
        .execute(db)
        .await
        .ok();
    }

    emit_status(
        app,
        job_id,
        "done",
        Some(serde_json::json!({ "placed_path": dest_str })),
    );
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn validate_extension(db: &SqlitePool, path: &Path) -> Result<(), String> {
    let allowed_raw: String =
        sqlx::query_scalar!("SELECT value FROM browser_settings WHERE key = 'allowed_extensions'")
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| ".zip,.7z,.rar,.tar,.gz".to_string());

    let allowed: Vec<&str> = allowed_raw.split(',').map(str::trim).collect();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();

    if allowed.contains(&ext.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "Extension '{ext}' is not in the allowed list: {allowed_raw}"
        ))
    }
}

fn hash_file(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("Cannot read file for hashing: {e}"))?;
    Ok(blake3::hash(&data).to_hex().to_string())
}

async fn stage_archive(app: &AppHandle, job_id: &str, archive: &Path) -> Result<PathBuf, String> {
    let staging_root = match app.path().app_data_dir() {
        Ok(d) => d.join("staging"),
        Err(_) => PathBuf::from("staging"),
    };
    let job_dir = staging_root.join(job_id);
    std::fs::create_dir_all(&job_dir).map_err(|e| format!("Cannot create staging dir: {e}"))?;

    let filename = archive.file_name().ok_or("Archive has no filename")?;
    let dest = job_dir.join(filename);
    std::fs::copy(archive, &dest).map_err(|e| format!("Cannot copy to staging: {e}"))?;
    Ok(dest)
}

struct MatchResult {
    category: Option<String>,
    object_id: Option<String>,
    confidence: f64,
    reason: Option<String>,
}

/// Attempt deep match. If the scanner service has a `quick_folder_match` function, call it.
/// Falls back to confidence 0.0 (needs_review) if the scanner is unavailable.
async fn try_deep_match(
    _app: &AppHandle,
    extract_dir: &Path,
    _game_id: Option<&str>,
) -> Option<MatchResult> {
    // Load all .ini files for basic heuristic
    let ini_count = walkdir::WalkDir::new(extract_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "ini").unwrap_or(false))
        .count();

    // Heuristic: if no .ini we already blocked above; return low confidence for manual review
    if ini_count == 0 {
        return None;
    }

    // TODO (Phase 8+): Wire to scanner::deep_matcher::analyze(extract_dir, game_id)
    // For now return 0.0 so all imports go through needs_review for user confirmation.
    Some(MatchResult {
        category: None,
        object_id: None,
        confidence: 0.0,
        reason: Some(
            "Automatic deep match not yet available — please confirm manually.".to_string(),
        ),
    })
}

fn resolve_collision(dest: PathBuf) -> PathBuf {
    if !dest.exists() {
        return dest;
    }
    let base = dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "mod".to_string());
    let parent = dest.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut n = 2u32;
    loop {
        let candidate = parent.join(format!("{base} ({n})"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

fn emit_status(app: &AppHandle, job_id: &str, status: &str, extra: Option<serde_json::Value>) {
    let mut payload = serde_json::json!({ "job_id": job_id, "status": status });
    if let Some(extra) = extra {
        if let serde_json::Value::Object(map) = extra {
            if let serde_json::Value::Object(ref mut p) = payload {
                p.extend(map);
            }
        }
    }
    let _ = app.emit("import:job-update", payload);
}

async fn set_job_status(
    db: &SqlitePool,
    job_id: &str,
    status: &str,
    error_msg: Option<&str>,
) -> Result<(), String> {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    sqlx::query!(
        "UPDATE import_jobs SET status = ?, error_msg = COALESCE(?, error_msg), updated_at = ? WHERE id = ?",
        status, error_msg, now, job_id
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB update import_job status failed: {e}"))?;
    Ok(())
}

// ── Queries ──────────────────────────────────────────────────────────────────

/// Return all active (non-canceled) import jobs ordered by most recent first.
pub async fn list_jobs(db: &SqlitePool) -> Result<Vec<ImportJobDto>, String> {
    let rows = sqlx::query!(
        r#"SELECT id, download_id, game_id, archive_path, status,
                  match_category, match_object_id, match_confidence, match_reason,
                  placed_path, error_msg, is_duplicate, created_at, updated_at
           FROM import_jobs
           WHERE status != 'canceled'
           ORDER BY created_at DESC
           LIMIT 100"#
    )
    .fetch_all(db)
    .await
    .map_err(|e| format!("DB list jobs failed: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| ImportJobDto {
            id: r.id.unwrap_or_default(),
            download_id: r.download_id,
            game_id: r.game_id,
            archive_path: r.archive_path,
            status: r.status,
            match_category: r.match_category,
            match_object_id: r.match_object_id,
            match_confidence: r.match_confidence,
            match_reason: r.match_reason,
            placed_path: r.placed_path,
            error_msg: r.error_msg,
            is_duplicate: r.is_duplicate != 0,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect())
}

/// Manual confirmation for a needs_review job.
/// Sets game_id, category, object_id, then resumes pipeline (place step).
pub async fn confirm_review(
    db: &SqlitePool,
    app: &AppHandle,
    job_id: &str,
    game_id: &str,
    category: &str,
    object_id: Option<&str>,
) -> Result<(), String> {
    sqlx::query!(
        "UPDATE import_jobs SET game_id = ?, match_category = ?, match_object_id = ?, status = 'placing', updated_at = datetime('now') WHERE id = ?",
        game_id, category, object_id, job_id
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB confirm_review failed: {e}"))?;

    // Resume placement
    let archive_opt: Option<String> =
        sqlx::query_scalar!("SELECT staging_path FROM import_jobs WHERE id = ?", job_id)
            .fetch_one(db)
            .await
            .map_err(|e| format!("Job not found: {e}"))?;

    let archive = archive_opt.ok_or_else(|| "No staging_path for job".to_string())?;

    let extract_dir = PathBuf::from(&archive).parent().unwrap().join("extracted");

    let match_result = MatchResult {
        category: Some(category.to_string()),
        object_id: object_id.map(|s| s.to_string()),
        confidence: 1.0,
        reason: Some("User confirmed".to_string()),
    };

    place_mod(db, app, job_id, &extract_dir, &match_result).await
}

/// Cancel a job and clean up its staging folder.
pub async fn cancel_job(db: &SqlitePool, job_id: &str) -> Result<(), String> {
    let staging: Option<String> =
        sqlx::query_scalar!("SELECT staging_path FROM import_jobs WHERE id = ?", job_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .flatten();

    if let Some(p) = staging {
        let staging_dir = PathBuf::from(&p)
            .parent()
            .map(|d| d.to_path_buf())
            .unwrap_or_default();
        if staging_dir.exists() {
            let _ = std::fs::remove_dir_all(&staging_dir);
        }
    }

    sqlx::query!(
        "UPDATE import_jobs SET status = 'canceled', updated_at = datetime('now') WHERE id = ?",
        job_id
    )
    .execute(db)
    .await
    .map_err(|e| format!("DB cancel_job failed: {e}"))?;
    Ok(())
}

/// Bulk-queue import jobs for a list of download IDs (from Download Manager multi-select).
pub async fn bulk_queue_imports(
    db: &SqlitePool,
    app: &AppHandle,
    download_ids: &[String],
    game_id: &str,
) -> Result<Vec<String>, String> {
    let mut job_ids = Vec::with_capacity(download_ids.len());

    for dl_id in download_ids {
        let row = sqlx::query!(
            "SELECT file_path, session_id FROM browser_downloads WHERE id = ? AND status = 'finished'",
            dl_id
        )
        .fetch_optional(db)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

        let Some(r) = row else { continue };
        let Some(file_path) = r.file_path else { continue };

        // Override game_id
        let job_id = Uuid::new_v4().to_string();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        sqlx::query!(
            r#"INSERT INTO import_jobs
               (id, download_id, game_id, archive_path, status, is_duplicate, created_at, updated_at)
               VALUES (?, ?, ?, ?, 'queued', 0, ?, ?)"#,
            job_id, dl_id, game_id, file_path, now, now
        )
        .execute(db)
        .await
        .map_err(|e| format!("DB insert failed: {e}"))?;

        // Spawn pipeline
        let db_c = db.clone();
        let app_c = app.clone();
        let jid = job_id.clone();
        let fp = file_path.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = run_pipeline(&db_c, &app_c, &jid, &fp).await {
                log::error!("Bulk import pipeline error job {jid}: {e}");
                let _ = set_job_status(&db_c, &jid, "failed", Some(&e)).await;
                let _ = app_c.emit(
                    "import:job-update",
                    serde_json::json!({
                        "job_id": jid, "status": "failed", "error": e
                    }),
                );
            }
        });

        job_ids.push(job_id);
    }

    Ok(job_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // Insert a dummy game for foreign keys
        // Since we don't enable PRAGMA foreign_keys, it might not be strictly needed, but let's be safe
        sqlx::query!(
            "INSERT INTO games (id, name, path) VALUES ('test_game', 'Test Game', 'C:\\')"
        )
        .execute(&pool)
        .await
        .ok();

        pool
    }

    #[tokio::test]
    async fn test_import_job_status_transitions() {
        let pool = setup_db().await;

        // Insert a dummy download record
        sqlx::query!("INSERT INTO browser_downloads (id, filename, started_at) VALUES ('dl-1', 'test.zip', '2025-01-01T00:00:00')")
            .execute(&pool).await.unwrap();

        let job_id = "job-1";
        sqlx::query!(
            "INSERT INTO import_jobs (id, download_id, archive_path, status, created_at, updated_at) 
             VALUES (?, 'dl-1', 'C:\\dummy.zip', 'queued', '2025-01-01T00:00:00', '2025-01-01T00:00:00')",
             job_id
        ).execute(&pool).await.unwrap();

        // 1. Transition to extracting
        set_job_status(&pool, job_id, "extracting", None)
            .await
            .unwrap();

        let status1 = sqlx::query_scalar!("SELECT status FROM import_jobs WHERE id = ?", job_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status1, "extracting");

        // 2. Transition to failed with error
        let err_msg = "CRC Mismatch";
        set_job_status(&pool, job_id, "failed", Some(err_msg))
            .await
            .unwrap();

        let rec2 = sqlx::query!(
            "SELECT status, error_msg FROM import_jobs WHERE id = ?",
            job_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(rec2.status, "failed");
        assert_eq!(rec2.error_msg.unwrap(), err_msg);
    }

    #[tokio::test]
    async fn test_import_job_dedup_hash() {
        let pool = setup_db().await;

        sqlx::query!("INSERT INTO browser_downloads (id, filename, started_at) VALUES ('dl-1', 'done.zip', '2025'), ('dl-2', 'new.zip', '2025')")
            .execute(&pool).await.unwrap();

        // Done job with hash
        sqlx::query!("INSERT INTO import_jobs (id, download_id, archive_path, status, archive_hash, created_at, updated_at) VALUES ('job-old', 'dl-1', 'C:\\1.zip', 'done', 'hash_abc', '2025', '2025')")
            .execute(&pool).await.unwrap();

        // New queued job
        sqlx::query!("INSERT INTO import_jobs (id, download_id, archive_path, status, created_at, updated_at) VALUES ('job-new', 'dl-2', 'C:\\2.zip', 'queued', '2025', '2025')")
            .execute(&pool).await.unwrap();

        let new_hash = "hash_abc";

        // Inline dedup logic from run_pipeline
        let existing = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM import_jobs WHERE archive_hash = ? AND status = 'done'",
            new_hash
        )
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

        assert!(existing > 0);

        // Mark as duplicate
        sqlx::query!(
            "UPDATE import_jobs SET is_duplicate = 1 WHERE id = ?",
            "job-new"
        )
        .execute(&pool)
        .await
        .unwrap();

        let is_dup =
            sqlx::query_scalar!("SELECT is_duplicate FROM import_jobs WHERE id = 'job-new'")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(is_dup, 1);
    }
}
