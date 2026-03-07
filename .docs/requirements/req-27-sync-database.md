# Epic 27: Sync Database

## 1. Executive Summary

- **Problem Statement**: After a scan + Deep Matcher produces approved `ScoredCandidate` mappings, the SQLite DB must be updated atomically — creating new Objects if needed, updating `folder_path → object_id` associations, and purging "orphaned" DB rows whose physical folders no longer exist.
- **Proposed Solution**: A `commit_scan` backend command that runs a single SQLite transaction: upserts `folders` rows from the approved candidate list, runs BLAKE3 identity matching to detect moved folders (path changed, content is the same), then calls `repair_orphan_mods` to delete rows with non-existent paths.
- **Success Criteria**:
  - `commit_scan` for 500 approved candidates completes in ≤ 3s (bounded by SQLite batch insert performance).
  - Zero duplicate DB rows after commit — `INSERT OR REPLACE` ensures idempotency.
  - BLAKE3 identity match correctly detects moved folders in ≥ 95% of test cases (50-folder benchmark).
  - `repair_orphan_mods` removes all DB rows with non-existent `folder_path` in ≤ 500ms for a DB with ≤ 2,000 rows.
  - A partial DB write failure (interrupted transaction) leaves the DB in its pre-commit state — no half-applied changes.

---

## 2. User Experience & Functionality

### User Stories

#### US-27.1: Commit Scan Results

As a system, I want to safely write approved scan mappings to the DB, so that the UI shows newly discovered mods correctly on next load.

| ID        | Type        | Criteria                                                                                                                                                                                                       |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-27.1.1 | ✅ Positive | Given N approved `ScoredCandidate` items, when `commit_scan` is called, then the `folders` table is upserted with `folder_path → object_id` associations in a single SQLite transaction                        |
| AC-27.1.2 | ✅ Positive | Given a folder detected as moved (same BLAKE3 hash, different path), when committed, then the existing DB row's `folder_path` is updated rather than creating a duplicate row                                  |
| AC-27.1.3 | ❌ Negative | Given the SQLite transaction is interrupted (crash, power loss), then the DB rolls back to its pre-commit state — no partially-applied rows with inconsistent `object_id` assignments                          |
| AC-27.1.4 | ⚠️ Edge     | Given a candidate's `proposed_object_id` references an Object that doesn't exist in the DB yet (user manually assigned in review), then the commit auto-creates that Object row before inserting the folder FK |
| AC-27.1.5 | ✅ Positive | Given a successful commit, the database stores the folder ID and paths as relative to the `mods_path` (e.g., `Raiden/ModA`), ensuring the library survives moving the entire game folder to another drive      |

---

#### US-27.2: Repair Orphaned Mods

As a user, I want the app to clean up DB entries for folders I deleted externally, so that my library doesn't show "ghost" mods.

| ID        | Type        | Criteria                                                                                                                                                                                                               |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-27.2.1 | ✅ Positive | Given `repair_orphan_mods` runs after commit, then any `folders` row whose `folder_path` does not exist on disk is deleted — the objectlist counts update on next React Query fetch                                       |
| AC-27.2.2 | ✅ Positive | Given N orphans were removed, then the commit result includes `{added, updated, removed}` counts — a toast shows "Scan complete: N added, M updated, K removed"                                                        |
| AC-27.2.3 | ⚠️ Edge     | Given the filesystem check for each row's existence is slow (network drive), then `repair_orphan_mods` runs with a 10s timeout — partial results are still committed, unverified rows are left as-is with a `warn` log |

---

### Non-Goals

- No bi-directional sync — only scan results drive DB updates; the DB does not back-propagate to the filesystem.
- No versioning or history of DB state changes — commit is always applied to the current state.
- Orphan repair is post-scan only — it does not run continuously in the background (that is Epic 28, File Watcher).

---

## 3. Technical Specifications

### Architecture Overview

```
commit_scan(game_id, candidates: Vec<ApprovedCandidate>) → CommitResult:
  1. Begin SQLite exclusive transaction
  2. For each candidate:
     a. If object_id not in DB: INSERT INTO objects (name, category_id, game_id, ...)
     b. Compute BLAKE3 hash of core INI file(s) in candidate.folder_path
        → SELECT folder_path FROM folders WHERE content_hash = ? AND game_id = ?
        → if found: UPDATE folder_path (moved folder) else INSERT new row
     c. INSERT OR REPLACE INTO folders (folder_path, object_id, game_id, is_enabled, ...)
  3. repair_orphan_mods(game_id):
     → SELECT folder_path FROM folders WHERE game_id = ?
     → for each: if !folder_path.exists(): DELETE FROM folders WHERE folder_path = ?
  4. Commit transaction
  5. Return CommitResult { added: u32, updated: u32, removed: u32 }

Frontend:
  ScanReviewModal → "Commit to Library" button
    → invoke('commit_scan', { game_id, candidates })
    → queryClient.invalidateQueries()  (all caches)
```

### Integration Points

| Component        | Detail                                                                                                |
| ---------------- | ----------------------------------------------------------------------------------------------------- |
| DB               | `sqlx` SQLite with `BEGIN EXCLUSIVE / COMMIT` for atomic batch upsert                                 |
| BLAKE3           | `blake3::hash(ini_content_bytes)` — computed at commit time for moved-folder detection                |
| Frontend         | `ScanReviewModal.tsx` — "Commit to Library" → `useMutation(invoke('commit_scan'))`                    |
| Cache Invalidate | `queryClient.invalidateQueries()` on success — refreshes objectlist `['objects']` and grid `['folders']` |

### Security & Privacy

- **All `folder_path` values are validated** with `canonicalize()` + `starts_with(mods_path)` before any DB write.
- **`commit_scan` is idempotent** — running it twice with the same candidates produces the same DB state (no duplicates via `INSERT OR REPLACE` + UNIQUE constraint).
- **Orphan repair deletes only by `folder_path`** — never an unbounded DELETE; always scoped to the confirmed non-existent paths.

---

## 4. Dependencies

- **Blocked by**: Epic 26 (Deep Matcher — provides `Vec<ApprovedCandidate>`), Epic 01 (App Bootstrap — DB pool).
- **Blocks**: Epic 28 (File Watcher — starts watching `mods_path` after initial sync).
