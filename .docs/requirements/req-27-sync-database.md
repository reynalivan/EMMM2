# Epic 27: Sync Database

## 1. Executive Summary

- **Problem Statement**: After an explicit scan + Deep Match Scanner review produces approved mappings, SQLite must be updated atomically with canonical enrichment metadata. Passive filesystem projection is owned by Disk Reconcile, not by scan commit.
- **Proposed Solution**: A `commit_scan` backend command that runs a single SQLite transaction for explicit enrichment: upserts object/mod mappings from the approved candidate list, applies user-approved canonical metadata, and leaves physical add/remove/rename/status projection to Disk Reconcile.
- **Success Criteria**:
  - `commit_scan` for 500 approved candidates completes in ≤ 3s (bounded by SQLite batch insert performance).
  - Zero duplicate DB rows after commit — `INSERT OR REPLACE` ensures idempotency.
  - BLAKE3 identity match correctly detects moved folders in ≥ 95% of test cases (50-folder benchmark).
  - Scan commit does not act as the continuous filesystem sync path; watcher/startup/refocus/manual repair use Disk Reconcile for passive projection cleanup.
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

#### US-27.2: Explicit Commit Cleanup Boundary

As a system, I want scan commit and passive filesystem reconcile to have separate responsibilities, so user-approved MasterDB enrichment never becomes a background auto-matcher.

| ID        | Type        | Criteria                                                                                                                                                                                     |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-27.2.1 | ✅ Positive | Given scan commit runs, then it writes only the approved enrichment/mapping changes in one transaction; passive deletion of missing physical folders remains Disk Reconcile's responsibility |
| AC-27.2.2 | ✅ Positive | Given N orphans were removed, then the commit result includes `{added, updated, removed}` counts — a toast shows "Scan complete: N added, M updated, K removed"                              |
| AC-27.2.3 | ⚠️ Edge     | Given the filesystem source is unavailable, scan commit must not mass-delete DB rows; Disk Reconcile returns a no-write unavailable-source result instead                                    |

---

### Non-Goals

- No bi-directional sync — only scan results drive DB updates; the DB does not back-propagate to the filesystem.
- No versioning or history of DB state changes — commit is always applied to the current state.
- Orphan repair is not the continuous sync mechanism. Disk Reconcile owns passive filesystem projection from startup, Mods entry, refocus, watcher, internal mutation completion, and manual repair triggers.

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
  3. Commit transaction
  4. Trigger projection refresh for affected objects/mods
  5. Return CommitResult { added: u32, updated: u32, removed: u32 }

Frontend:
  ScanReviewModal → "Commit to Library" button
    → commands.commitScan({ game_id, candidates })
    → publish runtime-sync descriptors for object/folder/preview refresh
```

### Integration Points

| Component     | Detail                                                                                                                 |
| ------------- | ---------------------------------------------------------------------------------------------------------------------- |
| DB            | `sqlx` SQLite with `BEGIN EXCLUSIVE / COMMIT` for atomic batch upsert                                                  |
| BLAKE3        | `blake3::hash(ini_content_bytes)` — computed at commit time for moved-folder detection                                 |
| Frontend      | `ScanReviewModal.tsx` — "Commit to Library" → `useMutation(commands.commitScan())`                                     |
| Cache Refresh | Uses descriptor/runtime refresh coordination on success; feature code should not reintroduce ad-hoc broad invalidation |

### Security & Privacy

- **All `folder_path` values are validated** with `canonicalize()` + `starts_with(mods_path)` before any DB write.
- **`commit_scan` is idempotent** — running it twice with the same candidates produces the same DB state (no duplicates via `INSERT OR REPLACE` + UNIQUE constraint).
- **Orphan repair deletes only by `folder_path`** — never an unbounded DELETE; always scoped to the confirmed non-existent paths.

---

## 4. Dependencies

- **Blocked by**: Epic 26 (Deep Match Scanner — provides `Vec<ApprovedCandidate>`), Epic 01 (App Bootstrap — DB pool).
- **Blocks**: Epic 28 (File Watcher — starts watching `mods_path` after initial sync).
