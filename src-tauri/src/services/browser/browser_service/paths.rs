//! Filename sanitizing and download destination resolution.

use chrono::Utc;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::repo::browser_repo;

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

    if cleaned.len() <= MAX_FILENAME_LEN {
        return if cleaned.is_empty() {
            format!("download_{}", Utc::now().timestamp())
        } else {
            cleaned
        };
    }

    // Preserve extension if present
    let path = Path::new(&cleaned);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&cleaned);

    if ext.is_empty() {
        stem[..MAX_FILENAME_LEN].to_string()
    } else {
        let max_stem = MAX_FILENAME_LEN.saturating_sub(ext.len() + 1);
        format!("{}.{}", &stem[..max_stem.min(stem.len())], ext)
    }
}

/// Resolve the collision-safe destination path for a download.
///
/// Layout:
/// - `root/YYYY-MM/<session_id>/<safe_filename>.<ext>` (session-linked)
/// - `root/YYYY-MM/adhoc/<timestamp>_<safe_filename>.<ext>` (no session)
pub fn compute_download_path(root: &Path, session_id: Option<&str>, filename: &str) -> PathBuf {
    let now = Utc::now();
    let month_dir = now.format("%Y-%m").to_string();
    let safe_name = sanitize_filename(filename);

    let dir = match session_id {
        Some(sid) => root.join(&month_dir).join(sid),
        None => root.join(&month_dir).join("adhoc"),
    };

    // Generate unique path (avoid collision)
    let mut candidate = dir.join(&safe_name);
    if !candidate.exists() {
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
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// Get the `BrowserDownloadsRoot` path.
///
/// Priority: `browser_settings.downloads_root` (if non-empty + writable) → default.
pub async fn get_downloads_root(app: &AppHandle, db: &SqlitePool) -> PathBuf {
    let override_path: Option<String> = browser_repo::get_setting(db, "downloads_root")
        .await
        .ok()
        .flatten();

    if let Some(p) = override_path {
        if !p.is_empty() {
            let path = PathBuf::from(&p);
            if path.exists() || std::fs::create_dir_all(&path).is_ok() {
                return path;
            }
        }
    }

    // Default: AppData/EMM2/BrowserDownloads/
    match app.path().app_data_dir() {
        Ok(data_dir) => data_dir.join("BrowserDownloads"),
        Err(_) => PathBuf::from("BrowserDownloads"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_compute_download_path_with_session() {
        let temp = tempdir().unwrap();
        let session_id = "1234-5678-uuid";
        let filename = "mod_pack.zip";

        let path = compute_download_path(temp.path(), Some(session_id), filename);

        let month_dir = Utc::now().format("%Y-%m").to_string();
        let expected = temp
            .path()
            .join(&month_dir)
            .join(session_id)
            .join("mod_pack.zip");
        assert_eq!(path, expected);
    }

    #[test]
    fn test_compute_download_path_adhoc() {
        let temp = tempdir().unwrap();
        let filename = "loose_mod.rar";

        let path = compute_download_path(temp.path(), None, filename);

        let month_dir = Utc::now().format("%Y-%m").to_string();
        let expected = temp
            .path()
            .join(&month_dir)
            .join("adhoc")
            .join("loose_mod.rar");
        assert_eq!(path, expected);
    }

    #[test]
    fn test_compute_download_path_collision() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        // Make the structure
        let month_dir = Utc::now().format("%Y-%m").to_string();
        let adhoc_dir = root.join(&month_dir).join("adhoc");
        std::fs::create_dir_all(&adhoc_dir).unwrap();

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
}
