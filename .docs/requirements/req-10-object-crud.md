# Epic 10: Object CRUD Operations

## 1. Executive Summary

- **Problem Statement**: The auto-scanner may fail to map some mod folders to any known game entity — users need to manually create, edit, rename, delete, and pin objects to keep their objectlist organized without touching the filesystem.
- **Proposed Solution**: A full CRUD interface for Object records in the local SQLite DB, with: creation (with name uniqueness enforcement), category edit (with schema validation), deletion (blocked if mods exist), and a pin-to-top flag — all surfaced via objectlist context menus and modals.
- **Success Criteria**:
  - Object creation (DB insert + objectlist refresh) completes in ≤ 300ms from submit.
  - Object rename/category edit reflects in the objectlist in ≤ 200ms (optimistic update before DB confirm).
  - Delete is blocked 100% of the time when `folder_count > 0` — no orphaned mod folders on disk.
  - Pin/unpin state change reflects in objectlist sort order in ≤ 100ms.
  - Rapid "Submit" spam creates exactly 1 DB record due to UNIQUE constraint + frontend debounce.

---

## 2. User Experience & Functionality

### User Stories

#### US-10.1: Create Custom Object

As a user, I want to manually create a new Object entry, so that I can organize mods the auto-scanner failed to map.

| ID        | Type        | Criteria                                                                                                                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.1.1 | ✅ Positive | Given I fill in the creation form (name, category from schema dropdown), when I submit, then a new Object row is inserted in the DB and appears in the objectlist virtual list in ≤ 300ms                                                            |
| AC-10.1.2 | ✅ Positive | Given successful creation, then the new Object is immediately available as a drop target for mod folders dragged from the grid                                                                                                                    |
| AC-10.1.3 | ❌ Negative | Given an object name that already exists for the same game (case-insensitive), when creating, then a "Duplicate object name" error is shown inline — the backend returns `UNIQUE constraint failed` and the frontend displays it without crashing |
| AC-10.1.4 | ⚠️ Edge     | Given the user clicks "Submit" multiple times rapidly before the first response returns, then the submit button is disabled on the first click (loading state) and exactly 1 DB record is created                                                 |

---

#### US-10.2: Edit Object Properties

As a user, I want to edit an object's name or category, so that I can correct mapping mistakes made by the scanner.

| ID        | Type        | Criteria                                                                                                                                                                                                                                                         |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.2.1 | ✅ Positive | Given the edit modal, when I change the category (e.g., Weapon → Character) and save, then the DB record is updated and the object visually moves to its new category section in ≤ 200ms                                                                         |
| AC-10.2.2 | ✅ Positive | Given I change only the display name, when saved, then the objectlist row updates via optimistic mutation — no full list refetch required                                                                                                                           |
| AC-10.2.3 | ✅ Positive | Given I switch to the "Auto-Sync" tab, I can search MasterDB entries; selecting one auto-populates the form's name, category, structured metadata, and thumbnail URL for review before saving                                                                    |
| AC-10.2.4 | ❌ Negative | Given I edit the category to a value no longer present in the active schema (e.g., tampered request), then the backend rejects with a `SchemaValidationError` — the object's category is not changed                                                             |
| AC-10.2.5 | ⚠️ Edge     | Given I rename an object to precisely match an existing object's name, then the form shows a warning "Another object has this name — this may cause confusion" but does not block submission (names are not globally unique by enforced constraint, only warned) |

---

#### US-10.3: Delete Object

As a user, I want to delete empty Objects from the objectlist, so that the list stays uncluttered.

| ID        | Type        | Criteria                                                                                                                                                                                                                        |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.3.1 | ✅ Positive | Given an object with `folder_count = 0`, when I select "Delete" from context menu and confirm, then the DB record is deleted and the row disappears from the objectlist in ≤ 200ms                                                 |
| AC-10.3.2 | ❌ Negative | Given an object that still has ≥ 1 linked mod folders (FK constraint), when delete is attempted, then the backend returns `ObjectHasModsError` and the user sees "Move or delete mods first" — no DB row is deleted             |
| AC-10.3.3 | ⚠️ Edge     | Given a scanner background thread populates the object with a new mod folder in the same instant the user clicks delete, then the DB transaction detects the FK violation and blocks the delete — no orphaned folder is created |

---

#### US-10.4: Pin Object

As a user, I want to pin frequently used objects to the top of their category, so that my most-used characters are instantly accessible without scrolling.

| ID        | Type        | Criteria                                                                                                                                                                                          |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.4.1 | ✅ Positive | Given an unpinned object, when I click "Pin" from the context menu, then `is_pinned = true` is written to DB and the object sorts to the top of its category section in ≤ 100ms                   |
| AC-10.4.2 | ✅ Positive | Given a pinned object, when unpinned, it immediately drops back into alphabetical order within its category                                                                                       |
| AC-10.4.3 | ❌ Negative | Given a non-existent or stale `object_id` in the pin request (e.g., deleted by another session), then the backend returns `NotFound` and the objectlist refreshes its cache — no unhandled exception |
| AC-10.4.4 | ⚠️ Edge     | Given ≥ 50 pinned objects in a single category, they sort alphabetically amongst themselves at the top tier — stable sort, no random order                                                        |

---

#### US-10.5: Single-Object Database Sync

As a user, I want to sync a single object's metadata with the MasterDB via the context menu, so that I can automatically pull in correct tags, element, and rarity without manually editing the form.

| ID        | Type        | Criteria                                                                                                                                                                                 |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-10.5.1 | ✅ Positive | Given the context menu, when I click "Sync with DB", then `match_object_with_db` runs via the staged quick matcher pipeline and opens the `SyncConfirmModal` showing a diff preview      |
| AC-10.5.2 | ✅ Positive | Given the `SyncConfirmModal`, when I click Apply, then the DB object logic updates name, category, and metadata atomically and invalidates the cache                                     |
| AC-10.5.3 | ❌ Negative | Given `match_object_with_db` finds no confident match, then a toast warns "No matched DB entry found" and asks if the user wants to open Manual Edit instead                             |
| AC-10.5.4 | ⚠️ Edge     | Given the object already perfectly matches the MasterDB data, the modal still shows the diff (which will be empty/identical) allowing the user to confirm or cancel without side-effects |

---

### Non-Goals

- No bulk object deletion via multi-select in this phase.
- No custom object icons or avatar uploads.
- Objects are DB-only entities — creating an object does NOT create a corresponding directory on disk; folders are mapped to objects via the `folder_path` FK.
- No object merging (combining two object records into one).

---

## 3. Technical Specifications

### Architecture Overview

```
Backend (commands/objects/)
  ├── create_object(game_id, name, category_id) → ObjectRecord
  │   └── INSERT INTO objects ... (UNIQUE constraint on game_id + lower(name))
  ├── update_object(object_id, name?, category_id?) → ObjectRecord
  │   └── Validates category_id against loaded GameSchema in memory
  ├── delete_object(object_id) → ()
  │   └── Fails if COUNT(folders WHERE object_id = ?) > 0
  └── pin_object(object_id, is_pinned: bool) → ()

Frontend
  └── useObjectMutations() hook
      ├── createObject → useMutation → invalidate(['objects', gameId])
      ├── updateObject → useMutation + optimistic update
      ├── deleteObject → useMutation → confirm dialog first
      └── pinObject → useMutation + optimistic sort update
```

### Integration Points

| Component         | Detail                                                                                                       |
| ----------------- | ------------------------------------------------------------------------------------------------------------ |
| DB Table          | `objects(id UUID PK, game_id FK, name TEXT, category_id TEXT, is_pinned BOOL, is_safe BOOL, created_at)`     |
| Schema Validation | Category IDs validated against `GameSchema` loaded in `Arc<RwLock<...>>` — no DB query needed                |
| ObjectList Refresh   | All mutations call `queryClient.invalidateQueries(['objects', gameId])` on success                           |
| Delete Guard      | `DELETE FROM objects WHERE id = ? AND NOT EXISTS (SELECT 1 FROM folders WHERE object_id = ?)` — atomic check |
| Optimistic Pin    | `queryClient.setQueryData(['objects', gameId], draft => sortByPin(draft))` on pin toggle                     |

### Security & Privacy

- **`category_id` is validated against the in-memory schema** before any DB write — no arbitrary string can be inserted as a category; must be in the schema's `category_ids` set.
- **Object names are sanitized** (trimmed, max 128 chars) before DB insert — no unbounded input.
- **Delete is blocked by a transactional check** (`EXISTS` subquery) — the block happens inside the same DB transaction, not as a separate check before delete, preventing TOCTOU race.
- **No filesystem operations** in Object CRUD — creating, editing, or deleting an Object never touches disk; all operations are DB-only.

---

## 4. Dependencies

- **Blocked by**: Epic 01 (App Bootstrap — DB pool), Epic 02 (Game Management — `game_id` FK), Epic 09 (Object Schema — category validation).
- **Blocks**: Epic 07 (Object List — displays CRUD results), Epic 25 (Scan Engine — creates objects automatically), Epic 40 (Metadata Actions — pin, favorite).
