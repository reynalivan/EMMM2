# Test Cases: Thumbnail Cache System (Epic 41)

## A. Requirement Summary

- **Feature Goal**: Dual-layer thumbnail cache (L1: in-memory`RwLock<HashMap>`, L2: disk WebP at`{app_data_dir}/thumbnails/{blake3_hash}.webp`) for mod folder preview images. Source images downscaled to 256×256 WebP, served via custom`emmm2://` protocol. Concurrent generation capped by`Semaphore(4)`.
- **User Roles**: Application System (automatic), End User (for cache maintenance).
- **User Story**:
 - US-41.1: Auto-generate thumbnails for mod folders.
 - US-41.2: Cache thumbnails with L1 memory hit ≤ 1ms, L2 generation ≤ 200ms.
 - US-41.3: GC: clear old cache, prune orphan entries.
- **Acceptance Criteria**:
 - AC-41.1.1: Folder with`preview.png/jpg` → 256×256 WebP generated, stored, URI returned ≤ 200ms.
 - AC-41.1.2: Concurrent requests capped at 4 simultaneous — others queue without dropping.
 - AC-41.1.3: No images in folder →`None` returned; frontend shows`<ModPlaceholderIcon />`.
 - AC-41.1.4: Corrupted/0-byte image →`None` cached; no repeated re-attempt.
 - AC-41.2.1: Generated WebP stored at`thumbnails/{blake3(folder_path)}.webp`; L1 updated.
 - AC-41.2.2: Repeated request for same folder → L1 hit ≤ 1ms, no disk read or re-encode.
 - AC-41.2.3: Source image`mtime` changed → L1 invalidated, re-encoded.
 - AC-41.2.4:`thumbnails/` dir missing at startup →`create_dir_all` called on first use.
 - AC-41.3.1: "Clear Cache" deletes entries not accessed in > 30 days; count returned.
 - AC-41.3.2:`prune_orphans` deletes WebP files whose`folder_path` no longer in DB.
 - AC-41.3.3: BLAKE3 hash collision (theoretical) → path disambiguation via appended path segment.
- **Success Criteria**: L1 hit ≤ 1ms; L2 generation ≤ 200ms; scroll 200 cards ≤ 10% CPU spike; prune 5,000 entries ≤ 500ms.
- **Main Risks**: L1 cache unbounded memory growth under very large libraries (thousands of mods); source image race condition where file is deleted between path resolution and encode.
---

## B. Coverage Matrix

| Acceptance Criteria | Covered by TC IDs | Requirement File |
| :---------------------------------------- | :---------------- | :-------------------------------------------------------------- |
| AC-41.1.1 (Generate WebP ≤ 200ms) | TC-41-001 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.1.2 (Semaphore(4) throttle) | TC-41-002 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.1.3 (No images → placeholder) | TC-41-003 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.1.4 (Corrupted image → None cached) | TC-41-004 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.2.1 (L2 disk write + L1 update) | TC-41-005 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.2.2 (L1 hit ≤ 1ms) | TC-41-006 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.2.3 (mtime invalidation) | TC-41-007 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.2.4 (Missing thumbnails dir) | TC-41-008 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.3.1 (Clear old cache) | TC-41-009 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.3.2 (Prune orphan entries) | TC-41-010 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |
| AC-41.3.3 (Hash collision handling) | TC-41-011 |`e:\Dev\EMMM2NEW\.docs\requirements\req-41-thumbnail-system.md` |

---

## C. Test Cases

| TC ID | Scenario | Type | Priority | Test Data | Steps | Expected Result | Failure Severity | Coverage |
| :-------- | :------------------------------------------- | :------- | :------- | :------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------- | :-------- |
| TC-41-001 | Generate thumbnail from preview.png | Positive | High |`preview.png` = valid 1024×1024 PNG | 1. Ensure Folder`mods_path/Characters/Keqing/KaeyaMod/` contains`preview.png` (1024×1024, valid PNG). L1 cache empty. L2 disk cache empty.<br>2. Navigate in app to Keqing Object grid.<br>3. Start timer when grid renders.<br>4. Observe the KaeyaMod card thumbnail area.<br>5. Stop timer when thumbnail image appears.<br>6. Check`{app_data_dir}/thumbnails/` for new`.webp` file.<br>7. Verify image dimensions using image viewer or`identify` tool. | Thumbnail appears within ≤ 200ms of first render. File`{app_data_dir}/thumbnails/{blake3hash}.webp` exists on disk. WebP image dimensions are exactly 256×256. | S1 | AC-41.1.1 |
| TC-41-002 | Semaphore throttles concurrent generation | Edge | High | 200 folders, all needing thumbnail gen | 1. Ensure 200 mod folders loaded. All L1 and L2 caches empty (first launch). CPU monitor open.<br>2. Navigate to an Object grid containing 200+ mod cards.<br>3. Open Task Manager (CPU view) before navigating.<br>4. Scroll through the full grid as fast as possible.<br>5. Monitor CPU usage during scroll.<br>6. Check that exactly 4 workers are active at peak (via debug log or profiler). | CPU spike stays ≤ 10% during scroll. At no point more than 4 simultaneous WebP encode operations. Remaining generation requests queue and complete without any being dropped or lost. All cards eventually show thumbnails. | S2 | AC-41.1.2 |
| TC-41-003 | Folder with no images shows placeholder | Negative | High | No images in folder | 1. Ensure Folder`mods_path/Characters/Kaeya/INIOnlyMod/` contains only`mod.ini` — no image files.<br>2. Navigate to Kaeya Object grid.<br>3. Locate`INIOnlyMod` card.<br>4. Observe the thumbnail region of the card.<br>5. Inspect browser developer tools: check`<img>` src attribute on that card. | Card shows a`<ModPlaceholderIcon />` component (generic placeholder graphic). No broken`<img>` tag, no 404 error in console. Backend returns`None` for this folder_path. | S2 | AC-41.1.3 |
| TC-41-004 | Corrupted image does not loop retries | Edge | High |`preview.png` = 0 bytes (empty file) | 1. Ensure Folder contains`preview.png` which is 0 bytes. L1 and L2 caches both empty.<br>2. Navigate to the object grid containing this mod card.<br>3. Observe card thumbnail (first render).<br>4. Re-scroll past the card 5 more times to re-trigger requests.<br>5. Check backend logs for`get_or_generate_thumbnail` call count for this folder. | Card shows placeholder icon (encode returned None). Backend logs show exactly 1 attempt to encode — the`None` result is cached in L1 preventing re-attempts on subsequent requests. No repeated encode loop. | S2 | AC-41.1.4 |
| TC-41-005 | L2 disk file creation + L1 update | Positive | High |`preview.jpg` valid | 1. Ensure Folder with valid`preview.jpg`. Both caches empty.<br>2. Open`{app_data_dir}/thumbnails/` in file explorer (note: empty).<br>3. Navigate to the mod card in the app.<br>4. Wait for thumbnail to render.<br>5. Check`{app_data_dir}/thumbnails/` again.<br>6. Compute`blake3(folder_path)` and check that filename matches. | New`.webp` file exists at`{app_data_dir}/thumbnails/{blake3hash}.webp`. File is a valid WebP image (open with image viewer). L1 HashMap now contains an entry for this folder_path's hash key with the`emmm2://thumbnails/{hash}.webp` URI. | S2 | AC-41.2.1 |
| TC-41-006 | L1 cache hit is sub-millisecond | Positive | High | L1 entry present | 1. Ensure Same mod card thumbnail already generated; L1 cache populated from TC-41-001 or TC-41-005.<br>2. Navigate away from the Object grid.<br>3. Navigate back to the same Object grid.<br>4. Use browser devtools Network tab — filter for`emmm2://` protocol requests.<br>5. Note: L1 hit returns synchronously before any network request.<br>6. Time the card render start to thumbnail display. | Thumbnail appears almost instantly (no visible generation delay). No new`.webp` file written to disk (L2 not re-written). Network tab shows no new`emmm2://thumbnails/...` fetch in progress — the URI is already known synchronously. | S2 | AC-41.2.2 |
| TC-41-007 | mtime change triggers cache invalidation | Edge | High | New`preview.png` (different image) | 1. Ensure`KaeyaMod` thumbnail was generated and cached. Source`preview.png` is then replaced with a different image (new mtime).<br>2. Verify KaeyaMod thumbnail shows in grid (original image).<br>3. Replace`preview.png` with a visually distinct new image file (same filename).<br>4. Navigate away then back to the grid (trigger re-request).<br>5. Observe thumbnail on the card.<br>6. Check new`.webp` timestamp vs old. | Grid shows the NEW thumbnail image. The old L1 entry was invalidated (mtime mismatch). New`.webp` file written to disk (different content from previous). ≤ 200ms re-generation time. | S1 | AC-41.2.3 |
| TC-41-008 | Missing thumbnails directory is auto-created | Edge | Medium | No thumbnails dir | 1. Ensure`{app_data_dir}/thumbnails/` directory does NOT exist (e.g., fresh install or manually deleted).<br>2. Confirm`{app_data_dir}/thumbnails/` does not exist.<br>3. Launch the app (or trigger first thumbnail request).<br>4. Navigate to any Object grid.<br>5. Observe thumbnail generation.<br>6. Inspect`{app_data_dir}/` for thumbnails folder. |`create_dir_all` is called during first thumbnail request.`{app_data_dir}/thumbnails/` directory is created. Thumbnail generation proceeds normally. No error toast. No crash. | S2 | AC-41.2.4 |
| TC-41-009 | Clear old cache removes stale entries | Positive | Medium | 20 WebP files (8 old) | 1. Ensure`{app_data_dir}/thumbnails/` contains 20`.webp` files. 8 have not been accessed in > 30 days (mock old atime).<br>2. Go to Settings > Maintenance.<br>3. Click "Clear Thumbnail Cache".<br>4. Observe the result message.<br>5. Count files remaining in`{app_data_dir}/thumbnails/`. | Exactly 8 stale`.webp` files are deleted. 12 recent files remain untouched. UI confirmation shows count: "Cleared 8 thumbnails". L1 cache entries for deleted files are also removed. | S3 | AC-41.3.1 |
| TC-41-010 | Prune orphan thumbnails after mod deleted | Positive | High | Orphan`.webp` file | 1. Ensure`{app_data_dir}/thumbnails/` has a`.webp` whose source`folder_path` was deleted from the`folders` DB table (e.g., mod was trash-deleted).<br>2. Delete a mod folder (move to trash) — removes the DB row.<br>3. Note: orphan`.webp` file still exists in`thumbnails/`.<br>4. Trigger`prune_orphans` (occurs after a scan commit, or via Settings).<br>5. Check`{app_data_dir}/thumbnails/` for the orphan file. | Orphan`.webp` file is deleted from disk within ≤ 500ms for ≤ 5,000 entries. DB still has no row for that`folder_path`. L1 cache entry for that key is also removed. No`.webp` files for valid/existing mods are deleted. | S2 | AC-41.3.2 |
| TC-41-011 | BLAKE3 hash collision disambiguation | Edge | Low | Simulated hash collision via test mock | 1. Ensure Two different`folder_path` strings are crafted that produce the same BLAKE3 hash (theoretical). In practice: use test hook to force a simulated collision.<br>2. Inject two folders with`folder_path` values that collide to the same hash key (via test hook).<br>3. Request thumbnails for both.<br>4. Inspect cache keys and filenames in`{app_data_dir}/thumbnails/`. | Each folder gets a distinct cache key (original hash + path segment appended). Two separate`.webp` files exist. No data overwrite between the two conflicting entries. Each folder's thumbnail displays. | S3 | AC-41.3.3 |

---

## D. Missing / Implied Test Areas

- **image-first Priority**: Folder contains both`preview.png` AND`preview.jpg` —`preview.png` should be chosen over`preview.jpg`. Also: folder with only`random.jpg` (not named`preview`) should still generate a thumbnail from that first image.
- **Very Large Source Image**: Source image is 8K resolution (7680×4320, 30MB+) — encode time may approach 200ms threshold. Should not block or panic.
- **Thumbnail for Disabled Mod**:`DISABLED KaeyaMod/` folder — does the thumbnail cache key use the full folder path including`DISABLED` prefix? If the mod is toggled to enabled, the path changes, so the cache key must also change and a new thumbnail must be generated.
- **Protocol Security**:`emmm2://thumbnails/../../etc` path traversal attempt via crafted URI — must be blocked by the protocol handler.

---

## E. Open Questions / Gaps

- Is L1 cache bounded in size? For a library with 10,000 mods, the L1 HashMap could consume hundreds of MB of RAM storing URI strings. Should there be an LRU eviction policy?
- Does`clear_old_cache` use file`atime` (access time) on Windows?`atime` is often disabled by default (`NtfsDisableLastAccessUpdate`). If so, the 30-day threshold would never work as expected.

---

## F. Automation Candidates

- **TC-41-001 (Generate WebP)**: Rust integration test — temp folder with`preview.png`, call`get_or_generate_thumbnail`, assert return value is`Some(uri)` and WebP file on disk.
- **TC-41-004 (0-byte image)**: Rust unit test — pass 0-byte file, assert`None` returned AND L1 entry exists (no re-attempt).
- **TC-41-006 (L1 hit)**: Rust unit test — populate L1, call`get_or_generate_thumbnail` second time, assert NO disk write occurred (spy on`fs::write`).
- **TC-41-010 (Orphan prune)**: Rust integration test — insert orphan`.webp`, call`prune_orphans`, assert file deleted.

---

## G. Test Environment Setup

- **OS**: Windows 10/11
- **App**: EMMM2 dev build (`cargo tauri dev`)
- **Game**: Genshin Impact configured
- **Filesystem State** (prepare before TC group):
 - Create:`mods_path/Characters/Keqing/KaeyaMod/preview.png` (valid 1024×1024 PNG)
 - Create:`mods_path/Characters/Kaeya/INIOnlyMod/mod.ini` (no images)
 - Create:`mods_path/Characters/Fischl/ZeroBytemod/preview.png` (0 bytes)
 - Create/clear:`{app_data_dir}/thumbnails/` (empty for fresh-start tests)
- **L1 Cache**: Cleared by restarting the app or calling a cache-reset test hook
- **CPU Monitor**: Task Manager or Resource Monitor open during TC-41-002

## H. Cross-Epic E2E Scenarios

- **E2E-41-01 (Mass Import Auto-Thumbnail Trigger)**: Perform a Mass Archive Import (Epic 23 / 37) of 50 new mods containing varying image assets directly into the`mods_path`. Verify that the subsequent File Watcher (Epic 28) implicitly triggering Sync Scanner (Epic 27) populates the L2 disk cache without blocking any active UI threads simultaneously rendering`S1`.
- **E2E-41-02 (Safe Mode Explicit Thumbnail Block)**: Enter Safe Mode visually (Epic 30) thereby inherently hiding specific NSFW Objects. Verify that any explicit`emmm2://thumbnails/...` protocol fetching directly blocks all related NSFW folder image assets rigorously preventing ANY UI Sidebar caching.`S1`.
