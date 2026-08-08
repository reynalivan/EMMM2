use crate::common::sync::lock;
use crate::domain::errors::AppError;
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

/// How long an unused thumbnail is kept on disk before maintenance prunes it.
pub const THUMBNAIL_RETENTION_DAYS: u64 = 30;

/// TTL for L1 entries — skip mtime stat() calls within this window.
const ENTRY_TTL_SECS: u64 = 60;

/// Thumbnail dimensions. `DynamicImage::thumbnail` uses a fast box filter,
/// which is visually indistinguishable from Lanczos3 at card sizes.
const THUMB_SIZE: u32 = 256;

/// How many resolved folders the in-memory L1 keeps before evicting.
const L1_CAPACITY: NonZeroUsize = NonZeroUsize::new(500).unwrap();

const SECS_PER_DAY: u64 = 86_400;

static THUMBNAIL_CACHE: OnceLock<Mutex<ThumbnailCache>> = OnceLock::new();

/// A cached thumbnail entry with a TTL timestamp.
struct CachedEntry {
    webp_path: PathBuf,
    cached_at: Instant,
}

pub struct ThumbnailCache {
    /// Folder-path → CachedEntry (used by `resolve()`)
    folder_cache: LruCache<String, CachedEntry>,
    base_dir: Option<PathBuf>,
}

impl ThumbnailCache {
    fn new() -> Self {
        Self {
            folder_cache: LruCache::new(L1_CAPACITY),
            base_dir: None,
        }
    }

    fn get_instance() -> &'static Mutex<ThumbnailCache> {
        THUMBNAIL_CACHE.get_or_init(|| Mutex::new(Self::new()))
    }

    /// Points the cache at `app_data_dir`, dropping L1 if the location moved
    /// (its entries name files under the previous directory). Returns the
    /// resolved cache directory.
    fn set_base_dir(app_data_dir: &Path) -> PathBuf {
        let cache_dir = thumbnail_cache_dir(app_data_dir);
        let mut cache = lock(Self::get_instance());
        if cache.base_dir.as_ref() != Some(&cache_dir) {
            cache.folder_cache.clear();
        }
        cache.base_dir = Some(cache_dir.clone());
        cache_dir
    }

    pub fn init(app_data_dir: &Path) {
        let cache_dir = Self::set_base_dir(app_data_dir);
        if !cache_dir.exists() {
            let _ = fs::create_dir_all(&cache_dir);
        }
    }

    // ─── Primary API: Folder-keyed resolution (FolderGrid) ────────────

    /// Async entry-point for the folder grid thumbnail pipeline.
    ///
    /// 1. Check folder-keyed L1
    /// 2. Acquire semaphore permit (caps concurrent generations to 4)
    /// 3. Double-check L1 (another task may have resolved while waiting)
    /// 4. Cold-resolve in `spawn_blocking` (FS traversal + image processing)
    ///
    /// A folder that does not exist resolves to `None` via `find_thumbnail`.
    pub async fn resolve(_game_id: &str, folder_path: &str) -> Result<Option<String>, AppError> {
        // Fast path: folder-keyed L1 hit
        if let Some(hit) = Self::folder_l1_path(folder_path) {
            debug!("[Thumbnail] L1 hit for {}", folder_path);
            return Ok(Some(hit));
        }

        // Acquire permit — async, does NOT block the Tokio runtime
        let _permit = GEN_SEMAPHORE.acquire().await?;

        // Double-check after wait (dedup: another task may have resolved it)
        if let Some(hit) = Self::folder_l1_path(folder_path) {
            return Ok(Some(hit));
        }

        let path = PathBuf::from(folder_path);
        let folder_key = folder_path.to_string();
        tokio::task::spawn_blocking(move || Self::resolve_cold(&path, &folder_key)).await?
    }

    /// Returns the cached `.webp` path when the L1 entry is still usable.
    ///
    /// The filesystem check runs outside the lock on purpose: a `stat` held
    /// under the global mutex would serialize every concurrently mounting card.
    fn folder_l1_path(folder_path: &str) -> Option<String> {
        let fresh_path = {
            let mut cache = lock(Self::get_instance());
            let entry = cache.folder_cache.get(folder_path)?;
            (entry.cached_at.elapsed().as_secs() < ENTRY_TTL_SECS).then(|| entry.webp_path.clone())
        };

        match fresh_path {
            Some(path) if path.exists() => Some(path.to_string_lossy().to_string()),
            _ => {
                lock(Self::get_instance()).folder_cache.pop(folder_path);
                None
            }
        }
    }

    /// Cold path: find_thumbnail → generate/read disk cache → insert L1.
    fn resolve_cold(folder_path: &Path, folder_key: &str) -> Result<Option<String>, AppError> {
        use crate::services::scanner::core::thumbnail::find_thumbnail;

        let Some(original) = find_thumbnail(folder_path) else {
            debug!("[Thumbnail] No image found in: {:?}", folder_path);
            return Ok(None);
        };
        debug!("[Thumbnail] Found source image: {:?}", original);

        let webp_path = Self::generate(&original).map_err(|e| {
            warn!("[Thumbnail] Generate failed for {:?}: {}", original, e);
            e
        })?;

        {
            let mut cache = lock(Self::get_instance());
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
        let mut cache = lock(Self::get_instance());
        cache.folder_cache.pop(folder_path);
    }

    /// Invalidate the parent folder cache for a changed image path.
    pub fn invalidate(original_path: &Path) {
        if let Some(parent) = original_path.parent() {
            Self::invalidate_folder(&parent.to_string_lossy());
        }
    }

    // ─── Shared internals ─────────────────────────────────────────────

    /// Disk-cache filename stem for a source image. The prune pass must derive
    /// its keep-set the same way, so this is the only place the rule lives.
    fn cache_key(original_path: &str) -> String {
        blake3::hash(original_path.as_bytes()).to_string()
    }

    /// Generate (or retrieve from L2 disk cache) a WebP thumbnail.
    fn generate(original_path: &Path) -> Result<PathBuf, AppError> {
        let base_dir = {
            let cache = lock(Self::get_instance());
            cache
                .base_dir
                .clone()
                .ok_or_else(|| AppError::Internal("Cache not initialized".to_string()))?
        };

        let key = Self::cache_key(&original_path.to_string_lossy());
        let cached_path = base_dir.join(format!("{}.webp", key));

        // L2 disk hit — the cached file must be at least as new as its source.
        if let Ok(cache_meta) = fs::metadata(&cached_path) {
            if Self::is_cache_fresh(original_path, &cache_meta) {
                return Ok(cached_path);
            }
        }

        // Generate: Fast thumbnail resize
        let img = image::open(original_path)?;
        let resized = img.thumbnail(THUMB_SIZE, THUMB_SIZE);

        let mut bytes: Vec<u8> = Vec::new();
        resized.write_to(&mut Cursor::new(&mut bytes), ImageFormat::WebP)?;

        // The cache directory is created on demand rather than up front, so an
        // L2 hit never pays a mkdir syscall.
        if let Err(first_error) = fs::write(&cached_path, &bytes) {
            fs::create_dir_all(&base_dir)?;
            fs::write(&cached_path, &bytes).map_err(|second_error| {
                AppError::Io(format!(
                    "Failed to save thumbnail: {second_error} (first attempt: {first_error})"
                ))
            })?;
        }

        Ok(cached_path)
    }

    /// An unreadable source mtime is treated as stale, so the thumbnail is rebuilt.
    fn is_cache_fresh(original: &Path, cache_meta: &fs::Metadata) -> bool {
        let Ok(meta_orig) = fs::metadata(original) else {
            return false;
        };
        let mtime_orig = meta_orig.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let mtime_cache = cache_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        mtime_cache >= mtime_orig
    }

    /// Prune thumbnails for a specific app data directory.
    /// Returns number of deleted files.
    /// Prune thumbnails older than `max_age_days`, rooting the cache at
    /// `app_data_dir` first.
    ///
    /// Maintenance runs before anything has necessarily resolved a thumbnail,
    /// so it cannot rely on the base dir having been set by a prior lookup.
    pub fn clear_old_cache_for_app_data(
        app_data_dir: &Path,
        max_age_days: u64,
    ) -> Result<usize, AppError> {
        Self::set_base_dir(app_data_dir);
        Self::clear_old_cache(max_age_days)
    }

    /// Prune thumbnails older than `max_age_days`.
    /// Returns number of deleted files.
    pub fn clear_old_cache(max_age_days: u64) -> Result<usize, AppError> {
        // Copy the directory out before walking it: holding the cache lock across
        // a full directory scan would stall every concurrent L1 lookup.
        let base_dir = {
            let cache = lock(Self::get_instance());
            cache
                .base_dir
                .clone()
                .ok_or_else(|| AppError::Internal("Cache not initialized".to_string()))?
        };

        let cutoff = SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(max_age_days * SECS_PER_DAY))
            .ok_or_else(|| AppError::Internal("Failed to compute cutoff time".to_string()))?;

        // An unreadable timestamp is treated as "recent" so a stat failure never
        // deletes a live thumbnail.
        Self::remove_webp_entries(&base_dir, |entry| {
            entry
                .metadata()
                .ok()
                .and_then(|meta| meta.accessed().or_else(|_| meta.modified()).ok())
                .is_none_or(|accessed| accessed >= cutoff)
        })
    }

    /// Removes every `.webp` in `base_dir` that `keep` rejects, returning the
    /// number deleted. Non-files and other extensions are never touched.
    ///
    /// `keep` receives the `DirEntry` so callers can reuse the metadata the
    /// directory walk already carries instead of re-stat'ing each path.
    fn remove_webp_entries(
        base_dir: &Path,
        keep: impl Fn(&fs::DirEntry) -> bool,
    ) -> Result<usize, AppError> {
        let entries = match fs::read_dir(base_dir) {
            Ok(entries) => entries,
            // Nothing has been cached yet — nothing to prune.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(error.into()),
        };

        let mut deleted_count = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            let is_file = entry.file_type().is_ok_and(|file_type| file_type.is_file());
            if !is_file || path.extension().is_none_or(|ext| ext != "webp") {
                continue;
            }
            if !keep(&entry) && fs::remove_file(&path).is_ok() {
                deleted_count += 1;
            }
        }
        Ok(deleted_count)
    }
}

#[cfg(not(test))]
pub(crate) fn thumbnail_cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cache").join("thumbnails")
}

#[cfg(test)]
pub(crate) fn thumbnail_cache_dir(app_data_dir: &Path) -> PathBuf {
    let key = ThumbnailCache::cache_key(&app_data_dir.to_string_lossy());
    std::env::temp_dir()
        .join("emmm-thumbnail-cache-tests")
        .join(key)
        .join("cache")
        .join("thumbnails")
}

#[cfg(test)]
#[path = "tests/thumbnail_cache_tests.rs"]
mod tests;
