//! Filename sanitizing and download destination resolution.

use chrono::Utc;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Illegal Windows filename characters to strip.
const ILLEGAL_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
/// Maximum safe filename length (chars, excluding extension).
const MAX_FILENAME_LEN: usize = 120;

/// Sanitize a raw filename from a download URL so it is safe to store on Windows.
///
/// Rules:
/// - Strip illegal Windows chars (`< > : " / \ | ? *`).
/// - Trim leading/trailing whitespace.
/// - Clamp total length to 120 chars (preserving extension).
pub fn sanitize_filename(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !ILLEGAL_CHARS.contains(c)).collect();
    let cleaned = cleaned.trim().to_string();

    if cleaned.chars().count() <= MAX_FILENAME_LEN {
        return if cleaned.is_empty() {
            format!("download_{}", Utc::now().timestamp())
        } else {
            cleaned
        };
    }

    // Preserve extension if present. Truncation counts chars, never bytes —
    // a byte slice panics mid-character on the CJK names mods commonly have.
    let path = Path::new(&cleaned);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&cleaned);

    if ext.is_empty() {
        stem.chars().take(MAX_FILENAME_LEN).collect()
    } else {
        let max_stem = MAX_FILENAME_LEN.saturating_sub(ext.chars().count() + 1);
        let truncated: String = stem.chars().take(max_stem).collect();
        format!("{truncated}.{ext}")
    }
}

/// Resolve the collision-safe destination path for a download directly in the root directory.
pub fn compute_download_path(dir: &Path, _session_id: Option<&str>, filename: &str) -> PathBuf {
    compute_download_path_with(dir, _session_id, filename, |candidate| !candidate.exists())
}

/// Resolve a unique path with a caller-provided availability check. The download
/// scheduler uses this to include destinations already reserved in memory while
/// keeping the filesystem-only helper available to the rest of the browser.
pub fn compute_download_path_with(
    dir: &Path,
    _session_id: Option<&str>,
    filename: &str,
    is_available: impl Fn(&Path) -> bool,
) -> PathBuf {
    let safe_name = sanitize_filename(filename);

    // Generate unique path (avoid collision)
    let mut candidate = dir.join(&safe_name);
    if is_available(&candidate) {
        return candidate;
    }

    let path_obj = Path::new(&safe_name);
    let ext = path_obj.extension().and_then(|e| e.to_str()).unwrap_or("");
    let stem = path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&safe_name);

    let mut n = 2u32;
    loop {
        let suffixed = if ext.is_empty() {
            format!("{stem}_({n})")
        } else {
            format!("{stem}_({n}).{ext}")
        };
        candidate = dir.join(&suffixed);
        if is_available(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Get the `BrowserDownloadsRoot` path.
///
/// Priority: Mod Inbox of the active game -> `AppData/EMM2/BrowserDownloads` fallback.
pub async fn get_downloads_root(app: &AppHandle, db: &SqlitePool) -> PathBuf {
    // If there is an active game, route downloads directly to its Mod Inbox.
    if let Ok(Some(active_game)) =
        crate::modules::system::adapters::sqlite::settings::get_setting(db, "active_game_id").await
    {
        if let Ok(inbox) = crate::modules::ingestion::application::import_batch::ready_to_move::resolve_mod_inbox_root(
            app,
            db,
            &active_game,
            None,
        )
        .await
        {
            return inbox;
        }
    }

    // Fallback: AppData/EMM2/BrowserDownloads/
    match app.path().app_data_dir() {
        Ok(data_dir) => data_dir.join("BrowserDownloads"),
        Err(_) => PathBuf::from("BrowserDownloads"),
    }
}

/// Resolve a browser download directory for a known game. Download requests
/// retain this game id so an active-game switch cannot redirect a confirmation
/// or retry into another Mod Inbox.
pub async fn get_downloads_root_for_game(
    app: &AppHandle,
    db: &SqlitePool,
    game_id: &str,
) -> PathBuf {
    if let Ok(inbox) =
        crate::modules::ingestion::application::import_batch::ready_to_move::resolve_mod_inbox_root(
            app, db, game_id, None,
        )
        .await
    {
        return inbox;
    }

    match app.path().app_data_dir() {
        Ok(data_dir) => data_dir.join("BrowserDownloads").join(game_id),
        Err(_) => PathBuf::from("BrowserDownloads").join(game_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tempfile::tempdir;

    #[test]
    fn test_sanitize_filename_strips_illegal_chars() {
        let raw = "file<o:m?p\"i>l|e*d.zip";
        let clean = sanitize_filename(raw);
        assert_eq!(clean, "fileompiled.zip");

        let raw2 = "   leading_and_trailing  \n";
        assert_eq!(sanitize_filename(raw2), "leading_and_trailing");

        let raw3 = "C:\\Windows\\System32\\cmd.exe";
        assert_eq!(sanitize_filename(raw3), "CWindowsSystem32cmd.exe");
    }

    #[test]
    fn test_sanitize_filename_max_length() {
        let long_stem = "A".repeat(150);
        let ext = ".zip";
        let raw = format!("{long_stem}{ext}");
        let clean = sanitize_filename(&raw);

        assert_eq!(clean.len(), MAX_FILENAME_LEN);
        assert!(clean.ends_with(".zip"));

        // Exceeded by far
        let very_long = "X".repeat(300);
        let clean2 = sanitize_filename(&very_long);
        assert_eq!(clean2.len(), MAX_FILENAME_LEN);
        assert!(clean2.chars().all(|c| c == 'X'));
    }

    #[test]
    fn test_sanitize_filename_truncates_multibyte_names_on_char_boundaries() {
        // 150 three-byte chars: byte-indexed truncation would slice mid-char and panic.
        let cjk = "国".repeat(150);
        let clean = sanitize_filename(&format!("{cjk}.zip"));
        assert!(clean.ends_with(".zip"));
        assert_eq!(clean.chars().count(), MAX_FILENAME_LEN);

        let no_ext = sanitize_filename(&cjk);
        assert_eq!(no_ext.chars().count(), MAX_FILENAME_LEN);
    }

    #[test]
    fn test_compute_download_path_flat() {
        let temp = tempdir().unwrap();
        let filename = "mod_pack.zip";

        // session_id is ignored now, but test to ensure API stability
        let path = compute_download_path(temp.path(), Some("ignored"), filename);

        let expected = temp.path().join("mod_pack.zip");
        assert_eq!(path, expected);
    }

    #[test]
    fn test_compute_download_path_collision() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        let filename = "mod.zip";

        // 1st time
        let path1 = compute_download_path(root, None, filename);
        assert_eq!(path1.file_name().unwrap().to_str().unwrap(), "mod.zip");
        // touch it
        std::fs::File::create(&path1).unwrap();

        // 2nd time (collision)
        let path2 = compute_download_path(root, None, filename);
        assert_eq!(path2.file_name().unwrap().to_str().unwrap(), "mod_(2).zip");
        std::fs::File::create(&path2).unwrap();

        // 3rd time (collision again)
        let path3 = compute_download_path(root, None, filename);
        assert_eq!(path3.file_name().unwrap().to_str().unwrap(), "mod_(3).zip");

        // Test extensionless collision
        let path_noext1 = compute_download_path(root, None, "readme");
        assert_eq!(path_noext1.file_name().unwrap().to_str().unwrap(), "readme");
        std::fs::File::create(&path_noext1).unwrap();

        let path_noext2 = compute_download_path(root, None, "readme");
        assert_eq!(
            path_noext2.file_name().unwrap().to_str().unwrap(),
            "readme_(2)"
        );
    }

    #[test]
    fn reserved_download_destinations_get_distinct_names_before_writing() {
        let temp = tempdir().unwrap();
        let root = temp.path();
        let first = compute_download_path(root, None, "mod.zip");
        let reservations = HashSet::from([first]);

        let second = compute_download_path_with(root, None, "mod.zip", |candidate| {
            !candidate.exists() && !reservations.contains(candidate)
        });

        assert_eq!(
            second.file_name().and_then(|name| name.to_str()),
            Some("mod_(2).zip")
        );
    }
}
