# Epic 13: Core Mod Operations (Toggle / Rename / Delete)

## 1. Executive Summary

- **Problem Statement**: The three most frequent user actions on individual mods — toggle, rename, delete — must feel instant (optimistic UI), be safe (suppress file watcher, hold `OperationLock`), and handle filesystem edge cases (locks, collisions, permission failures) without corrupting any app state.
- **Proposed Solution**: A shared runtime action engine that routes toggle through the public workspace switch command (`execute_workspace_switch`) and delegates physical enabled/disabled folder mutation to the same runtime mutation engine used by Collections and Safe Mode switch. Rename/delete remain dedicated backend commands, with `OperationLock` + ref-counted `WatcherSuppression`, structured errors, targeted Disk Reconcile/projection refresh completion, and optimistic runtime descriptors that roll back on failure.
- **Success Criteria**:
  - [x] Toggle UI optimistic update applies in ≤ 16ms (one frame); backend confirms within ≤ 300ms on SSD.
  - [x] Rename completes (disk + metadata sync) in ≤ 500ms for a flat mod folder.
  - [x] Delete (move to Internal Trash) completes in ≤ 500ms; decrements objectlist object counts immediately via optimistic update.
  - [x] 0 watcher-triggered re-fetches caused by app's own toggle/rename/delete operations (WatcherSuppression enforced).
  - [x] Rapid toggle spam is serialized via `OperationLock`.

---

## 2. User Experience & Functionality

### User Stories

#### US-13.1: Toggle Mod State

As a user, I want to enable or disable a mod with a single click, so that I can test different combinations without manually renaming folders.

| ID        | Type        | Criteria                                                                                                                                                                                                                                  |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-13.1.1 | ✅ Positive | Given a disabled mod, when I click its toggle, then the optimistic UI update flips the card's enabled state in ≤ 16ms; the backend completes the `DISABLED ` prefix rename on disk within ≤ 300ms                                         |
| AC-13.1.2 | ✅ Positive | Given a slow HDD, when I toggle a mod, the UI toggle animates immediately (optimistic) while filesystem IO runs in the background — the user sees no lag on the toggle switch                                                             |
| AC-13.1.3 | ❌ Negative | Given the folder is locked by an external process (e.g., the game engine is reading it), when toggled, then the `rename` syscall fails; the UI rolls back the optimistic toggle and shows a "Folder locked — cannot toggle" toast         |
| AC-13.1.4 | ⚠️ Edge     | Given rapid toggle spam (> 3 clicks before the previous `rename` completes), then the backend serializes via `OperationLock` — only the last intended state takes effect; no intermediate partial renames produce a corrupted folder name |
| AC-13.1.5 | ✅ Positive | Given a disabled mod, when I select "Enable Only This" from context menu, then this mod is enabled AND all other currently enabled mods in the same Object are disabled within the same `OperationLock` atomic transaction                |
| AC-13.1.6 | ⚠️ Edge     | Given I enable a mod, and another mod with the same `master_object_id` (e.g., Character) is already enabled, then a Duplicate Warning dialog appears: "A mod for [Character Name] is already active! Force Enable or Cancel?"             |
| AC-13.1.7 | ⚠️ Edge     | Given I enable a mod that has known hash conflicts (detected via `ShaderFixes` or `.ini`), then a non-blocking toast "Shader Collision detected with [Mod Name]" appears — the action is not blocked, just a notice                       |
| AC-13.1.8 | ✅ Positive | Given a successful toggle operation, a success toast includes an "Undo" button (5s timeout); clicking it reverts the state via a new backend command call without needing to manually toggle it back                                      |

---

#### US-13.2: Rename Mod

As a user, I want to rename a mod folder and have the display name update, so that I can organize it clearly.

| ID        | Type        | Criteria                                                                                                                                                                                                                        |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-13.2.1 | ✅ Positive | Given a valid new name (no invalid characters), when rename is submitted, then the physical folder is renamed on disk (preserving `DISABLED ` prefix if applicable) and `info.json` `name` field is updated in ≤ 500ms          |
| AC-13.2.2 | ❌ Negative | Given the new name contains invalid Windows characters (`\ / : * ? " < > \|`), the frontend form rejects the input before IPC call — an inline error shows "Name contains invalid characters"                                   |
| AC-13.2.3 | ❌ Negative | Given the new folder path already exists (collision), when `rename_mod` is called, then the backend returns `CollisionError` and a `ConflictResolveDialog` appears — no partial rename occurs                                   |
| AC-13.2.4 | ⚠️ Edge     | Given a folder with a deeply nested mod asset path where renaming would exceed the Windows 260-char path limit, then the backend validates the resulting path length before renaming and returns `PathTooLongError` if exceeded |

---

#### US-13.3: Delete Mod (Move to Trash)

As a user, I want to delete a mod by moving it to the OS Trash, so that I can recover it if I change my mind — without permanent data loss.

| ID        | Type        | Criteria                                                                                                                                                                                                        |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-13.3.1 | ✅ Positive | Given I select "Delete" from the context menu and confirm in the dialog, then the mod folder is moved to the Internal App Trash (`app_data/trash/{uuid}/`) within ≤ 500ms.                                      |
| AC-13.3.2 | ✅ Positive | Given a successful delete of an enabled mod, then the parent object's `enabled_count` decrements optimistically in the objectlist and the card disappears from the grid in ≤ 100ms.                             |
| AC-13.3.3 | ❌ Negative | Given a folder where some nested files are locked by permissions, when delete is attempted, then the backend returns a `PartialDeleteError` listing affected paths — the original folder is NOT moved to Trash. |
| AC-13.3.4 | ✅ Positive | Given I initiate a delete, the system runs `pre_delete_check` to report folder item count; if >0, a confirmation dialog warns "Folder contains N items. Delete to trash?" before proceeding.                    |

---

### Non-Goals

- No deep "Undo" stack for rename/delete — Trash handles recovery for delete; rename is a deliberate manual action. However, a 5s Undo toast **is** provided for Toggle operations.
- No mod duplication (Clone) in this epic.
- `WatcherSuppression` is an internal implementation detail — not user-visible.
- `OperationLock` is per-game-path — concurrent operations on different games are not blocked.

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Backend Service: Mods / Workspace Runtime

execute_workspace_switch(input):
  1. Acquire OperationLock(game_id) + Suppress Watcher for affected paths.
  2. Compute target path / duplicate strategy / parent-enable resolution.
  3. Validate current corridor parity, path traversal, missing source path, and target collision before disk mutation.
  4. Delegate enabled/disabled filesystem rename and DB projection (`status`, `folder_path`, `folder_path_key`, `disabled_reason`) to `runtime_mutation_engine`.
  5. Finish through targeted Disk Reconcile or the shared runtime projection effect path.
  6. Return structured switch result + WorkspaceImpact.

rename_mod(old_path, new_name):
  1. Acquire OperationLock + Suppress Watcher.
  2. Compute `new_path` (preserving disabled prefix semantics).
  3. fs::rename + update metadata name fields.
  4. Return payload + WorkspaceImpact for workspace/object/preview refresh.

delete_mod(path):
  1. Acquire OperationLock + Suppress Watcher.
  2. Move folder to `./app_data/trash/{uuid}/`.
  3. Persist trash metadata.
  4. Remove live record and return payload + WorkspaceImpact.
```

### Integration Points

| Component                 | Detail                                                                                                                           |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `OperationLock`           | Shared mutex per game workspace; serializes all filesystem mutations.                                                            |
| `Trash Service`           | App-level soft delete; supports `restore_from_trash` with context parity checks.                                                 |
| `Watcher Guard`           | `SuppressionGuard` blocks event cycles during bulk or sensitive moves.                                                           |
| `Workspace Switch Engine` | Frontend toggle path goes through `execute_workspace_switch(...)` and maps `WorkspaceImpact` into runtime effects.               |
| `Runtime Mutation Engine` | Single backend boundary for `DISABLED ` prefix changes used by manual toggle, collection apply, and Safe/Unsafe corridor switch. |
| `Runtime Descriptor`      | Optimistic/cache updates and refresh publish are centralized; feature code does not call raw `invalidateQueries`.                |
| Disk Reconcile            | Internal filesystem mutations suppress watcher noise, then complete through one intentional runtime refresh path.                |

### Security & Privacy

- **Path Isolation**: `is_path_safe` prevents traversal outside the game's designated mod directory.
- **Context Parity**: Trash restore is blocked if the target game context has changed since deletion.
- **Atomic Renames**: DB updates for children are part of the same service transaction as the folder rename.
- **Cross-Drive Handling**: `rename_cross_drive_fallback` ensures reliability across different physical disks/partitions.

---

## 4. Dependencies

- **Blocked by**: Epic 11 (Folder Listing — `folder_path` semantics), Epic 12 (Folder Grid — UI toggle/rename/delete entry points), Epic 28 (File Watcher — `WatcherSuppression` API).
- **Blocks**: Epic 14 (Bulk Operations — calls these same commands in batch), Epic 15 (Explorer Interactions — context menu triggers).
