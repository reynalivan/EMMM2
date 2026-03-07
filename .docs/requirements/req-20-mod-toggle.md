# Epic 20: Mod Toggle Operations

## 1. Executive Summary

- **Problem Statement**: Enabling/disabling a mod by adding/removing a `DISABLED ` prefix from the folder name is the single most frequent user action — it must be instant from the UI's perspective and safe against concurrent toggles, folder locks, and rename collisions.
- **Proposed Solution**: A `toggle_mod` Tauri command that acquires `OperationLock` + `WatcherSuppression`, performs `fs::rename` with a pre-check for collision, returns structured errors on failure, and is wrapped by an optimistic React Query cache update with `onError` rollback.
- **Success Criteria**:
  - UI optimistic toggle update applies within ≤ 16ms (one frame).
  - `fs::rename` completes within ≤ 300ms on SSD.
  - ObjectList `enabled_count` badge updates within ≤ 50ms via optimistic mutation.
  - Collision detection prevents overwriting an existing folder 100% of the time (pre-check inside `OperationLock` scope).
  - Rapid toggle spam (5 clicks in < 1s) results in the correct final state with no intermediate corruption.

---

## 2. User Experience & Functionality

### User Stories

#### US-20.1: Toggle Individual Mod

As a user, I want to enable or disable a mod with one click on a toggle, so that I can control which mods are active without using the file manager.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                     |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-20.1.1 | ✅ Positive | Given `DISABLED MyMod` (disabled), when I click the toggle to enable, then the folder is renamed to `MyMod` on disk and the card shows "enabled" within ≤ 16ms (optimistic)                                                                                  |
| AC-20.1.2 | ✅ Positive | Given `MyMod` (enabled), when I click the toggle to disable, then the folder is renamed to `DISABLED MyMod` and the card shows "disabled" within ≤ 16ms (optimistic)                                                                                         |
| AC-20.1.3 | ✅ Positive | Given a successful toggle, then the object's `enabled_count` in the objectlist badge updates within ≤ 50ms via the same optimistic batch update                                                                                                                 |
| AC-20.1.4 | ❌ Negative | Given `OperationLock` is already held by another operation (in-flight rename or bulk toggle), when the toggle fires, then it awaits the lock with a timeout of 3s; if not acquired, returns "Operation in progress" toast and reverts the optimistic UI      |
| AC-20.1.5 | ⚠️ Edge     | Given rapid toggle spam (≥ 5 clicks in < 1s), then IPC calls are debounced on the frontend (the last click's target state is the intended state); the backend serializes via `OperationLock` — no intermediate state leaves a partially-prefixed folder name |

---

#### US-20.2: Handle Toggle Collisions

As a system, I want to detect if a toggle would rename into an already-existing folder, so that I can prompt the user to resolve the conflict instead of silently overwriting data.

| ID        | Type        | Criteria                                                                                                                                                                                                                                 |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-20.2.1 | ❌ Negative | Given toggling `DISABLED MyMod` to enabled but `MyMod` already exists, then `toggle_mod` returns `CommandError::Conflict { existing_path }` — no rename is executed, no data is overwritten                                              |
| AC-20.2.2 | ✅ Positive | Given a `Conflict` error, then the `ConflictResolveDialog` appears with three choices: "Skip" (leave as disabled), "Rename target" (rename existing to `MyMod_conflict`), or "Overwrite" (move conflicting folder to Trash, then rename) |
| AC-20.2.3 | ⚠️ Edge     | Given the user chooses "Overwrite" but the Trash is unavailable, then the Overwrite is blocked and an additional error toast shows "Cannot overwrite — Trash unavailable" — the original mod stays disabled                              |

---

### Non-Goals

- Toggle does not create a new folder — it only renames the existing one.
- Toggle is not reversible via an undo stack; Trash handles the Overwrite path.
- `DISABLED ` prefix is the only enable/disable mechanism — no DB-only state flag.
- No batch toggle via this command; bulk toggling is Epic 14.

---

## 3. Technical Specifications

### Architecture Overview

```
Frontend:
  FolderCard toggle switch → onClick → useFolderGridActions.toggleMod(folderPath, targetState)
    ├── onMutate: queryClient.setQueryData(['folders', gameId, subPath], optimisticUpdater)
    │             + queryClient.setQueryData(['objects', gameId], countUpdater)
    ├── invoke('toggle_mod', { game_id, folder_path, enable: targetState })
    ├── onSuccess: queryClient.invalidateQueries(['folders', ...]) + ['objects', ...]
    └── onError:  queryClient.setQueryData(['folders', ...], previousSnapshot)  [rollback]

Backend toggle_mod(game_id, folder_path, enable) → Result<(), CommandError>:
  1. Resolve and validate folder_path (canonicalize + starts_with mods_path)
  2. Acquire OperationLock(game_id).await (timeout: 3s)
  3. Activate WatcherSuppression(folder_path)
  4. Compute new_path = if enable: strip "DISABLED " prefix else prepend "DISABLED "
  5. Check new_path.exists() → if true: return Err(CommandError::Conflict { existing_path: new_path })
  6. fs::rename(folder_path, new_path) → map OS error to CommandError::IoError
  7. Drop SuppressionGuard + OperationLock (RAII)
  8. Return Ok(())
```

### Integration Points

| Component           | Detail                                                                                           |
| ------------------- | ------------------------------------------------------------------------------------------------ |
| Command             | `mod_core_cmds.rs::toggle_mod`                                                                   |
| OperationLock       | `Arc<Mutex<()>>` per `game_id` — shared with rename, delete, bulk ops                            |
| WatcherSuppression  | `watcher.rs::SuppressionGuard` — suppresses events for `folder_path` and `new_path`              |
| Optimistic Update   | `useFolderGridActions.ts::toggleMod` — `onMutate` patches `['folders']` and `['objects']` caches |
| Conflict Resolution | `ConflictResolveDialog.tsx` — shown when `CommandError::Conflict` is returned                    |

### Security & Privacy

- **Path validation**: `folder_path` is `canonicalize()`d and checked `starts_with(mods_path)` before lock acquisition — no path traversal.
- **Collision check is inside `OperationLock` scope** — checked and acted upon atomically; no TOCTOU window between check and rename.
- **`WatcherSuppression` covers both `folder_path` and `new_path`** — prevents the watcher from triggering a grid re-fetch for either the old or new name during the rename.

---

## 4. Dependencies

- **Blocked by**: Epic 11 (Folder Listing — `folder_path` semantics), Epic 12 (Folder Grid — toggle UI), Epic 13 (Core Mod Ops — shared OperationLock), Epic 28 (File Watcher — WatcherSuppression).
- **Blocks**: Epic 24 (Conflict Resolution — dialog triggered by toggle collision), Epic 14 (Bulk Operations — calls `toggle_mod_inner` for each item).
