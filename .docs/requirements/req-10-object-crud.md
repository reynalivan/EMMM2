# Epic 10: Object CRUD Operations

## 1. Executive Summary

- **Problem Statement**: The Deep Match Scanner may fail to map some mod folders to any known game entity — users need to manually create, edit, rename, delete, and pin objects to keep their object list organized without touching the filesystem.
- **Proposed Solution**: A full CRUD interface for Object records in the local SQLite DB, with: creation (with validation + disk folder creation), category edit (with schema validation), deletion (blocked via server-side FK constraints), and persistent properties (pin, favorite, auto-sync) — all surfaced via `ObjectList` context menus and modals.
- **Success Criteria**:
  - Object creation (DB insert + list refresh) completes in ≤ 300ms from submit.
  - Object rename/category edit reflects in the list in ≤ 200ms via optimistic UI updates.
  - Delete is blocked by the backend when mods exist — ensuring 100% data integrity.
  - Pin visibility updates the sort order immediately (items pinned to top of their sections).
  - Reveal in Explorer accurately highlights the object's root folder on disk.

---

## 2. User Experience & Functionality

### User Stories

#### US-10.1: Create Custom Object

As a user, I want to manually create a new Object entry, so that I can organize mods the auto-scanner failed to map.

| ID        | Type        | Criteria                                                                                                                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.1.1 | ✅ Positive | Given I fill in the creation form (name, category from schema dropdown), when I submit, then a new Object row is inserted in the DB and appears in the objectlist virtual list in ≤ 300ms                                                         |
| AC-10.1.2 | ✅ Positive | Given successful creation, then the new Object is immediately available as a drop target for mod folders dragged from the grid                                                                                                                    |
| AC-10.1.3 | ❌ Negative | Given an object name that already exists for the same game (case-insensitive), when creating, then a "Duplicate object name" error is shown inline — the backend returns `UNIQUE constraint failed` and the frontend displays it without crashing |
| AC-10.1.4 | ⚠️ Edge     | Given the user clicks "Submit" multiple times rapidly before the first response returns, then the submit button is disabled on the first click (loading state) and exactly 1 DB record is created                                                 |

---

#### US-10.2: Edit Object Properties

As a user, I want to edit an object's name or category, so that I can correct mapping mistakes made by the scanner.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                         |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.2.1 | ✅ Positive | Given the edit modal, when I change the category (e.g., Weapon → Character) and save, then the DB record is updated and the object visually moves to its new category section in ≤ 200ms                                                                         |
| AC-10.2.2 | ✅ Positive | Given I change only the display name or thumbnail, when saved, then the objectlist row updates via optimistic mutation — no wait for an unrelated full list refetch                                                                                             |
| AC-10.2.3 | ✅ Positive | Given I switch to the "Auto-Sync" tab, I can search MasterDB entries; selecting one auto-populates the form's name, category, structured metadata, and thumbnail URL for review before saving                                                                    |
| AC-10.2.4 | ❌ Negative | Given I edit the category to a value no longer present in the active schema (e.g., tampered request), then the backend rejects with a `SchemaValidationError` — the object's category is not changed                                                             |
| AC-10.2.5 | ⚠️ Edge     | Given I rename an object to precisely match an existing object's name, then the form shows a warning "Another object has this name — this may cause confusion" but does not block submission (names are not globally unique by enforced constraint, only warned) |

---

#### US-10.3: Delete Object

As a user, I want to delete empty Objects from the objectlist, so that the list stays uncluttered.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                                                 |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.3.1 | ✅ Positive | Given an object with `folder_count = 0`, when I select "Delete" from context menu and confirm, then the DB record is deleted and the row disappears from the objectlist in ≤ 200ms                                                                                                       |
| AC-10.3.2 | ⚠️ Edge     | Given an object that still has ≥ 1 linked mod folders, when delete is attempted without force, then the backend returns `ObjectHasModsError(count)`. The user is prompted with a secondary confirmation dialog ("Yes, I understand delete X mods"). If confirmed, the deletion proceeds. |
| AC-10.3.3 | ⚠️ Edge     | Given a scanner background thread populates the object with a new mod folder in the same instant the user clicks delete, then the DB transaction detects the FK violation and blocks the delete — no orphaned folder is created                                                          |

---

#### US-10.4: Pin & Reveal

As a user, I want to pin frequently used objects and quickly open their folders on disk, so that I can access my most-used content and its files instantly.

| ID        | Type        | Criteria                                                                                                                                                |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.4.1 | ✅ Positive | Given an unpinned object, when I click "Pin", then `is_pinned = true` is written to DB and the object sorts to the top of its section immediately via optimistic UI, with backend reconciliation after                                                  |
| AC-10.4.2 | ✅ Positive | Given a pinned object, when unpinned, it immediately drops back into alphabetical order within its category                                                                                                                              |
| AC-10.4.3 | ✅ Positive | Given the "Reveal in Explorer" action, the OS file explorer opens with the object's root folder selected (folder_path resolution)                       |
| AC-10.4.4 | ❌ Negative | Given a folder that was manually deleted from disk, "Reveal" caught by the backend returns `NotFound` and triggers a cache invalidation to clean the UI |

---

#### US-10.5: Single-Object Database Sync

As a user, I want to sync a single object's metadata with the MasterDB via the context menu, so that I can explicitly run a Deep Match-style metadata alignment without manually editing the form.

| ID        | Type        | Criteria                                                                                                                                                                                 |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.5.1 | ✅ Positive | Given the context menu, when I click "Sync with DB", then `match_object_with_db` runs via the explicit Deep Match Scanner matcher pipeline and opens the `SyncConfirmModal` showing a diff preview |
| AC-10.5.2 | ✅ Positive | Given the `SyncConfirmModal`, when I click Apply, then the DB object logic updates name, category, and metadata atomically and invalidates the cache                                     |
| AC-10.5.3 | ❌ Negative | Given `match_object_with_db` finds no confident match, then a toast warns "No matched DB entry found" and asks if the user wants to open Manual Edit instead                             |
| AC-10.5.4 | ⚠️ Edge     | Given the object already perfectly matches the MasterDB data, the modal still shows the diff (which will be empty/identical) allowing the user to confirm or cancel without side-effects |

---

### Non-Goals

- No destructive bulk object merge or archive workflow in this phase; bulk delete / pin / enable / disable may exist where already implemented in UI.
- No custom object icons or avatar uploads.
- Objects are not DB-only entities in current runtime flow — creating an object also creates the corresponding directory on disk so watcher and UI stay aligned.
- No object merging (combining two object records into one).
- Disk Reconcile does NOT run this feature automatically; MasterDB sync is always user-driven.

---

## 3. Technical Specifications

### Architecture Overview

```rust
// Backend Service: Objects (mutate.rs, query.rs)

create_object(game_id, name, object_type, folder_path?, thumbnail_url?):
  1. Generate UUID.
  2. Resolve mods root path from DB.
  3. Pre-computation: Determine target folder and future thumbnail path (`preview.*`).
  4. DB FIRST: Insert record into `objects` table.
     - Prevents race conditions with Watcher (Watcher finds record, skips "Other" default).
  5. Disk Second: Create physical folder via `std::fs::create_dir_all`.
  6. Copy thumbnail from cache to object folder if provided.

delete_object(id):
  1. Acquire OperationLock + Suppress Watcher.
  2. Fetch object details from DB.
  3. Move physical folder to TRASH (via trash service).
  4. Cascade-delete child mod records from DB.
  5. Delete object record from DB.
```

### Integration Points

| Component          | Detail                                                                                                                               |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| DB Table           | `objects(id UUID PK, game_id FK, name TEXT, folder_path TEXT, category_id TEXT, is_pinned BOOL, metadata JSON, thumbnail_path TEXT)` |
| Schema Validation  | Category IDs validated against `GameSchema` loaded in memory; Unique name check on game_id + lower(name).                            |
| ObjectList Refresh | Mutations use shared object-query refresh helpers; direct object edits and pinning patch ObjectList optimistically, then reconcile with an active refresh. |
| Delete Guard       | Handled via server-side logic: acquires `OperationLock` and moves folder to Trash (Epic 22) + Cascade.                               |
| Watcher Flow       | "DB First" creation ensures Disk Reconcile recognizes the new folder as a tracked object immediately.                                 |

### Security & Privacy

- **Sanitization**: Folder names are normalized and sanitized to prevent path traversal; unique constraints enforced at the DB level.
- **Atomic Deletion**: The whole operation—from trashing the disk folder to DB cleanup—is guarded by an `OperationLock`.
- **Race Condition Prevention**: "DB First" pattern ensures filesystem watcher and UI stay in sync during object birth.
- **Trash Safety**: Deletion is non-destructive initially; folders can be recovered from the `.trash` directory via maintenance tools.
- **Domain Boundary**: Object CRUD mutates the curated object model. Disk Reconcile maintains filesystem truth, while Deep Match Scanner is the only automatic canonical matching flow.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap — DB pool), Epic 02 (Game Management — `game_id` FK), Epic 09 (Object Schema — category validation).
- **Blocks**: Epic 07 (Object List — displays CRUD results), Epic 25 (Scan Engine — creates objects automatically), Epic 40 (Metadata Actions — pin, favorite).
