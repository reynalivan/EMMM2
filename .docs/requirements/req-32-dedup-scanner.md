# Epic 32: Smart Duplicate Scanner (Storage Optimizer)

## 1. Executive Summary

- **Problem Statement**: Mod management frequently results in accumulating duplicate heavy assets (`.dds`, `.vb`, `.ib`) across different folder structures or imports, silently devouring disk space.
- **Proposed Solution**: A parallel BLAKE3 hashing scanner employing a Multi-Signal Matching Algorithm (evaluating structure, partial-hashing for large files, and content identity). Generates a Duplicate Comparison Report UI where users resolve conflicts using NTFS Hardlinks (space reclaiming without breaking mods) or Trash (soft delete).
- **Success Criteria**:
  - Scanning 1,000 files (avg 10MB) completes in ≤ 15s using `rayon` multi-threading (CPU scales to 80-90%).
  - Multi-Signal matching uses 1KB + 1KB partial sampling for files > 5MB, achieving a 100x speed increase for massive textures.
  - Partial scans can be safely cancelled within ≤ 1s.
  - Resolving via NTFS hardlink decreases duplicate bytes used by ≥ 95% on identical volumes without breaking folder integrity.
  - DB table `duplicate_reports` caches results, eliminating re-scan requirements across UI navigations.

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

#### US-32.2: Conflict Resolution Interface (Report Table)

As a user, I want to review duplicates side-by-side and choose bulk resolutions, so that clearing space is rapid and safe.

| ID        | Type        | Criteria                                                                                                                                                                                                     |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-32.2.1 | ✅ Positive | Given the scan completes, metrics reflect grouped pairs in `duplicate_reports` DB table: Side-by-side folder paths, size MB/GB wasted, and match justification (e.g., "Hash 100% Match")                     |
| AC-32.2.2 | ✅ Positive | Given bulk action radio buttons (Keep Original, Replace, Ignore/Whitelist), when I select resolutions and hit "Apply All Changes", the instructions process sequentially locked under `OperationLock`        |
| AC-32.2.3 | ✅ Positive | Given standard identical folders (e.g., "Albedo" and "DISABLED Albedo"), the system natively groups these regardless of the `DISABLED ` physical toggle prefix                                               |
| AC-32.2.4 | ❌ Negative | Given identical files across different physical drives, choosing "Hardlink" will cleanly fallback with a "Requires Delete Strategy - Cross-Drive Link Unsupported" warning, safely preserving both originals |

---

#### US-32.3: Safe Deletion & Trashing

As a user, I want the delete resolution to act as a soft-delete, so I can restore a folder if compiling breaks the mod.

| ID        | Type        | Criteria                                                                                                                                                            |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-32.3.1 | ✅ Positive | Given the "Replace" or "Delete" option, the deleted folder is moved directly to the custom `./app_data/trash/` location (Epic 22 rule), never permanently destroyed |
| AC-32.3.2 | ⚠️ Edge     | Given the resolution affects multiple paths inside an object grid, TanStack query `['mods', gameId]` invalidates to refresh the app's overall memory sizes          |

---

### Non-Goals

- No automatic background deduplication — always an explicit user-run action.
- Hardlinking restricted specifically against standard portable metadata files (`info.json` or `.ini` text lines are skipped if sizes < 512KB).
- Cross-game deduplication is completely ignored (scoped only to the current `game_id`).

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Parallel Hashing Pipeline
fn calculate_folder_signature(folder_path: &Path) -> Vec<FileSignature> {
    let files = walkdir::WalkDir::new(folder_path)...;
    files.into_par_iter().map(|path| {
        let size = file.metadata().len();
        let mut hasher = blake3::Hasher::new();
        if size > 5_000_000 {
            // Partial Sampling (1KB head + 1KB tail) for heavy textures
            hasher.update(&read_head(&file, 1024));
            hasher.update(&read_tail(&file, 1024));
        } else {
            hasher.update_reader(&mut file); // Full hash
        }
        FileSignature { hash, size, rel_path }
    }).collect()
}

// Database Schema (Caches results for UI persistence)
CREATE TABLE duplicate_reports (
    id INTEGER PRIMARY KEY,
    group_id TEXT NOT NULL,
    mod_a_id TEXT NOT NULL,
    mod_b_id TEXT NOT NULL,
    confidence_score INTEGER,
    match_reason TEXT,
    resolution TEXT DEFAULT 'PENDING' // KEEP_A, KEEP_B, IGNORE
);
```

### Integration Points

| Component     | Detail                                                                                            |
| ------------- | ------------------------------------------------------------------------------------------------- |
| Parallelism   | Uses `rayon::prelude::*` for heavy IO/CPU workload scaling out to all logic cores.                |
| Hardlinks     | `fs::hard_link(keep_path, target_path)`. Fails safely on `EXDEV` (cross-device).                  |
| Trash Service | Epic 22 Trash Service handles standard deletions safely to `/app_data/trash/`.                    |
| Report DB     | UI pulls paginated tables off SQLite `duplicate_reports` ensuring heavy scans remain dismissible. |

### Security & Privacy

- **Safe Recovery**: File removals execute soft delete procedures exclusively.
- **Operation Guarantee**: Scan reads are lock-free and robust against `EACCESS`. Writes require global `OperationLock` + `WatcherSuppression` arrays during actual application loop to halt mid-way anomalies or recursive refresh triggers.

---

## 4. Dependencies

- **Blocked by**: Epic 22 (Trash Safety hooks), Epic 28 (File Watcher suppression).
- **Blocks**: None — Terminal action.
