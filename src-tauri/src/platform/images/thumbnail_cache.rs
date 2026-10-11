use crate::shared::errors::AppError;
use crate::shared::sync::lock;
use std::borrow::Borrow;
use std::fs;
use std::io::Cursor;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Instant, SystemTime};

use image::{ImageFormat, ImageReader};
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
const MAX_RESOLVE_ATTEMPTS: usize = 3;

/// How many resolved folders the in-memory L1 keeps before evicting.
const L1_CAPACITY: NonZeroUsize = NonZeroUsize::new(500).unwrap();

const SECS_PER_DAY: u64 = 86_400;

static THUMBNAIL_CACHE: OnceLock<Mutex<ThumbnailCache>> = OnceLock::new();

/// A cached thumbnail entry with a TTL timestamp.
struct CachedEntry {
    webp_path: PathBuf,
    cached_at: Instant,
}

struct ResolutionScope {
    folder_key: String,
    location: PathBuf,
}

// Only admitted cold attempts register; the existing semaphore bounds this list.
// Weak tickets plus the attempt's Drop guard cannot retain completed folder state.
struct ResolutionAttempt<'a> {
    cache_state: &'a Mutex<ThumbnailCache>,
    scope: Arc<ResolutionScope>,
    generation: u64,
}

impl<'a> ResolutionAttempt<'a> {
    fn begin(cache_state: &'a Mutex<ThumbnailCache>, folder: &Path, folder_key: &str) -> Self {
        let scope = Arc::new(ResolutionScope {
            folder_key: folder_key.to_string(),
            location: ThumbnailCache::location_key(folder),
        });
        let mut cache = lock(cache_state);
        cache.pending_resolutions.push(Arc::downgrade(&scope));
        Self {
            cache_state,
            scope,
            generation: cache.invalidation_generation,
        }
    }

    fn is_current(&self, cache: &ThumbnailCache) -> bool {
        let ticket = Arc::downgrade(&self.scope);
        cache.invalidation_generation == self.generation
            && cache
                .pending_resolutions
                .iter()
                .any(|pending| pending.ptr_eq(&ticket))
    }
}

impl Drop for ResolutionAttempt<'_> {
    fn drop(&mut self) {
        let ticket = Arc::downgrade(&self.scope);
        lock(self.cache_state)
            .pending_resolutions
            .retain(|pending| !pending.ptr_eq(&ticket));
    }
}

pub struct ThumbnailCache {
    /// Folder-path → CachedEntry (used by `resolve()`)
    folder_cache: LruCache<String, CachedEntry>,
    base_dir: Option<PathBuf>,
    invalidation_generation: u64,
    pending_resolutions: Vec<Weak<ResolutionScope>>,
}

impl ThumbnailCache {
    fn new() -> Self {
        Self {
            folder_cache: LruCache::new(L1_CAPACITY),
            base_dir: None,
            invalidation_generation: 0,
            pending_resolutions: Vec::new(),
        }
    }

    fn get_instance() -> &'static Mutex<ThumbnailCache> {
        THUMBNAIL_CACHE.get_or_init(|| Mutex::new(Self::new()))
    }

    /// Points the cache at `app_data_dir`, dropping L1 if the location moved
    /// (its entries name files under the previous directory). Returns the
    /// resolved cache directory.
    fn set_base_dir(app_data_dir: &Path) -> PathBuf {
        Self::set_base_dir_with(Self::get_instance(), app_data_dir)
    }

    fn set_base_dir_with(cache_state: &Mutex<Self>, app_data_dir: &Path) -> PathBuf {
        let cache_dir = thumbnail_cache_dir(app_data_dir);
        let mut cache = lock(cache_state);
        if cache.base_dir.as_ref() != Some(&cache_dir) {
            cache.folder_cache.clear();
            cache.invalidation_generation = cache.invalidation_generation.wrapping_add(1);
        }
        cache.base_dir = Some(cache_dir.clone());
        cache_dir
    }

    pub fn init(app_data_dir: &Path) {
        Self::init_with(Self::get_instance(), app_data_dir);
    }

    fn init_with(cache_state: &Mutex<Self>, app_data_dir: &Path) {
        let cache_dir = Self::set_base_dir_with(cache_state, app_data_dir);
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
        Self::resolve_with_cache(Self::get_instance(), folder_path).await
    }

    async fn resolve_with_cache<C>(
        cache_state: C,
        folder_path: &str,
    ) -> Result<Option<String>, AppError>
    where
        C: Borrow<Mutex<Self>> + Send + Sync + 'static,
    {
        let folder_key = Self::identity_key(Path::new(folder_path));

        // Fast path: folder-keyed L1 hit
        if let Some(hit) = Self::folder_l1_path(cache_state.borrow(), &folder_key) {
            debug!("[Thumbnail] L1 hit for {}", folder_path);
            return Ok(Some(hit));
        }

        // Acquire permit — async, does NOT block the Tokio runtime
        let permit = GEN_SEMAPHORE.acquire().await?;

        // Double-check after wait (dedup: another task may have resolved it)
        if let Some(hit) = Self::folder_l1_path(cache_state.borrow(), &folder_key) {
            return Ok(Some(hit));
        }

        let path = PathBuf::from(folder_path);
        tokio::task::spawn_blocking(move || {
            // Cancellation of the async waiter must not admit another decoder
            // while this blocking generation is still running.
            let _permit = permit;
            Self::resolve_cold(cache_state.borrow(), &path, &folder_key)
        })
        .await?
    }

    /// Native identity survives a toggle but distinguishes coexisting folders.
    fn identity_key(path: &Path) -> String {
        crate::platform::fs::file_utils::filesystem_identity(path)
            .map(|identity| format!("physical:{identity}"))
            .unwrap_or_else(|| crate::shared::path_key::exact_location_key_for_path(path))
    }

    fn location_key(path: &Path) -> PathBuf {
        // Watcher removal paths may be gone; a surviving junction ancestor still
        // identifies the physical subtree of an in-flight thumbnail.
        let canonical = path
            .ancestors()
            .find_map(|ancestor| {
                let canonical = fs::canonicalize(ancestor).ok()?;
                Some(canonical.join(path.strip_prefix(ancestor).ok()?))
            })
            .unwrap_or_else(|| path.to_path_buf());
        PathBuf::from(crate::shared::path_key::exact_location_key_for_path(
            &canonical,
        ))
    }

    /// Returns the cached `.webp` path when the L1 entry is still usable.
    /// `folder_key` is an identity key from [`Self::identity_key`].
    ///
    /// The filesystem check runs outside the lock on purpose: a `stat` held
    /// under the global mutex would serialize every concurrently mounting card.
    fn folder_l1_path(cache_state: &Mutex<Self>, folder_key: &str) -> Option<String> {
        let fresh_path = {
            let mut cache = lock(cache_state);
            let entry = cache.folder_cache.get(folder_key)?;
            (entry.cached_at.elapsed().as_secs() < ENTRY_TTL_SECS).then(|| entry.webp_path.clone())
        };

        match fresh_path {
            Some(path) if path.exists() => Some(path.to_string_lossy().to_string()),
            _ => {
                lock(cache_state).folder_cache.pop(folder_key);
                None
            }
        }
    }

    /// Cold path: find_thumbnail → generate/read disk cache → insert L1.
    fn resolve_cold(
        cache_state: &Mutex<Self>,
        folder_path: &Path,
        folder_key: &str,
    ) -> Result<Option<String>, AppError> {
        Self::resolve_cold_with(cache_state, folder_path, folder_key, |source| {
            Self::generate(cache_state, source)
        })
    }

    fn resolve_cold_with(
        cache_state: &Mutex<Self>,
        folder_path: &Path,
        folder_key: &str,
        generate: impl Fn(&Path) -> Result<PathBuf, AppError>,
    ) -> Result<Option<String>, AppError> {
        use crate::modules::workspace::application::scanner::core::thumbnail::find_thumbnail;
        for _ in 0..MAX_RESOLVE_ATTEMPTS {
            let attempt = ResolutionAttempt::begin(cache_state, folder_path, folder_key);
            let Some(original) = find_thumbnail(folder_path) else {
                return Ok(None);
            };
            let webp_path = generate(&original).map_err(|e| {
                warn!("[Thumbnail] Generate failed for {:?}: {}", original, e);
                e
            })?;
            let mut cache = lock(cache_state);
            if !attempt.is_current(&cache) {
                continue;
            }
            if Self::identity_key(folder_path) != folder_key {
                return Err(AppError::Io(
                    "Preview folder changed during thumbnail generation".to_string(),
                ));
            }
            cache.folder_cache.put(
                folder_key.to_string(),
                CachedEntry {
                    webp_path: webp_path.clone(),
                    cached_at: Instant::now(),
                },
            );
            return Ok(Some(webp_path.to_string_lossy().to_string()));
        }
        Err(AppError::Io(
            "Preview visibility changed repeatedly; retry thumbnail resolution".to_string(),
        ))
    }

    /// Invalidate folder-keyed L1 entry. Accepts a raw path in any prefix or
    /// case state — the identity key makes them all hit the same entry.
    pub fn invalidate_folder(folder_path: &str) {
        Self::invalidate_folder_with(Self::get_instance(), folder_path);
    }

    fn invalidate_folder_with(cache_state: &Mutex<Self>, folder_path: &str) {
        let key = Self::identity_key(Path::new(folder_path));
        let location = Self::location_key(Path::new(folder_path));
        let mut cache = lock(cache_state);
        cache.pending_resolutions.retain(|pending| {
            pending.upgrade().is_some_and(|scope| {
                scope.folder_key != key
                    && !scope.location.starts_with(&location)
                    && !location.starts_with(&scope.location)
            })
        });
        cache.folder_cache.pop(&key);
    }

    /// A completed full disk scan has no precise image paths for offline
    /// changes. Drop only L1 so the next explicitly-invalidated UI query can
    /// revalidate its existing L2 thumbnail against the source image.
    pub fn clear_memory() {
        Self::clear_memory_with(Self::get_instance());
    }

    fn clear_memory_with(cache_state: &Mutex<Self>) {
        let mut cache = lock(cache_state);
        cache.invalidation_generation = cache.invalidation_generation.wrapping_add(1);
        cache.folder_cache.clear();
    }

    /// Invalidate the parent folder cache for a changed image path.
    pub fn invalidate(original_path: &Path) {
        if let Some(parent) = original_path.parent() {
            Self::invalidate_folder(&parent.to_string_lossy());
        }

        let base_dir = lock(Self::get_instance()).base_dir.clone();
        let Some(base_dir) = base_dir else {
            return;
        };
        Self::invalidate_l2_at(&base_dir, original_path);
    }

    fn invalidate_l2_at(base_dir: &Path, original_path: &Path) {
        let cached_path = base_dir.join(format!(
            "{}.webp",
            Self::cache_key(&Self::identity_key(original_path))
        ));
        if let Err(error) = fs::remove_file(&cached_path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                warn!(
                    "Failed to remove stale thumbnail cache {}: {error}",
                    cached_path.display()
                );
            }
        }
    }

    // ─── Shared internals ─────────────────────────────────────────────

    /// Disk-cache filename stem for a source image. The prune pass must derive
    /// its keep-set the same way, so this is the only place the rule lives.
    fn cache_key(original_path: &str) -> String {
        blake3::hash(original_path.as_bytes()).to_string()
    }

    /// Generate (or retrieve from L2 disk cache) a WebP thumbnail.
    fn generate(cache_state: &Mutex<Self>, original_path: &Path) -> Result<PathBuf, AppError> {
        let base_dir = {
            let cache = lock(cache_state);
            cache
                .base_dir
                .clone()
                .ok_or_else(|| AppError::Internal("Cache not initialized".to_string()))?
        };

        // L2 keyed by identity too: the same source image keeps one .webp
        // across enable/disable renames instead of regenerating per toggle.
        let key = Self::cache_key(&Self::identity_key(original_path));
        let cached_path = base_dir.join(format!("{}.webp", key));

        // L2 disk hit — the cached file must be at least as new as its source.
        if let Ok(cache_meta) = fs::metadata(&cached_path) {
            if Self::is_cache_fresh(original_path, &cache_meta) {
                return Ok(cached_path);
            }
        }

        // Generate: Fast thumbnail resize
        // Preview assets in mods are often named with the wrong extension.
        // Detect from the bytes so a valid PNG named `preview.jpg` still works.
        let img = ImageReader::open(original_path)?
            .with_guessed_format()?
            .decode()?;
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
