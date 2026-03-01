use std::fs;
use std::io::Cursor;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime};

use image::ImageFormat;
use log::{debug, warn};
use lru::LruCache;
use tokio::sync::Semaphore;

/// Max concurrent thumbnail generations (image open + resize + encode).
/// Prevents CPU/IO saturation when the virtualizer mounts many cards at once.
static GEN_SEMAPHORE: Semaphore = Semaphore::const_new(4);

/// TTL for L1 entries — skip mtime stat() calls within this window.
const ENTRY_TTL_SECS: u64 = 60;

/// Thumbnail dimensions. CatmullRom 256×256 is ~50 % faster than Lanczos3 400×400
/// while remaining visually identical at card sizes.
const THUMB_SIZE: u32 = 256;

static THUMBNAIL_CACHE: OnceLock<Mutex<ThumbnailCache>> = OnceLock::new();

/// A cached thumbnail entry with a TTL timestamp.
struct CachedEntry {
    webp_path: PathBuf,
    cached_at: Instant,
}

pub struct ThumbnailCache {
    /// Folder-path → CachedEntry (used by `resolve()`)
    folder_cache: LruCache<String, CachedEntry>,
    /// Original-image-path → CachedEntry (used by legacy `get_thumbnail()`)
    image_cache: LruCache<String, CachedEntry>,
    base_dir: Option<PathBuf>,
}

impl ThumbnailCache {
    fn new() -> Self {
        Self {
            folder_cache: LruCache::new(NonZeroUsize::new(500).unwrap()),
            image_cache: LruCache::new(NonZeroUsize::new(500).unwrap()),
            base_dir: None,
        }
    }

    fn get_instance() -> &'static Mutex<ThumbnailCache> {
        THUMBNAIL_CACHE.get_or_init(|| Mutex::new(Self::new()))
    }

    pub fn init(app_data_dir: &Path) {
        let mut cache = Self::get_instance().lock().unwrap();
        let cache_dir = app_data_dir.join("cache").join("thumbnails");
        if !cache_dir.exists() {
            let _ = fs::create_dir_all(&cache_dir);
        }
        cache.base_dir = Some(cache_dir);
    }

    // ─── Primary API: Folder-keyed resolution (FolderGrid) ────────────

    /// Async entry-point for the folder grid thumbnail pipeline.
    ///
    /// 1. Check folder-keyed L1 (fast, no I/O)
    /// 2. Acquire semaphore permit (caps concurrent generations to 4)
    /// 3. Double-check L1 (another task may have resolved while waiting)
    /// 4. Cold-resolve in `spawn_blocking` (FS traversal + image processing)
    pub async fn resolve(folder_path: &str) -> Result<Option<String>, String> {
        let path = PathBuf::from(folder_path);
        if !path.is_dir() {
            debug!("[Thumbnail] Not a directory, skipping: {}", folder_path);
            return Ok(None);
        }

        // Fast path: folder-keyed L1 hit
        if let Some(hit) = Self::check_folder_l1(folder_path) {
            debug!("[Thumbnail] L1 hit for {}", folder_path);
            return Ok(Some(hit));
        }

        // Acquire permit — async, does NOT block the Tokio runtime
        let _permit = GEN_SEMAPHORE
            .acquire()
            .await
            .map_err(|e| format!("Semaphore closed: {}", e))?;

        // Double-check after wait (dedup: another task may have resolved it)
        if let Some(hit) = Self::check_folder_l1(folder_path) {
            return Ok(Some(hit));
        }

        let folder_key = folder_path.to_string();
        tokio::task::spawn_blocking(move || Self::resolve_cold(&path, &folder_key))
            .await
            .map_err(|e| format!("Thumbnail task failed: {}", e))?
    }

    /// Check folder-keyed L1. Returns the WebP path string if valid.
    fn check_folder_l1(folder_path: &str) -> Option<String> {
        let mut cache = Self::get_instance().lock().unwrap();
        if let Some(entry) = cache.folder_cache.get(folder_path) {
            if entry.cached_at.elapsed().as_secs() < ENTRY_TTL_SECS && entry.webp_path.exists() {
                return Some(entry.webp_path.to_string_lossy().to_string());
            }
            cache.folder_cache.pop(folder_path);
        }
        None
    }

    /// Cold path: find_thumbnail → generate/read disk cache → insert L1.
    fn resolve_cold(folder_path: &Path, folder_key: &str) -> Result<Option<String>, String> {
        use crate::services::scanner::core::thumbnail::find_thumbnail;

        let original = match find_thumbnail(folder_path) {
            Some(p) => {
                debug!("[Thumbnail] Found source image: {:?}", p);
                p
            }
            None => {
                debug!("[Thumbnail] No image found in: {:?}", folder_path);
                return Ok(None);
            }
        };

        let webp_path = Self::generate(&original).map_err(|e| {
            warn!("[Thumbnail] Generate failed for {:?}: {}", original, e);
            e
        })?;

        debug!("[Thumbnail] Resolved {:?} → {:?}", folder_path, webp_path);

        // Insert into folder-keyed L1
        {
            let mut cache = Self::get_instance().lock().unwrap();
            cache.folder_cache.put(
                folder_key.to_string(),
                CachedEntry {
                    webp_path: webp_path.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        Ok(Some(webp_path.to_string_lossy().to_string()))
    }

    /// Invalidate folder-keyed L1 entry.
    pub fn invalidate_folder(folder_path: &str) {
        let mut cache = Self::get_instance().lock().unwrap();
        cache.folder_cache.pop(folder_path);
    }

    // ─── Legacy API: Image-path-keyed (mod_cmds, preview_image, metadata) ─

    /// Invalidate by original image path (backward compat).
    /// Also clears the parent folder from `folder_cache`.
    pub fn invalidate(original_path: &Path) {
        let mut cache = Self::get_instance().lock().unwrap();
        let key = original_path.to_string_lossy().to_string();
        cache.image_cache.pop(&key);
        // Also clear parent folder so the grid picks up changes
        if let Some(parent) = original_path.parent() {
            cache
                .folder_cache
                .pop(&parent.to_string_lossy().to_string());
        }
    }

    /// Get/generate thumbnail by original image path (backward compat).
    pub fn get_thumbnail(original_path: &Path) -> Result<PathBuf, String> {
        let key = original_path.to_string_lossy().to_string();

        // Check image-keyed L1 with TTL
        {
            let mut cache = Self::get_instance().lock().unwrap();
            if let Some(entry) = cache.image_cache.get(&key) {
                if entry.cached_at.elapsed().as_secs() < ENTRY_TTL_SECS && entry.webp_path.exists()
                {
                    return Ok(entry.webp_path.clone());
                }
                cache.image_cache.pop(&key);
            }
        }

        let webp_path = Self::generate(original_path)?;

        // Insert into image-keyed L1
        {
            let mut cache = Self::get_instance().lock().unwrap();
            cache.image_cache.put(
                key,
                CachedEntry {
                    webp_path: webp_path.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        Ok(webp_path)
    }

    // ─── Shared internals ─────────────────────────────────────────────

    /// Generate (or retrieve from L2 disk cache) a WebP thumbnail.
    fn generate(original_path: &Path) -> Result<PathBuf, String> {
        let original_str = original_path.to_string_lossy().to_string();

        let base_dir = {
            let cache = Self::get_instance().lock().unwrap();
            cache.base_dir.clone().ok_or("Cache not initialized")?
        };

        let hash = blake3::hash(original_str.as_bytes()).to_string();
        let cached_path = base_dir.join(format!("{}.webp", hash));

        // L2 disk hit — validate mtime
        if cached_path.exists() {
            if let Ok(true) = Self::validate_mtime(original_path, &cached_path) {
                return Ok(cached_path);
            }
        }

        // Generate: Fast thumbnail resize
        let img = image::open(original_path).map_err(|e| format!("Failed to open image: {}", e))?;
        let resized = img.thumbnail(THUMB_SIZE, THUMB_SIZE);

        let mut bytes: Vec<u8> = Vec::new();
        resized
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::WebP)
            .map_err(|e| format!("Failed to encode WebP: {}", e))?;

        fs::write(&cached_path, &bytes).map_err(|e| format!("Failed to save thumbnail: {}", e))?;

        Ok(cached_path)
    }

    fn validate_mtime(original: &Path, cached: &Path) -> Result<bool, String> {
        let meta_orig = fs::metadata(original).map_err(|e| e.to_string())?;
        let meta_cache = fs::metadata(cached).map_err(|e| e.to_string())?;
        let mtime_orig = meta_orig.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let mtime_cache = meta_cache.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        Ok(mtime_cache >= mtime_orig)
    }

    /// Prune thumbnails that don't correspond to any path in `valid_paths`.
    /// Returns number of deleted files.
    pub fn prune_orphans(valid_paths: &[String]) -> Result<usize, String> {
        let cache = Self::get_instance().lock().unwrap();
        let base_dir = cache.base_dir.as_ref().ok_or("Cache not initialized")?;

        let mut keep_hashes = std::collections::HashSet::new();
        for path in valid_paths {
            let hash = blake3::hash(path.as_bytes()).to_string();
            keep_hashes.insert(hash);
        }

        let mut deleted_count = 0;
        let entries = fs::read_dir(base_dir).map_err(|e| e.to_string())?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Check if it's a webp file
            if path.extension().is_some_and(|e| e == "webp") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // It's a hash.webp. If hash not in keep_hashes, delete.
                    if !keep_hashes.contains(stem) && fs::remove_file(&path).is_ok() {
                        deleted_count += 1;
                    }
                }
            }
        }
        Ok(deleted_count)
    }

    /// Prune thumbnails older than `max_age_days`.
    /// Returns number of deleted files.
    pub fn clear_old_cache(max_age_days: u64) -> Result<usize, String> {
        let cache = Self::get_instance().lock().unwrap();
        let base_dir = cache.base_dir.as_ref().ok_or("Cache not initialized")?;

        let cutoff = SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(max_age_days * 86400))
            .ok_or_else(|| "Failed to compute cutoff time".to_string())?;

        let mut deleted_count = 0;
        let entries = fs::read_dir(base_dir).map_err(|e| e.to_string())?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if path.extension().is_some_and(|e| e == "webp") {
                if let Ok(meta) = fs::metadata(&path) {
                    if let Ok(accessed) = meta.accessed().or_else(|_| meta.modified()) {
                        if accessed < cutoff && fs::remove_file(&path).is_ok() {
                            deleted_count += 1;
                        }
                    }
                }
            }
        }
        Ok(deleted_count)
    }
}

#[cfg(test)]
#[path = "tests/thumbnail_cache_tests.rs"]
mod tests;
