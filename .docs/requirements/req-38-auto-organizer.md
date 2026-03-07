# Epic 38: Auto-Organizer

## 1. Executive Summary

- **Problem Statement**: Users who extract mods manually (not via Epic 37) often end up with a flat `Mods/` root that mixes hundreds of folders across different categories and characters — browsing becomes impractical without a hierarchy like `Mods/Characters/Keqing/`.
- **Proposed Solution**: An `auto_organize_mods` backend command that reads each selected mod's `object_id` from the DB, resolves the target path (`mods_path/{category}/{object_name}/{folder}`), moves the folder using `fs::rename` under `OperationLock` + `SuppressionGuard`, updates `folder_path` in the DB, and reports success/skip/error per item in a `BulkResult`.
- **Success Criteria**:
  - Organizing 100 mod folders completes in ≤ 10s (SSD) — bounded by `fs::rename` syscall speed.
  - Zero data loss on collision — duplicate targets are skipped and reported; the original folder is never moved to a conflicting path.
  - DB `folder_path` is updated atomically with the filesystem rename in a single SQLite transaction.
  - 0 "ghost" DB entries after a successful move — old `folder_path` row is replaced with new path.
  - React Query cache is invalidated after the full batch, not per-move — 1 invalidation per organize call, not N.

---

## 2. User Experience & Functionality

### User Stories

#### US-38.1: Auto-Organize Raw Mod Folders

As a user, I want to select unorganized mod folders and have the app move them to the correct hierarchy, so that my filesystem stays clean.

| ID        | Type        | Criteria                                                                                                                                                                                                           |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-38.1.1 | ✅ Positive | Given a selection of mods with known `object_id`, when I trigger "Auto-Organize", then each mod is moved to `mods_path/{category}/{object_name}/{folder_name}` — folder name is preserved, only its parent changes |
| AC-38.1.2 | ✅ Positive | Given a successful move, then the `folders` DB row's `folder_path` is updated to the new path in the same DB transaction as the metadata update — no orphaned rows                                                 |
| AC-38.1.3 | ✅ Positive | Given the batch completes, then a toast shows "Organized N mods" and `queryClient.invalidateQueries()` refreshes the grid                                                                                          |
| AC-38.1.4 | ❌ Negative | Given the target destination `mods_path/{category}/{object_name}/{folder_name}` already exists, then that mod is skipped — logged as "DUPLICATE" in `BulkResult.errors`; the original folder is NOT moved          |
| AC-38.1.5 | ❌ Negative | Given a mod has `object_id = NULL` (uncategorized), then it is also skipped with status "NO_OBJECT" — organize cannot place what has no owner                                                                      |
| AC-38.1.6 | ⚠️ Edge     | Given the mod is currently enabled (`is_enabled = true`), then the rename/move still proceeds (the new path retains the filename without "DISABLED " prefix) — the enabled state is preserved correctly            |

---

### Non-Goals

- No recursive sub-folder organization — only root-level mod folders are moved.
- No batch auto-organize of the entire library without explicit selection.
- No category-only organization mode (e.g., move all to `Characters/` without Object subfolder).

---

## 3. Technical Specifications

### Architecture Overview

```
auto_organize_mods(game_id, folder_paths: Vec<PathBuf>) → BulkResult:
  1. Acquire OperationLock(game_id) + WatcherSuppression(all paths)
  2. Load GameSchema → resolve category + object_name for each folder_path
     via: SELECT o.name, c.name FROM folders f JOIN objects o ON ... JOIN categories c ON ...
     if object_id is NULL: skip → BulkResult.errors.push({ path, reason: "NO_OBJECT" })
  3. For each path:
     target = mods_path / category_name / object_name / folder_basename
     if target.exists():
       → skip → BulkResult.errors.push({ path, reason: "DUPLICATE" })
     else:
       fs::create_dir_all(target.parent())
       fs::rename(path, target)
       → BEGIN TRANSACTION
         UPDATE folders SET folder_path = target WHERE folder_path = path AND game_id = ?
       → COMMIT
       BulkResult.success.push(target)
  4. Release suppression + lock
  5. Return BulkResult

Frontend:
  → invoke('auto_organize_mods', { game_id, folder_paths })
  → on success: queryClient.invalidateQueries(['folders', gameId])
               + toast summary
```

### Integration Points

| Component                          | Detail                                                                                     |
| ---------------------------------- | ------------------------------------------------------------------------------------------ |
| OperationLock + WatcherSuppression | Scoped to the entire batch — prevents intermediate FS events                               |
| DB Update                          | `UPDATE folders SET folder_path = new WHERE folder_path = old` in same txn as `fs::rename` |
| Category/Object Lookup             | `JOIN objects + categories` using current DB state (no GameSchema reload needed)           |
| Master DB                          | Shared with Epic 26 — `object_name` resolved from DB, not re-matched                       |
| Frontend                           | Context menu "Auto-Organize" on bulk selection → `invoke('auto_organize_mods')`            |

### Security & Privacy

- **`target` path validated** with `canonicalize(target.parent()) + starts_with(mods_path)` before every `fs::rename` — no escape from `mods_path` hierarchy.
- **DB transaction is atomic** — if `fs::rename` succeeds but DB update fails, the transaction rolls back; the file remains at new path but orphan detection (Epic 27 `repair_orphan_mods`) will reconcile on next scan.
- **`OperationLock` prevents concurrent organize calls** on the same game — no race conditions between two batch operations.

---

## 4. Dependencies

- **Blocked by**: Epic 26 (Deep Matcher/Master DB — `object_id` must be known for target path computation), Epic 28 (File Watcher — WatcherSuppression), Epic 09 (Object Schema — category names).
- **Blocks**: Nothing — organizational utility leaf feature.
