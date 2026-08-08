//! The queued → extracted → matched → placed run for a single import job.

use crate::domain::errors::BrowserError;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::repo::browser_repo;

use super::jobs::{emit_status, set_job_status};
use super::matching::try_deep_match;
use super::placement::place_mod;

pub(super) async fn run_pipeline(
    db: &SqlitePool,
    app: &AppHandle,
    job_id: &str,
    archive_path: &str,
) -> Result<(), BrowserError> {
    let archive = PathBuf::from(archive_path);

    // -- Step 1: Validate extension is allowed --
    validate_extension(db, &archive).await?;

    // -- Step 2: Hash (BLAKE3) + dedup check --
    set_job_status(db, job_id, "extracting", None).await?;
    emit_status(app, job_id, "extracting", None);

    let hash = hash_file(&archive)?;

    // Check dedup
    let existing = browser_repo::count_done_with_hash(db, &hash)
        .await
        .unwrap_or(0);

    if existing > 0 {
        browser_repo::mark_duplicate(db, job_id).await.ok();
        // Status must land before the event: this path is terminal, so a refetch
        // that beat the write would leave the job stuck on "extracting".
        set_job_status(db, job_id, "needs_review", None).await?;
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
        return Ok(());
    }

    // Store hash
    browser_repo::set_archive_hash(db, job_id, &hash).await.ok();

    // -- Step 3: Stage (copy to staging dir) --
    let staging_path = stage_archive(app, job_id, &archive).await?;
    let staging_path_str = staging_path.to_string_lossy().to_string();
    browser_repo::set_staging_path(db, job_id, &staging_path_str)
        .await
        .ok();

    // -- Step 4: Extract --
    let extract_dir = staging_path.parent().unwrap().join("extracted");
    std::fs::create_dir_all(&extract_dir)?;

    // No password UI yet; extend `ExtractOptions` when there is one.
    let extraction = crate::services::mods::archive::extract_archive(
        &staging_path,
        &extract_dir,
        crate::services::mods::archive::ExtractOptions {
            overwrite: true,
            ..Default::default()
        },
    )
    .map_err(|error| BrowserError::Import(error.to_string()))?;

    // `extract_archive` already split the archive into mod roots and named each
    // one. Using `extract_dir` as the mod would name every import "extracted".
    let mod_roots = extracted_mod_roots(&extraction, &extract_dir);

    // -- Step 5: Validate (check for at least one .ini file) --
    let ini_count = count_ini_files(&extract_dir);

    if ini_count == 0 {
        return Err(BrowserError::Import(
            "no .ini files found in archive — this does not appear to be a valid 3DMigoto mod"
                .to_string(),
        ));
    }

    // -- Step 6: Deep Match Scanner --
    set_job_status(db, job_id, "matching", None).await?;
    emit_status(app, job_id, "matching", None);

    // Load game_id for this job
    let game_id: Option<String> = browser_repo::get_job_game_id(db, job_id)
        .await
        .ok()
        .flatten();

    // Match against the primary mod root, so the candidate carries the mod's
    // real name rather than the staging directory's.
    let match_result = try_deep_match(app, &mod_roots[0], game_id.as_deref()).await;

    // Store match result
    if let Some(ref m) = match_result {
        let _ = browser_repo::set_match_result(db, job_id, m).await;
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
                    "entry_key": m.entry_key,
                    "alias_name": m.alias_name,
                    "confidence": m.confidence,
                    "reason": m.reason,
                })
            }),
        );
        return Ok(());
    }

    // -- Step 7: Place --
    place_mod(db, app, job_id, &mod_roots, &match_result.unwrap(), None).await
}

/// The mod root folders an extraction produced, as absolute paths.
///
/// Falls back to the extract directory itself when the extractor reported no
/// roots, so a malformed archive still places something rather than vanishing.
fn extracted_mod_roots(
    extraction: &crate::services::mods::archive::ExtractionResult,
    extract_dir: &Path,
) -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = extraction
        .dest_paths
        .iter()
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .collect();

    if roots.is_empty() {
        return mod_roots_on_disk(extract_dir);
    }
    roots
}

/// Recover the mod roots of an already-extracted job from disk.
///
/// `extract_archive` moves each root into the extract directory, so its child
/// directories are the roots. Used by the review/confirm path, which runs long
/// after the `ExtractionResult` is gone.
pub(super) fn mod_roots_on_disk(extract_dir: &Path) -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = std::fs::read_dir(extract_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();

    if roots.is_empty() {
        return vec![extract_dir.to_path_buf()];
    }
    roots
}

/// Number of `.ini` files anywhere under `dir` — the 3DMigoto sanity signal.
pub(super) fn count_ini_files(dir: &Path) -> usize {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("ini"))
        })
        .count()
}

async fn validate_extension(db: &SqlitePool, path: &Path) -> Result<(), BrowserError> {
    let allowed_raw: String = browser_repo::get_setting(db, "allowed_extensions")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| ".zip,.7z,.rar,.tar,.gz".to_string());

    // Case-insensitive on both sides: browsers regularly hand out `FILE.ZIP`,
    // and the allowed list is user-edited text.
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_ascii_lowercase()))
        .unwrap_or_default();

    if allowed_raw
        .split(',')
        .any(|allowed| allowed.trim().eq_ignore_ascii_case(&ext))
    {
        Ok(())
    } else {
        Err(BrowserError::Import(format!(
            "extension '{ext}' is not in the allowed list: {allowed_raw}"
        )))
    }
}

fn hash_file(path: &Path) -> Result<String, BrowserError> {
    // Streaming hash: archives can be multi-GB, so reading the whole file
    // into memory for one hash is an avoidable spike.
    let file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(file)?;
    Ok(hasher.finalize().to_hex().to_string())
}

async fn stage_archive(
    app: &AppHandle,
    job_id: &str,
    archive: &Path,
) -> Result<PathBuf, BrowserError> {
    let staging_root = match app.path().app_data_dir() {
        Ok(d) => d.join("staging"),
        Err(_) => PathBuf::from("staging"),
    };
    let job_dir = staging_root.join(job_id);
    std::fs::create_dir_all(&job_dir)?;

    let filename = archive.file_name().ok_or_else(|| {
        BrowserError::Import(format!("archive has no filename: {}", archive.display()))
    })?;
    let dest = job_dir.join(filename);
    std::fs::copy(archive, &dest)?;
    Ok(dest)
}
