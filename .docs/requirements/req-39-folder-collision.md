# Epic 39: Folder Collision Resolution

## 1. Executive Summary

- **Problem Statement**: The `DISABLED ` prefix toggle system creates naming collisions — if `DISABLED Kaeya` exists and the user tries to enable it while a folder named `Kaeya` also exists, `fs::rename` fails; without a structured resolution flow, the user sees a cryptic OS error and cannot proceed.
- **Proposed Solution**: A `CommandError::PathCollision` error variant from toggle/move operations; the frontend catches it, opens `ConflictResolveDialog` with a side-by-side comparison (file counts, sizes, thumbnails, INI lists for both paths); the user picks a resolution strategy (`KeepEnabled`, `KeepBoth`, `ReplaceWithIncoming`); the backend renames the conflicting folder with a collision-safe suffix and auto-retries the original operation.
- **Success Criteria**:
  - `ConflictResolveDialog` opens within ≤ 300ms of a `PathCollision` error.
  - Side-by-side comparison data (file counts, sizes, thumbnail, INI list) loads in ≤ 500ms per folder.
  - `resolve_conflict` + auto-retry of the original toggle completes in ≤ 1s (SSD).
  - Suffix generation guarantees a free name in ≤ 10 attempts: `{name} (dup)`, `{name} (dup 2)`, `{name} (dup 3)`, ...
  - Zero data loss — no folder is deleted during collision resolution; all paths remain on disk.

---

## 2. User Experience & Functionality

### User Stories

#### US-39.1: Side-By-Side Comparison

As a user, I want to see exactly what's inside both conflicting folders before deciding which to keep, so that I don't accidentally discard the better version.

| ID        | Type        | Criteria                                                                                                                                                                                                              |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-39.1.1 | ✅ Positive | Given an enable operation returns `CommandError::PathCollision`, then `ConflictResolveDialog` opens within ≤ 300ms; it shows both paths (enabled + disabled variant) as labeled columns                               |
| AC-39.1.2 | ✅ Positive | Given the dialog is open, then `get_collision_info(path_a, path_b)` returns `{ file_count, total_size_bytes, thumbnail_path, ini_files: Vec<String> }` for each folder — displayed as two side-by-side summary panels |
| AC-39.1.3 | ⚠️ Edge     | Given one of the conflicting folders has no `.ini` files, then its INI list shows "No INI files found" — not an error                                                                                                 |

---

#### US-39.2: Resolution Strategies

As a user, I want multiple options for resolving a folder conflict, so that I can choose the safest approach for my library.

| ID        | Type        | Criteria                                                                                                                                                                                                              |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-39.2.1 | ✅ Positive | Given a collision, when I choose "Keep Enabled Version", then the `DISABLED {name}` folder is renamed to `DISABLED {name} (dup)` (first free suffix) — the `{name}` folder stays; the toggle is retried automatically |
| AC-39.2.2 | ✅ Positive | Given a collision, when I choose "Keep Both (Separate)", then `DISABLED {name}` is renamed to `DISABLED {name} (copy)` — both folders remain independently accessible; no further toggle action                       |
| AC-39.2.3 | ✅ Positive | Given "Replace With Incoming" strategy: the existing `{name}` folder is moved to Trash (via `trash::delete`); the incoming `DISABLED {name}` toggle completes to `{name}`                                             |
| AC-39.2.4 | ❌ Negative | Given the assigned suffix `(dup N)` already exists for N = 1..10, then on Nth attempt a unique random UUID suffix is appended as final fallback — prevents infinite loop                                              |
| AC-39.2.5 | ⚠️ Edge     | Given I close the dialog without choosing a strategy, then the original toggle is cancelled; both folders remain untouched; no toast error                                                                            |

---

### Non-Goals

- No automatic collision resolution without user input — every collision requires an explicit strategy choice.
- No "merge" strategy — folders are never merged (file-level merge is out of scope).
- Collision resolution is triggered only by toggle/import/organize operations — not as a standalone "scan for collisions" tool.

---

## 3. Technical Specifications

### Architecture Overview

```
FolderCollisionInfo { path, file_count, total_size_bytes, thumbnail_path, ini_files: Vec<String> }

get_collision_info(path_a, path_b) → (FolderCollisionInfo, FolderCollisionInfo):
  for each path: shallow walk → count files, sum sizes, find first image, collect *.ini files
  (bounded: max_files_scanned = 100 per folder, max depth = 3)

resolve_conflict(game_id, collision_path, conflict_path, strategy: ConflictStrategy) → ():
  1. Acquire OperationLock(game_id) + WatcherSuppression([collision_path, conflict_path])
  2. match strategy:
     KeepEnabled → new_name = find_free_suffix(conflict_path, "(dup)")
                   fs::rename(conflict_path, new_name)
     KeepBoth    → new_name = find_free_suffix(conflict_path, "(copy)")
                   fs::rename(conflict_path, new_name)
     Replace     → trash::delete(collision_path)
  3. Release lock + suppression
  4. Return Ok(()) // frontend auto-retries original toggle

find_free_suffix(path, suffix) → PathBuf:
  try: "{name} {suffix}", "{name} {suffix} 2", ..., "{name} {suffix} 10"
  if all taken: "{name} {suffix} {uuid4}"
```

### Integration Points

| Component   | Detail                                                                                          |
| ----------- | ----------------------------------------------------------------------------------------------- |
| Error Catch | `useFolderGridActions.ts` catches `CommandError::PathCollision` → opens `ConflictResolveDialog` |
| Dialog Data | `get_collision_info(path_a, path_b)` — shallow folder scan, returns summary structs             |
| Resolution  | `invoke('resolve_conflict', { game_id, strategy, ... })` → OperationLock scope                  |
| Auto-Retry  | After `resolve_conflict` returns Ok, frontend re-calls the original toggle mutation             |
| Trash       | "Replace" strategy uses `trash::delete` (Epic 22) — never `fs::remove_dir_all`                  |

### Security & Privacy

- **All paths validated** with `canonicalize()` + `starts_with(mods_path)` before any rename or trash call.
- **UUID fallback suffix** prevents suffix-enumeration stalls while guaranteeing uniqueness.
- **`OperationLock`** prevents a second toggle from racing against the resolution rename.

---

## 4. Dependencies

- **Blocked by**: Epic 20 (Mod Toggle — collision originates from toggle `fs::rename` failure), Epic 22 (Trash Safety — "Replace" strategy uses trash delete).
- **Blocks**: Nothing — it is an error handler and resolution wrapper for the toggle pipeline.
