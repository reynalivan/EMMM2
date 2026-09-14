# Epic 32: Smart Duplicate Scanner (Storage Optimizer)

## 1. Executive Summary

- **Problem Statement**: Mod management frequently results in accumulating duplicate heavy assets (`.dds`, `.vb`, `.ib`) across different folder structures or imports, silently devouring disk space.
- **Proposed Solution**: A two-pass BLAKE3 scanner that treats each terminal mod root as one logical unit. Partial hashes reduce candidates, full hashes prove exact identity, and only exact copies receive destructive resolution controls.
- **Success Criteria**:
  - Scanning 1,000 files (avg 10MB) completes in ≤ 15s using `rayon` multi-threading (CPU scales to 80-90%).
  - Multi-Signal matching uses 1KB + 1KB partial sampling for files > 5MB, achieving a 100x speed increase for massive textures.
  - Partial scans can be safely cancelled within ≤ 1s.
  - Variant-Awareness: A merged/orchestrated root and all owned descendants are one candidate. Child subvariants are never compared as independent mods.
  - Persistent Whitelist: Ignored pairs are stored in the database and can be recovered via the UI.
  - Dedicated UI: A full-screen management interface at `/storage-optimizer`.

---

## 2. User Experience & Functionality

### User Stories

#### US-32.1: Multi-Signal Heavy Scan

As a user, I want the system to aggressively identify actual duplicates through structural and hash analyses, so that false positives are minimized and massive files are caught quickly.

| ID        | Type        | Criteria                                                                                                                                                                                                         |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-32.1.1 | ✅ Positive | Given the Dedup scanner starts, the system analyzes Multi-Signals: applies 1KB front + 1KB back partial BLAKE3 hashing on assets > 5MB, identifying duplicates significantly faster than reading exact full bits |
| AC-32.1.2 | ✅ Positive | Given the fast partial-hash matches, a full BLAKE3 verification pass confirms identity with cryptographic certainty guarantee before presenting the match to the user                                            |
| AC-32.1.3 | ✅ Positive | Given the scan finishes, a `DuplicateReport` groups idential files along with a Match Confidence percentage (accounting for structural folder similarities and front-name prefix matches)                        |
| AC-32.1.4 | ❌ Negative | Given file A is heavily read-locked by the game, the scanner gracefully logs a warning, skips the file, and proceeds to the remaining thousands without throwing a full error                                    |
| AC-32.1.5 | ⚠️ Edge     | Standard ignore patterns (`desktop.ini`, `thumbs.db`, `.DS_Store`) are hard-bypassed completely from the hashing tree array                                                                                      |

---

#### Logical ownership and exactness contract

- A terminal `ModPackRoot`, `VariantContainer`, or flat mod root owns every descendant DB row. Nested or disabled child INIs do not create extra candidates.
- A same-name, same-head/tail, or same-target shader/resource relation without an identical full manifest is not an exact duplicate.
- Every regular content file participates in exact verification. OS noise and dot-prefixed internal staging directories are excluded consistently.
- Similarity edges are pairwise evidence only; they cannot transitively promote a group to exact identity.

#### US-32.2: Conflict Resolution Interface (Report Table)

As a user, I want to review duplicates side-by-side and choose bulk resolutions, so that clearing space is rapid and safe.

| ID        | Type        | Criteria                                                                                                                                                                                              |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-32.2.1 | ✅ Positive | Given the scan completes, metrics reflect grouped members in `duplicate_reports` DB table: Dropdown selection for "Targeted Keep" (keep one, delete N-1), size MB/GB wasted, and match justification. |
| AC-32.2.2 | ✅ Positive | Given bulk action buttons (Ignore, Trash, Keep), when I select resolutions, the instructions process sequentially locked under `OperationLock`.                                                       |
| AC-32.2.3 | ✅ Positive | Given an "Ignored" button in the header, clicking it opens an `IgnoredPairsModal` to view and recover (remove from ignore list) whitelisted pairs                                                     |
| AC-32.2.4 | ✅ Positive | The UI at `/storage-optimizer` provides a full-width experience with confidence filtering (All, High, Medium, Low) and a detailed Match Reason signal badge list per group                            |
| AC-32.2.5 | ✅ Positive | Recovered pairs (removed from ignore list) are immediately available for re-scanning and resolution in the next scan run                                                                              |

---

#### Report lifecycle contract

- A completed report is persisted and loaded per `game_id`. Starting, failing, or cancelling another scan does not replace the last successful report.
- `Started`, `Finished`, `Cancelled`, and `Failed` are distinct lifecycle outcomes. The UI refreshes its report only after `Finished`.
- Keep/Delete/Hardlink controls require verified 100% identity. Non-exact relationships remain review/ignore-only.

#### US-32.3: Safe Deletion & Trashing

As a user, I want the delete resolution to act as a soft-delete, so I can restore a folder if compiling breaks the mod.

| ID        | Type        | Criteria                                                                                                                                                   |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-32.3.1 | ✅ Positive | Given the "Replace" or "Delete" option, the removed folder is sent through the recoverable recycle/trash service, never permanently destroyed              |
| AC-32.3.2 | ⚠️ Edge     | Given the resolution affects multiple paths inside an object grid, TanStack query `['mods', gameId]` invalidates to refresh the app's overall memory sizes |

---

#### Resolution safety contract

- Before Keep/Delete/Hardlink, the resolver recomputes full path-aware manifests for both folders.
- If either folder changed after scanning, both folders are preserved and the request is reported as failed.
- Removal uses the recoverable recycle/trash service. Hardlink replacement stages the original file and restores it on failure.

### Non-Goals

- No automatic background deduplication — always an explicit user-run action.
- OS-generated noise files and dot-prefixed internal directories are excluded from hardlink replacement; ordinary mod metadata remains part of identity verification.
- Cross-game deduplication is completely ignored (scoped only to the current `game_id`).

---

## 3. Technical Specifications

### Architecture Overview

```rust
fn scan_exact_duplicates(mods_root: &Path) -> Vec<DuplicateGroup> {
    let logical_units = scan_terminal_mod_roots(mods_root);
    let candidates = score_partial_snapshots(&logical_units);
    let verified = candidates
        .into_par_iter()
        .filter_map(full_manifest_match);
    group_identical_manifests(verified)
}

// Database Schema (Whitelist / Ignore Management)
CREATE TABLE duplicate_whitelist (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    folder_a_path TEXT NOT NULL,
    folder_b_path TEXT NOT NULL,
    folder_a_name TEXT NOT NULL,
    folder_b_name TEXT NOT NULL,
    reason TEXT,
    ignored_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Integration Points

| Component     | Detail                                                                                                                                  |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Parallelism   | Uses `rayon::prelude::*` for heavy IO/CPU workload scaling out to all logic cores.                                                      |
| Hardlinks     | Full manifests are revalidated immediately before per-file replacement; failed replacement restores the staged original.                |
| Trash Service | Recoverable recycle/trash service handles folder removal after exact revalidation.                                                      |
| Whitelist IR  | `get_ignored_pairs` and `remove_ignored_pair` commands provide recovery for whitelisted duplicates.                                     |
| Report DB     | Latest completed report is stored transactionally in existing `dedup_jobs`, `dedup_groups`, and `dedup_group_members`, scoped per game. |

### Security & Privacy

- **Safe Recovery**: File removals execute soft delete procedures exclusively.
- **Operation Guarantee**: Scan reads are lock-free and robust against `EACCESS`. Writes require global `OperationLock` + `WatcherSuppression` arrays during actual application loop to halt mid-way anomalies or recursive refresh triggers.
- **Stale-input rejection**: Folder paths supplied by the UI are not proof. The resolver requires path-aware, full-BLAKE3 manifest equality immediately before Keep/Delete/Hardlink.

---

## 4. Dependencies

- **Blocked by**: Epic 22 (Trash Safety hooks), Epic 28 (File Watcher suppression).
- **Blocks**: None — Terminal action.
