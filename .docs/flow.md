# EMMM — Data Flow Architecture

> Single source of truth: **Disk**. DB is a fast index. UI reads from both.

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

## 2\. Boot Sequence & Task Recovery Flow

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
      3. Lazy Sync: Check `mods_path` mtime. GC lost objects/mods, upsert new.
```

---

## 3\. ObjectList & Preview Panel Flow

Plaintext

```
Frontend: useObjects(game_id, filters) → commands.getObjectsCmd({ game_id, filters })
  → Backend queries `objects` LEFT JOIN `mods`
  → Safe Mode filter applied in SQL:
      - Safe Mode ON  = Counts ONLY Safe mods.
      - Safe Mode OFF = Counts ONLY Unsafe mods.
  → RETURN PAYLOAD: MUST include `active_mod_paths: string[]` per Object.

Frontend UI:
  → ObjectList ALWAYS shows all Objects (no objects are hidden by Safe Mode).
  → Preview Panel reads `active_mod_paths` from the selected Object to accurately
    highlight/identify exactly which mods are active (prevents multi-mod desync).
```

---

## 4\. Safe Mode Toggle Flow (Corridor Handoff)

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

## 5\. Virtual Collections Flow

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

## 6\. Auto-Healing & File Watcher Flow

Plaintext

```
External change detected (Create/Remove/Rename/Move) OR Manual 'move_mod' invoked.
  → [Backend]
      1. Trigger `handle_mod_moved_or_renamed(mod_id, new_path, new_object_id)`.
      2. CROSS-COLLECTION CASCADE:
         `UPDATE collection_mods SET mod_path = ?, object_id = ? WHERE mod_id = ?`
         (This ensures saved collections never break when a user reorganizes folders).
      3. If active mods changed state/count, trigger `handle_dirty_state`.

  → [Frontend]
      Invalidate TanStack: ['objects'], ['mod-folders'], ['collections']
```

---

## 7\. Query Key Strategy

| Hook             | Key                                                                   | Invalidated By             |
| ---------------- | --------------------------------------------------------------------- | -------------------------- |
| `useObjects`     | `['objects','list', {game_id, safe_mode, object_type, sort, search}]` | watcher, sync, mode switch |
| `useModFolders`  | `['mod-folders', modsPath, subPath, safeMode]`                        | watcher, toggle, navigate  |
| `useCollections` | `['collections', gameId, safeMode]`                                   | CRUD, dirty state, switch  |

Export to Sheets

**Rules:**

- `safeMode` MUST be in `useCollections` query key to prevent cross-corridor leakage.
- Mode switch invalidates all keys globally.

---

## 8\. Zustand Store Contract

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
- `setSafeMode` clears `selectedObjectFolderPath` to prevent rendering orphaned grids.

---

## 9\. Database Invariants

> **NEVER** use `INSERT OR REPLACE INTO games` — SQLite implements it as DELETE + INSERT, which triggers `ON DELETE CASCADE` on `objects`, `mods`, and other child tables, permanently wiping all foreign-key-linked data. Always use `INSERT ... ON CONFLICT(id) DO UPDATE SET`.

---

## 10\. In-Game Overlay Synchronization Flow (3DMigoto Bridge)

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
  → Match Phase: Score hashes against Resource Pack (Deep Matcher logic)
  → Write Phase:
      - Generate KeyViewer.ini (3DMigoto bridge)
      - Generate {hash}.txt per character (grouped by mod source)
      - Generate runtime_status.txt (Global banner)
  → Notification: Send `reload_fixes` keystroke to game window (Enigo)
```

### C. Zero-Leak Policy

To prevent stale character info from appearing after a mod is disabled, the `keybinds/active/` directory is **force-cleared** before every regeneration. This ensures that 3DMigoto only ever sees the current, valid mod set.
