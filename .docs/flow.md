# EMMM — Data Flow Architecture

> Single source of truth: **Disk**. DB is a fast index/projection. UI reads from both, but runtime freshness and canonical matching are different domains.

---

## 1. Core Data Model & Hierarchy

**CRITICAL RULE:** **Objects** (Top-Level) and **Mods** (Depth 1-5) are physically independent. Objects can be disabled, but doing so does NOT recurse to disable its mods. Collections only care about what is ENABLED.

```text
┌────────────────────────────────────────────────────────────┐
│  DISK (Source of Truth)                                    │
│  mods_path/                                                │
│    ├── Raiden/                      (Object Folder)        │
│    │   ├── Skin_A/                  (Mod - Depth 1)        │
│    │   │   ├── Variants/            (Container Folder)     │
│    │   │   │   └── Mod_C/           (Mod - Depth 3)        │
│    │   └── DISABLED Skin_B/         (Mod - Depth 1)        │
│    └── DISABLED Acheron/            (Disabled Object)      │
└────────────────────────────────────────────────────────────┘
         │                            ▲
         │ startup sync & lazy check  │ toggle / apply_collection
         │ (fs validation)            │ (fs::rename prefix)
         ▼                            │
┌────────────────────────────────────────────────────────────┐
│  SQLite DB (Fast Index)                                    │
│                                                            │
│  objects: { id, game_id, name, folder_path, status, object_type, is_pinned, ... }
│  mods: id, game_id, object_id, folder_path,                │
│        status, is_safe, disabled_reason (NULL/USER/SYSTEM/COLLECTION) │
│                                                            │
│  collections: id, game_id, name, is_safe_context,          │
│               is_unsaved, last_active                      │
│  collection_mods: collection_id, mod_id, mod_path, object_id │
│  collection_objects: collection_id, object_id, status      │
│                                                            │
│  tasks (NEW): id, type, payload (JSON), status             │
└────────────────────────────────────────────────────────────┘
```

---

## 2\. Domain Boundary: Disk Reconcile vs Deep Match Scanner

This distinction is mandatory. These two flows must never be treated as the same system.

Legacy note:

- `match_object_id` is legacy schema terminology.
- Canonical relation fields that matter now are:
  - `objects.matched_entry_key`
  - `objects.matched_alias_name`
  - `import_jobs.match_entry_key`
  - `import_jobs.match_alias_name`

### A. Disk Reconcile

Purpose:

- Synchronize the DB projection and UI with the current filesystem reality.
- Keep `ObjectList`, `FolderGrid`, collections, keyviewer, and runtime selection state aligned with add/remove/rename/move/enable/disable/modified changes on disk.

Triggers:

- watcher batch
- window refocus
- entering Mods view
- onboarding final runtime refresh after saving games
- manual runtime repair

Rules:

- source of truth is always the filesystem
- uses `reconcile_disk_state_cmd`
- must not perform canonical object matching
- onboarding must stop at Disk Reconcile; it must not silently trigger Deep Match Scanner
- newly discovered runtime items stay as runtime/disk truth and default to `Other` until explicit matching/import happens

Runtime ownership matrix:

| Trigger                                                                 | Owner                               | Collections Dirty        | KeyViewer Refresh        | Emits `disk_reconcile:result` |
| ----------------------------------------------------------------------- | ----------------------------------- | ------------------------ | ------------------------ | ----------------------------- |
| watcher batch / external filesystem                                     | Disk Reconcile                      | Yes                      | Yes                      | Yes                           |
| window refocus / first Mods entry / game switch hydrate / manual repair | Disk Reconcile                      | Yes when runtime changed | Yes when runtime changed | Yes                           |
| explicit toggle / rename / move / delete mod                            | explicit runtime service            | Yes                      | Yes                      | No                            |
| internal `info.json` / `.ini` mutation                                  | Disk Reconcile (`InternalMutation`) | Yes                      | Yes                      | Yes                           |
| thumbnail-only mutation                                                 | Disk Reconcile (`InternalMutation`) | No                       | No                       | Yes                           |
| object focus / folder navigation                                        | Workspace state only                | No                       | No                       | No                            |

### B. Deep Match Scanner

Purpose:

- Run explicit scan/import flows that try to match discovered folders to canonical MasterDB objects.
- Persist canonical relation and alias metadata without renaming or moving the physical object folder identity.
- Produce preview/review/commit behavior for user-approved matching.

Triggers:

- explicit scan now
- scan review modal
- import pipeline
- explicit user-driven matching flows

Rules:

- uses `deepmatch_preview_cmd` and `deepmatch_scanner_cmd`
- may read MasterDB and assign canonical relations and alias metadata
- must keep physical identity intact:
  - `objects.name` = physical folder name
  - `objects.folder_path` = physical folder path
  - canonical relation is stored separately via `matched_entry_key` and `matched_alias_name`
- must not be called from watcher, window focus, or silent runtime refresh
- browser import, auto organize, and archive/folder import review must all converge on this same preview/commit domain
- if browser import has a canonical suggestion but no physical object target yet, the system creates a physical object shell from the imported folder's physical name and stores canonical relation on that shell

### C. Practical Rule

If the question is:

- "What is true on disk right now?" -> use **Disk Reconcile**
- "Which canonical object does this folder belong to?" -> use **Deep Match Scanner**

---

## 3\. Boot Sequence & Task Recovery Flow

Plaintext

```
User opens app
  → [Backend Startup Check]
      1. Boot Guard: Read last active Safe Mode state.
         IF Unsafe Mode AND PIN is set:
           → Emit `LOCK_UI` event.
           → React mounts `PinEntryModal` and blurs Workspace.
      2. Task Recovery: Check `tasks` table for `status = 'PENDING'`.
         IF found (crash occurred during previous mass rename):
           → Emit `RECOVERY_REQUIRED` event.
           → React halts grid render, shows "Recovery Action" dialog (Resume / Rollback).
      3. Lazy Sync: Check `mods_path` mtime. Run startup reconcile / GC for lost objects and mods.
```

---

## 4\. ObjectList & Preview Panel Flow

Plaintext

```
Frontend: useWorkspaceViewModel() → commands.getWorkspaceViewModel({ input })
  → Backend projects ObjectList, FolderGrid, and Preview semantics from one runtime read-model
  → Safe Mode filter applied in SQL:
      - Safe Mode ON  = Counts ONLY Safe mods.
      - Safe Mode OFF = Counts ONLY Unsafe mods.
  → RETURN PAYLOAD: includes object rows, explorer nodes, preview selection, and runtime metadata.

Frontend UI:
  → ObjectList ALWAYS shows all Objects (no objects are hidden by Safe Mode).
  → Disk Reconcile refreshes the projection before the runtime refresh bus publishes scoped invalidation.
  → Preview Panel reads its selected node and summary from WorkspaceViewModel, then fetches only heavy details separately.
```

---

## 5\. Safe Mode Toggle Flow (Corridor Handoff)

Plaintext

```
User toggles Safe Mode (Shield icon)
  → Frontend: Call `switch_mode(target_safe_mode)`
  → Backend:
      1. Acquire OperationLock.
      2. Track Task: Insert 'SWITCH_MODE' into `tasks` table as 'PENDING'.

      3. Disable Leaving Corridor:
         - Find ENABLED `mods` where `is_safe != target_safe_mode`.
         - Batch fs::rename (prepend "DISABLED " to mod folders up to Depth 5).
           (NOTE: NEVER rename top-level Object folders).
         - Update DB: `status = 'DISABLED'`, `disabled_reason = 'SYSTEM'`.

      4. Restore Target Corridor (Memory):
         - Find collection where `is_safe_context == target_safe_mode` AND `last_active == true`.
         - IF FOUND: Execute `apply_collection(collection_id)`.
         - IF NONE: Fallback to manual enable WHERE `disabled_reason == 'SYSTEM'`.

      5. Complete Task: Update `tasks` to 'COMPLETED'.
      6. Return `restored_collection_id` to Frontend.

  → Frontend:
      - Zustand: `activeCollectionId = restored_collection_id`.
      - Topbar automatically syncs to the restored collection.
      - Invalidate queries: ['objects'], ['mod-folders'], ['collections'].
```

---

## 6\. Virtual Collections Flow

Collections are DB-backed loadouts explicitly isolated by `is_safe_context`. They rely on **Exclusive Swaps** and **Pre-Apply Validation**.

### A. Dirty State (The "Unsaved" Collection)

Triggered by: (1) Manual Mod Toggle in UI, or (2) FileWatcher external edit.

Plaintext

```
  → commands.handleDirtyState()
  → Upsert ONE `is_unsaved = true` collection for the active corridor.
  → Name it using Timestamp format: `YYYYMMDDXXXX`.
  → Snapshot currently ENABLED objects and mods into it.
  → Set `last_active = true`.
  → Frontend Topbar updates to this name with an "Unsaved *" badge.
```

### B. Save Collection

Plaintext

```
  → commands.saveCollection()
  → Read current active state / Unsaved Collection.
  → Disk Validation: Check `Path::exists()` for all mods.
  → Fails if 0 active mods.
  → Insert permanent collection, DELETE the Unsaved Collection record.
```

### C. Apply Collection (With Validation)

Plaintext

```
  → commands.applyCollection({ collection_id, ignore_missing: false })
  → 1. PRE-APPLY VALIDATION: Check physical paths.
       - IF files missing AND !ignore_missing: Return `MissingModsError`.
       - React intercepts → Shows Dialog: "Mods missing. Skip or Cancel?"
       - IF Skip: re-trigger with `ignore_missing = true`.

  → 2. Exclusive Swap (OperationLock + Task PENDING):
       - Diff calc: target mods vs currently active mods in the same corridor.
       - Enable targets (fs::rename), set `disabled_reason = NULL`.
       - Disable non-targets (fs::rename), set `disabled_reason = 'COLLECTION'`.
       - (Multiple mods per object are 100% allowed; warnings bypassed).

  → 3. Finalize:
       - Set `last_active = true` for this collection.
       - Update Task COMPLETED. Return `collection_id`.
```

### D. Active Collection Deletion

Plaintext

```
  → User deletes the active collection via UI.
  → Backend DOES NOT touch the filesystem (mods stay enabled).
  → Creates a new `is_unsaved = true` collection snapshot immediately.
  → Sets it as active in Topbar.
```

---

## 7\. Runtime Refresh & File Watcher Flow

Plaintext

```
External change detected (Create/Remove/Rename/Move/Modified) OR window refocus OR Mods entry OR manual runtime repair OR internal preview/file save.
  → [Backend]
      1. Run Disk Reconcile (`reconcile_disk_state_cmd`) to refresh the filesystem projection.
      2. Detect add/remove/rename/move/enable/disable/modified state from disk.
      3. Trigger `handle_mod_moved_or_renamed(mod_id, new_path, new_object_id)` when path healing is needed.
      4. CROSS-COLLECTION CASCADE:
         `UPDATE collection_mods SET mod_path = ?, object_id = ? WHERE mod_id = ?`
         (This ensures saved collections never break when a user reorganizes folders).
      5. If active mods changed state/count, trigger `handle_dirty_state`.
      6. If runtime files changed (`.ini`, `info.json`, thumbnail), trigger side effects as needed.
      7. Internal preview/file writes use `DiskReconcileReason::InternalMutation` under watcher suppression to avoid watcher conflicts.
      8. Thumbnail-only changes refresh thumbnail caches but must not mark collections dirty by themselves.
      9. Browser import placement also triggers scoped Disk Reconcile immediately after moving files so UI/DB projection do not wait on watcher delivery.

  → [Frontend]
      Publish runtime descriptor / refresh scopes:
      - `workspaceChanged`
      - `objectRowsChanged`
      - `folderTreeChanged`
      - `folderMetadataChanged`
      - `previewChanged`
      - `thumbnailChanged`
      - `conflictsChanged`
      - `corridorChanged`

  → [User Feedback]
      - External object/mod folder changes show a batched toast.
      - InternalMutation and thumbnail-only refreshes stay silent.

  → [Hard Rule]
      Deep Match Scanner is NOT part of this flow.
      Watcher/focus/mods-entry must never call `deepmatch_preview_cmd` or `deepmatch_scanner_cmd`.
```

---

## 8\. Deep Match Scanner Flow

Plaintext

```
User explicitly starts Scan / Import / Review flow
  → [Frontend]
      1. Call `deepmatch_preview_cmd` for preview/review flows.
      2. Call `deepmatch_scanner_cmd` for explicit scan + canonical import flows.

  → [Backend]
      1. Scan candidate folders and extract signals.
      2. Consult MasterDB / matching pipeline.
      3. Produce proposed canonical object matches.
      4. Open review/confirm path when needed.
      5. Commit approved canonical assignments into DB.

  → [Hard Rule]
      This flow is explicit and user-driven.
      It must not be used as a silent substitute for runtime filesystem freshness.
```

---

## 9\. Query Key Strategy

| Hook / Read Model       | Key                                                              | Refreshed By                       |
| ----------------------- | ---------------------------------------------------------------- | ---------------------------------- |
| `useWorkspaceViewModel` | `workspaceKeys.detail({ gameId, safeMode, filters, selection })` | Runtime descriptor refresh bus     |
| `useCollections`        | `['v2-collections', gameId, safeMode]`                           | CRUD, dirty state, Disk Reconcile  |
| `useDashboardStats`     | `['dashboard-stats', safeMode]`                                  | Disk Reconcile, collection changes |

Export to Sheets

**Rules:**

- `safeMode` MUST be in `useCollections` query key to prevent cross-corridor leakage.
- Mode switch refreshes runtime scopes globally.
- Dashboard quick-read stats are projection data and must refresh whenever Disk Reconcile detects object/mod/runtime-file changes.

---

## 10\. Zustand Store Contract

TypeScript

```
interface AppState {
  // Game & Global
  activeGameId: string | null;
  safeMode: boolean;

  // UI Selection
  selectedObject: string | null;
  selectedObjectFolderPath: string | null;

  // Topbar Collection Sync
  activeCollectionId: string | null;

  // ... (Grid views, Search)
}
```

**Key rules:**

- `activeCollectionId` strictly drives the Topbar Dropdown value.
- When `switch_mode` or `apply_collection` succeeds, they return the new `collection_id`, which Zustand immediately sets to keep the Topbar in perfect sync.
- `setSafeMode` clears corridor-sensitive workspace selection (`selectedObjectFolderPath`, `selectedModPath`, grid selection, and explorer path) to prevent orphaned grid/preview state from leaking across corridors.

---

## 11\. Database Invariants

> **NEVER** use `INSERT OR REPLACE INTO games` — SQLite implements it as DELETE + INSERT, which triggers `ON DELETE CASCADE` on `objects`, `mods`, and other child tables, permanently wiping all foreign-key-linked data. Always use `INSERT ... ON CONFLICT(id) DO UPDATE SET`.

---

## 12\. In-Game Overlay Synchronization Flow (3DMigoto Bridge)

EMMM maintains a set of runtime artifacts in `.emmm_data/` that are consumed by 3DMigoto to provide in-game feedback.

### A. Synchronization Triggers

Any state change that affects the active mod set or character matching triggers `trigger_overlay_refresh()`:

1. **Manual Toggles**: Toggling a mod or character in the UI.
2. **Collection Switch**: Applying a new preset or switching corridors.
3. **FS Watcher**: Renaming or moving folders in Windows Explorer.
4. **Hotkeys**: Toggling Safe Mode or cycling presets in-game.

### B. The Generation Pipeline

Plaintext

```
[Event Trigger]
  → Acquire Global OperationLock
  → Zero-Leak Phase: fs::remove_dir_all(".emmm_data/keybinds/active")
  → Harvest Phase: Scan enabled mods for hashes & keybinds
  → Match Phase: Score hashes against the Resource Pack matcher used by KeyViewer
  → Write Phase:
      - Generate KeyViewer.ini (3DMigoto bridge)
      - Generate {hash}.txt per character (grouped by mod source)
      - Generate runtime_status.txt (Global banner)
  → Notification: Send `reload_fixes` keystroke to game window (Enigo)
```

### C. Zero-Leak Policy

To prevent stale character info from appearing after a mod is disabled, the `keybinds/active/` directory is **force-cleared** before every regeneration. This ensures that 3DMigoto only ever sees the current, valid mod set.
