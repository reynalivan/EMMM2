# Epic 41: Thumbnail Cache System

## 1. Executive Summary

- **Problem Statement**: Rendering a grid of hundreds of mod cards backed by raw 4K `.png` previews causes CPU spikes, RAM exhaustion, and jank — each image must be decoded, scaled, and served on every render without any caching.
- **Proposed Solution**: A dual-layer thumbnail cache: L1 (in-memory `RwLock<HashMap<path_hash, WebpUri>>`) for instant repeated access, L2 (disk: `{app_data_dir}/thumbnails/{blake3_hash}.webp`) for cross-session persistence. Source images are detected via a priority strategy (`preview.png` > `preview.jpg` > first image in folder), downscaled to 256×256 WebP via the `image` crate, served through a custom `emmm2://` protocol. Concurrent generation is throttled by a `Semaphore`.
- **Success Criteria**:
  - First thumbnail generation (L2 miss): ≤ 200ms per image (256×256 WebP encode on modern CPU).
  - L1 cache hit (repeated scroll): ≤ 1ms response (pure HashMap lookup).
  - Concurrent generation limited to ≤ 4 simultaneous ops (configurable `Semaphore`) — grid scroll of 200 cards causes ≤ 10% CPU spike.
  - Cache invalidation on source image change (mtime mismatch) is detected and re-encoded within ≤ 200ms.
  - `prune_orphans` removes orphaned thumbnails in ≤ 500ms for ≤ 5,000 cache entries.

---

## 2. User Experience & Functionality

### User Stories

#### US-41.1: Automatic Thumbnail Generation

As a user, I want the app to automatically show preview images for my mods, so that the grid is visually rich and identifiable at a glance.

| ID        | Type        | Criteria                                                                                                                                                                                                                           |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-41.1.1 | ✅ Positive | Given a mod folder containing `preview.png` or `preview.jpg`, when the grid requests a thumbnail, then the backend generates a 256×256 WebP, stores it to disk, and returns an `emmm2://thumbnails/{hash}.webp` URI within ≤ 200ms |
| AC-41.1.2 | ✅ Positive | Given hundreds of thumbnails requested simultaneously (e.g., first grid load), then concurrent generation is capped at `Semaphore(4)` — 4 concurrent encode ops max; remaining requests queue without dropping                     |
| AC-41.1.3 | ❌ Negative | Given a mod folder has no image files, then `None` is returned; the frontend renders a generic fallback `<ModPlaceholderIcon />` — no broken image tag                                                                             |
| AC-41.1.4 | ⚠️ Edge     | Given a mod folder has a `preview.png` that is 0 bytes or not a valid image, then the encode fails silently; `None` is cached to avoid re-attempting on every request                                                              |

---

#### US-41.2: Persistent & Fast Caching

As a system, I want to cache generated WebP thumbnails to disk and serve L1 hits from memory, so that the grid scrolls smoothly without re-encoding.

| ID        | Type        | Criteria                                                                                                                                                                |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-41.2.1 | ✅ Positive | Given a thumbnail is generated, then it is saved to `{app_data_dir}/thumbnails/{blake3(folder_path)}.webp`; the L1 `RwLock<HashMap>` is updated with the `emmm2://` URI |
| AC-41.2.2 | ✅ Positive | Given a repeated request for the same folder (fast scroll, re-mount), then the L1 HashMap returns the URI in ≤ 1ms — no disk read, no re-encode                         |
| AC-41.2.3 | ✅ Positive | Given the source image's `mtime` has changed since last cache entry, then the L1 entry is invalidated, the WebP is re-generated and the new URI is served               |
| AC-41.2.4 | ⚠️ Edge     | Given `app_data_dir/thumbnails/` does not exist at startup, then `create_dir_all` is called once during bootstrap — no error on first generation                        |

---

#### US-41.3: Cache Maintenance & GC

As a user, I want old thumbnails cleaned up automatically, so that my `app_data` folder doesn't grow unbounded over time.

| ID        | Type        | Criteria                                                                                                                                                                                                                  |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-41.3.1 | ✅ Positive | Given the Settings > Maintenance "Clear Thumbnail Cache" button, when clicked, then `clear_old_cache(max_age_days=30)` deletes thumbnails not accessed in the last 30 days; count of freed files is returned              |
| AC-41.3.2 | ✅ Positive | Given `prune_orphans` runs (triggered by post-scan commit), then any `thumbnails/{hash}.webp` whose `folder_path` no longer exists in the `folders` DB table is deleted from disk                                         |
| AC-41.3.3 | ⚠️ Edge     | Given the same `blake3(folder_path)` hash maps to two different physical paths (hash collision — statistically impossible with BLAKE3 but must handle), then the cache key is disambiguated by appending the path segment |

---

### Non-Goals

- No server-side image hosting — thumbnails are always local disk WebP files.
- No animated GIF/WebP support — only static first-frame renders.
- No user-uploadable thumbnails from URLs — only local file drag-and-drop (Epic 15).
- No thumbnail size options (always 256×256) in this epic.

---

## 3. Technical Specifications

### Architecture Overview

```
ThumbnailCache state (Tauri managed):
  l1: Arc<RwLock<HashMap<String, CacheEntry>>>
    CacheEntry { uri: String, source_mtime: SystemTime }
  semaphore: Arc<Semaphore>  // permits = 4

get_or_generate_thumbnail(folder_path: PathBuf) → Option<String>:
  hash_key = blake3::hash(folder_path.as_bytes()).to_hex()
  1. Read L1: if let Some(entry) = l1.read()[hash_key]:
       if source_mtime == fs::metadata(source_image).modified(): return Some(entry.uri)
       else: invalidate entry
  2. Acquire semaphore permit
  3. Find source image: folder_path/preview.png > preview.jpg > first *.{jpg,png,webp}
     if None: l1.write().insert(hash_key, CacheEntry::EMPTY); return None
  4. let img = image::open(source)? → resize(256, 256, Lanczos3) → encode WebP
  5. fs::write(thumbnails_dir / "{hash_key}.webp", webp_bytes)
  6. l1.write().insert(hash_key, CacheEntry { uri: "emmm2://thumbnails/{hash_key}.webp", source_mtime })
  7. Return Some(uri)

emmm2:// custom protocol handler:
  map "emmm2://thumbnails/{hash}.webp" → fs::read(thumbnails_dir/{hash}.webp)
  serve with Content-Type: image/webp
```

### Integration Points

| Component                 | Detail                                                                                       |
| ------------------------- | -------------------------------------------------------------------------------------------- |
| `image` crate             | Resize + WebP encode: `image::open(path).resize(256,256,Lanczos3).to_webp()`                 |
| BLAKE3                    | `blake3::hash(folder_path.as_bytes())` — filename-safe hex key                               |
| Custom Protocol           | `emmm2://` registered in `tauri.conf.json` → serves local WebP bytes                         |
| FolderCard + PreviewPanel | `<img src={thumbnail_uri} />` — uses native browser cache once `emmm2://` URI is stable      |
| GC Trigger                | `prune_orphans` called after `commit_scan` (Epic 27); `clear_old_cache` exposed via Settings |

### Security & Privacy

- **`emmm2://` protocol serves only from `{app_data_dir}/thumbnails/`** — no arbitrary path serving.
- **Source image path validated** with `canonicalize()` + `starts_with(mods_path)` before reading.
- **Safe Mode respected**: If `safeMode && !mod.is_safe`, the frontend renders the blur placeholder icon — the thumbnail URI is never passed to the `<img>` tag.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap — `app_data_dir`), Epic 11 (Folder Listing — `folder_path` as cache key).
- **Blocks**: Epic 12 (Folder Grid — FolderCard thumbnail display), Epic 19 (Image Gallery — preview panel image serving).
