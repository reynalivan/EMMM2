# Epic 3: Game Switcher & Object List Management (Revised v3.0)

**Focus:** Building a primary sidebar interface that supports dynamic game switching, categorical object management (Character, Weapon, UI, Other), schema-driven filtering, and rich context menu operations — all backed by a flexible database schema and virtualized rendering.

## Dependencies

| Direction    | Epic   | Relationship                                                                   |
| ------------ | ------ | ------------------------------------------------------------------------------ |
| ⬆ Upstream   | Epic 1 | Requires game context from config                                              |
| ⬆ Upstream   | Epic 2 | Requires matched `object_type` from scanning                                   |
| ⬇ Downstream | Epic 4 | Drives grid content via sidebar selection                                      |
| ⬇ Downstream | Epic 5 | Provides category context for operations                                       |
| Reacts to    | Epic 7 | Sidebar filters by `appStore.safeMode` (Zustand). Safe Mode logic owned by E7. |

## Cross-Cutting Requirements

- **State:** Active game stored in `useAppStore` (Zustand). Switching calls `switchGame(id)` via `useGameSwitch`. Cache invalidation handled automatically via TanStack Query key factory (`objectKeys`).
- **Virtualization:** Sidebar list uses `@tanstack/react-virtual` with dynamic `estimateSize` (28px headers, 24px sub-headers, 70px rows / 82px mobile). Overscan: 10 items.
- **Forms:** Object CRUD uses `React Hook Form` + `Zod`. Name required (min 2 chars), Tags optional.
- **schema.json Fallback:** If missing or corrupt, fall back to default Categories: [Character, Weapon, UI, Other]. Log WARN. _(Implemented in `get_game_schema` backend command.)_

---

## Architecture Overview

### Component Hierarchy

```
ObjectList (orchestrator)
├── ObjectListToolbar
│   ├── Search input
│   ├── Filter toggle + badge
│   ├── Refresh button (+ repair orphan mods)
│   ├── Auto Reorganize (full scan & sync)
│   ├── Create New Object button
│   └── FilterPanel (collapsible)
│       ├── Category chips (from schema.json)
│       ├── Sort chips (A-Z, New, ★)
│       ├── Status chips (All/Enabled/Disabled)
│       └── Metadata filter chips (dynamic from schema)
├── ObjectListStates (loading / error / no-game / empty)
├── ObjectListContent (virtualized list)
│   ├── Category Headers (with count badges)
│   ├── Sub-Headers (for "Other" sub_category grouping)
│   └── ObjectRowItem (per object row)
│       ├── Thumbnail (with power-off overlay when disabled)
│       ├── Name + pin icon + mod count badges
│       └── Metadata subtext (element icon, weapon, rarity, gender)
├── ObjectContextMenu (right-click: dual mode for objects vs folders)
├── Status Bar (object count + "Show All" button)
└── ObjectListModals
    ├── Delete confirmation dialog
    ├── EditObjectModal (manual + auto-sync tabs)
    ├── CreateObjectModal
    ├── SyncConfirmModal (single-object DB match)
    └── ScanReviewModal (bulk scan preview + commit)
```

### Data Flow

```
schema.json ──→ get_game_schema (Rust) ──→ useGameSchema (TanStack Query)
                                              │
get_objects_cmd ──→ FS Scan + DB Merge ───→ useObjects ──→ useObjectListLogic
(FS Source of Truth)                                               │
                                                                   │
useSearchWorker (Web Worker) ──→ client-side search ───────────────┘
                                                                   │
useObjectListVirtualizer ←── flatObjectItems (grouped by category) ┘
```

---

## 1. User Stories (Implemented)

### US-3.1: Multi-Category Navigation ✅

**As a** user, **I want to** see a list of objects grouped by their type (Character, Weapon, UI, or other Other categories), **So that** navigation remains organized even if the game has hundreds of various mod types.

- **Acceptance Criteria (All Implemented):**
  - **Category Headers:** The sidebar displays visual headers for each category (e.g., "Characters", "Weapons", "User Interface"), with item count badges. Empty categories are automatically hidden.
  - **Dynamic "Other" Sub-Groups:** Objects with `object_type: "Other"` are sub-grouped by `sub_category` (e.g., "Enemy", "NPC", "VFX") with dedicated sub-headers.
  - **Uncategorized Fallback:** Objects whose type doesn't match any schema category are collected under an "Uncategorized" header.
  - **Game Switching**: Managed via `useGameSwitch` hook. Switching invalidates all TanStack Query caches (`objects`, `mod-folders`, `category-counts`). The sidebar clears and reloads for the new game.

### US-3.2: Object Identification & Matching ✅

**As the** system, **I must** match physical folders in the `/Mods` folder with the correct entries in the database, **So that** metadata (Rarity, Element, Tags) can be correctly displayed in the UI.

- **Acceptance Criteria (All Implemented):**
  - **Filesystem-First Validation:** `useObjects` hook directly invokes the `get_objects_cmd` Rust command, which actively scans the physical game directory and structurally merges it with database metadata. The FS is the ultimate single source of truth.
  - **Staged Matching Pipeline:** Uses `match_object_with_staged_pipeline` with the Deep Matcher from Epic 2 (name + content + fuzzy matching).
  - **Fallback to "Other":** If a folder does not match any category, it is placed under "Other" via auto-linking in `sync_with_db`.

### US-3.3: Object CRUD & Context Menu ✅

**As a** user, **I want to** create, edit, and manage category objects with full flexibility, either manually or through database synchronization.

- **Acceptance Criteria (All Implemented):**

  #### Create New Object (`CreateObjectModal`)
  - **Manual Mode**: react-hook-form + Zod validated form with fields: Name, Object Type (category), Sub-Category, Is Safe flag, and dynamic metadata fields based on the selected category's schema.
  - Creates DB entry via `objectService.createObject()` with duplicate name detection.

  #### Edit Object (`EditObjectModal`)
  - **Manual Tab**: Edit metadata fields (Element, Rarity, Gender, Weapon Type, Path, Tags), thumbnail, and NSFW flag.
  - **Auto-Sync Tab**: Search and select from MasterDB entries. When selected, auto-populates all form fields including name, category, metadata, and thumbnail from the database.
  - Uses `useMasterDbSync` hook for dropdown search against MasterDB.
  - Uses `useEditObjectForm` hook for form state management.
  - Supports both `ObjectSummary` (DB objects) and `ModFolder` (filesystem folders) as edit targets.

  #### Context Menu Actions (Dual Mode)

  **Object Mode** (right-click on DB objects):
  | Action | Implementation |
  |--------|---------------|
  | Edit Metadata | Opens `EditObjectModal` |
  | Reveal in File Explorer | `reveal_object_in_explorer` (Rust command) |
  | Pin to Top / Unpin | `pin_object` (Rust command), toggles `is_pinned` |
  | Enable | Renames folder removing `DISABLED ` prefix via `toggle_mod` |
  | Disable | Renames folder adding `DISABLED ` prefix via `toggle_mod` |
  | Move Category... | Submenu with all schema categories. Updates DB + child mod `info.json` files |
  | Sync with DB | Matches against MasterDB → opens `SyncConfirmModal` with diff preview |
  | Delete Object | Deletes from `objects` table via `objectService.deleteObject()` |

  **Folder Mode** (right-click on filesystem folders):
  | Action | Implementation |
  |--------|---------------|
  | Enable / Disable | `toggle_mod` (Rust) — folder rename with `DISABLED ` prefix |
  | Open in Explorer | `open_in_explorer` (Rust command) |
  | Favorite | `toggle_favorite` on the mod's DB record |
  | Edit Metadata | Opens `EditObjectModal` for the folder |
  | Move to... | Submenu — sets category via `set_mod_category` (Rust) |
  | Sync with DB | Matches folder name against MasterDB |
  | Move to Trash | `delete_mod` via Trash service (soft delete) |

### US-3.4: Schema-Driven Filtering & Sorting ✅

**As a** user, **I want** filters relevant to the game I am playing (e.g., Element in Genshin, Path in HSR).

- **Acceptance Criteria (All Implemented):**
  - **Dynamic Filter UI (`FilterPanel`)**: Chip-based filter panel with sections for:
    - **Category**: Chip per schema category (e.g., Character, Weapon, UI, Other)
    - **Sort**: A–Z / Newest / Rarity (★)
    - **Status**: All / Enabled / Disabled
    - **Metadata**: Dynamic chips from `schema.json` filters (Element, Rarity, Weapon Type, etc.)
  - **Live Filtering**: Backend SQL query (`objectService.getObjects`) applies all filters instantly. No "Apply" button.
  - **Active Filter Chips**: When filter panel is collapsed, active filters show as dismissible badge chips.
  - **Clear All**: One-click reset for all filters.
  - **Filter Badge**: Button shows count of active filters when panel is collapsed.

### US-3.5: Scalable Sidebar (Virtualization) ✅

**As a** user, **I want** the sidebar not to lag even with 1000+ objects.

- **Acceptance Criteria (All Implemented):**
  - **Virtual List**: `useObjectListVirtualizer` uses `@tanstack/react-virtual` with dynamic `estimateSize` per item type (header: 28px, sub-header: 24px, row: 70px/82px mobile). Overscan: 10.
  - **Sticky Header Logic**: Custom scroll tracking via `ScrollEvent` + `ResizeObserver` for sticky selected-item indicators (top/bottom arrow).
  - **Scroll-to-Selected**: `scrollToSelected()` smoothly scrolls to the selected object.
  - **No Collapse**: All category groups are always expanded — no lazy child render toggle needed since virtualization handles performance.

### US-3.6: Instant Search (Web Worker) ✅

**As a** user, **I want** search results to appear instantly (<50ms) as I type.

- **Acceptance Criteria (All Implemented):**
  - **Web Worker Search**: `useSearchWorker` hook runs search queries off-main-thread.
  - **Dual Search Mode**: Supports both SQL-level search (via `objectService`) and client-side Web Worker search (configurable via `localSearch` option).
  - **Search Scope**: Respects active category, status, and metadata filters.

### US-3.7: Auto Reorganize (Full Scan & Sync) ✅

**As a** user, **I want** to reorganize all mods at once by scanning and matching them against the MasterDB.

- **Acceptance Criteria (All Implemented):**
  - **Two-Phase Flow**:
    1. **Preview Phase**: `scanService.scanPreview()` scans all mods without DB writes → opens `ScanReviewModal`.
    2. **Commit Phase**: User reviews/adjusts matches, then commits via `scanService.commitScan()`.
  - **Override Support**: `ScanReviewModal` provides MasterDB entry search for manual overrides.
  - **Toast Feedback**: Shows scan results summary (scanned, new mods, new objects).

### US-3.8: Single-Object DB Sync ✅

**As a** user, **I want** to sync a single object's metadata with the MasterDB via context menu.

- **Acceptance Criteria (All Implemented):**
  - **Match via Rust**: `match_object_with_db` command uses the staged quick matcher pipeline from Epic 2.
  - **Diff Preview**: `SyncConfirmModal` shows current vs. matched data (name, category, metadata, thumbnail) before applying.
  - **Apply**: Updates DB object (SQL) or filesystem folder (rename + category + info.json + thumbnail) depending on item type.
  - **Fallback**: If no match found, user can switch to manual edit.

### US-3.Z: Drag & Drop Ingestion (Visual Feedback) ✅

**As a** user, **I want** visual feedback when dragging files over the sidebar.

- **Acceptance Criteria (Implemented):**
  - **Visual Feedback**: `useFileDrop` hook provides `isDragging` state — sidebar highlights with ring-2 border and bg tint.
  - **Note**: Full drag-and-drop ingestion (auto-tagging, background copy/move) is planned for a future iteration.

### US-3.S: Safe Mode ✅

**As a** user, **I want** control over the visibility of sensitive content.

- **Acceptance Criteria (Implemented):**
  - **Safe Mode Filter**: `objectService.getObjects()` SQL query respects `safe_mode` flag — hides objects where `is_safe = false` when Safe Mode is active.
  - **Zustand Integration**: Reacts to `appStore.safeMode` state.

---

## 2. Implementation Details

### A. Frontend Component Architecture

| File                    | Purpose                                                                 |
| ----------------------- | ----------------------------------------------------------------------- |
| `ObjectList.tsx`        | Thin orchestrator composing Toolbar, States, Content, Modals, StatusBar |
| `ObjectListToolbar.tsx` | Search bar + filter toggle + refresh + sync + create buttons            |
| `FilterPanel.tsx`       | Unified chip panel for category, sort, status, and metadata filters     |
| `ObjectListContent.tsx` | Virtualized list rendering with context menu integration                |
| `ObjectRowItem.tsx`     | Individual object row with thumbnail, metadata, element icons, badges   |
| `ObjectContextMenu.tsx` | Dual-mode context menu (object vs folder)                               |
| `ObjectListStates.tsx`  | Loading/error/no-game/empty state rendering                             |
| `ObjectListModals.tsx`  | Modal orchestrator (delete, edit, sync, create, scan review)            |
| `CreateObjectModal.tsx` | New object creation form (react-hook-form + zod)                        |
| `EditObjectModal.tsx`   | Edit metadata with Manual/Auto-Sync tabs                                |
| `SyncConfirmModal.tsx`  | Single-object DB match preview + apply                                  |
| `ScanReviewModal.tsx`   | Bulk scan preview + commit                                              |

### B. Custom Hooks

| Hook                          | Purpose                                                                     |
| ----------------------------- | --------------------------------------------------------------------------- |
| `useObjectListLogic.ts`       | Main orchestrator hook: wires up data, filters, search, virtualizer         |
| `useObjectListHandlers.ts`    | All action handlers (toggle, delete, edit, sync, pin, move, enable/disable) |
| `useObjectListVirtualizer.ts` | Virtualizer setup, category grouping, sticky header logic                   |
| `useEditObjectForm.ts`        | Form state management for EditObjectModal                                   |
| `useObjects.ts`               | TanStack Query hooks (useObjects, useGameSchema, useCategoryCounts, etc.)   |
| `useSearchWorker.ts`          | Web Worker-based instant search                                             |

### C. Service Layer

| File               | Purpose                                                                      |
| ------------------ | ---------------------------------------------------------------------------- |
| `objectService.ts` | Frontend SQL queries via `@tauri-apps/plugin-sql` (CRUD for `objects` table) |
| `scanService.ts`   | Scan preview/commit via Rust invoke commands                                 |

### D. Rust Backend Commands (`object_cmds.rs`)

| Command                | Purpose                                                  |
| ---------------------- | -------------------------------------------------------- |
| `get_game_schema`      | Load schema.json with fallback to default categories     |
| `get_object`           | Get single object by ID                                  |
| `get_master_db`        | Load MasterDB JSON for a game type                       |
| `pin_object`           | Toggle `is_pinned` flag in DB                            |
| `delete_object`        | Delete object from DB                                    |
| `match_object_with_db` | Match object name against MasterDB using staged pipeline |

Additional commands used from `mod_cmds.rs`:
| Command | Purpose |
|---------|---------|
| `reveal_object_in_explorer` | Open object's folder in Windows File Explorer |
| `toggle_mod` | Enable/disable via folder rename (`DISABLED ` prefix) |
| `open_in_explorer` | Open any path in File Explorer |
| `rename_mod_folder` | Rename a mod folder |
| `set_mod_category` | Update mod's category in `info.json` |
| `repair_orphan_mods` | Fix orphaned mod DB records |

### E. ObjectRowItem Visual Design

Each row in the sidebar renders:

- **Thumbnail** (44×44 / 64×64 mobile) with:
  - DB `thumbnail_path` image or fallback `Component` icon
  - Power-off overlay when all mods disabled (grayscale + dim)
- **Name** with pin icon (📌) and mod count badges (total / enabled)
- **Metadata subtext**: Element icon (Pyro🔥, Hydro💧, etc.), Weapon type, Rarity (★), Gender, Path
- **Selection indicator**: Right-side primary-colored bar with glow effect
- **Disabled state**: Line-through name, grayscale thumbnail

---

## 3. Database Schema (SQLite)

### `objects` table

| Column           | Type    | Description                                                    |
| ---------------- | ------- | -------------------------------------------------------------- |
| `id`             | TEXT PK | UUID primary key                                               |
| `game_id`        | TEXT    | Foreign key to games table                                     |
| `name`           | TEXT    | Display name (e.g., `Raiden Shogun`)                           |
| `object_type`    | TEXT    | Category: Character, Weapon, UI, Other                         |
| `sub_category`   | TEXT    | Specific name for Other type (Enemy, NPC, etc.)                |
| `folder_path`    | TEXT    | Physical folder path on disk (FS source of truth)              |
| `sort_order`     | INTEGER | Custom sort order                                              |
| `tags`           | TEXT    | JSON array of search aliases                                   |
| `metadata`       | TEXT    | JSON object (Element, Rarity, Gender, Weapon Type, Path, etc.) |
| `thumbnail_path` | TEXT    | Absolute path to thumbnail image                               |
| `is_safe`        | BOOLEAN | SFW flag for Safe Mode filtering                               |
| `is_pinned`      | BOOLEAN | Pin to top of list                                             |
| `is_auto_sync`   | BOOLEAN | Auto-sync with MasterDB on scan                                |
| `created_at`     | TEXT    | ISO timestamp                                                  |

### TypeScript Types (`types/object.ts`)

- `GameObject` — Full DB record
- `ObjectSummary` — Lightweight summary including `folder_path` with `mod_count` and `enabled_count` (joined query)
- `ObjectFilter` — Filter criteria DTO
- `GameSchema` — Schema definition (categories + filters)
- `CategoryDef` — Category with name, label, icon, color, per-category filters
- `FilterDef` — Metadata filter field (key, label, options)
- `UpdateObjectInput` / `CreateObjectInput` — Mutation DTOs

---

## 4. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [x] **Game Switching**: Toggle GIMI to WWMI → Sidebar clears GIMI items → Loads WWMI objects correctly from DB with cache invalidation.
- [x] **Dynamic Categorization**: Database entry `Type: Other, Sub: Enemy` → Sidebar automatically creates "Enemy" sub-header under "Other" and groups relevant items.
- [x] **Schema-Driven Filtering**: FilterPanel loads dynamic metadata chips from `schema.json` per game. Select "Pyro" → Only Pyro characters appear.
- [x] **Status Filtering**: Select "Enabled" → Only objects with `enabled_count > 0` appear.
- [x] **Sort Options**: Sort by Name (A-Z), Date (Newest), Rarity (★) — works across all categories.
- [x] **Alias Search**: Typing search query → Matches by name and tags via Web Worker or SQL `LIKE`.
- [x] **Create Object**: Create modal validates name (min 2 chars, no duplicates, filesystem-safe names).
- [x] **Edit Object**: Edit metadata with Manual tab (form fields) or Auto-Sync tab (MasterDB dropdown).
- [x] **Pin Object**: Context menu → Pin to Top → Object appears first in its category group.
- [x] **Enable/Disable Object**: Context menu → Enable/Disable → Folder renamed with/without `DISABLED ` prefix.
- [x] **Move Category**: Context menu → Move Category → Updates DB `object_type` + child mod `info.json` files.
- [x] **Sync with DB**: Context menu → Shows diff preview in SyncConfirmModal → Applies name, category, metadata, thumbnail.
- [x] **Auto Reorganize**: Toolbar button → Full scan preview → ScanReviewModal → Commit with results toast.
- [x] **Reveal in Explorer**: Context menu → Opens object's physical folder in Windows File Explorer.
- [x] **Delete Object**: Context menu → Removes from `objects` table with cache invalidation.
- [x] **Drag & Drop Visual**: Dragging files over sidebar → Ring highlight + background tint.

### 2. Negative Cases (Error Handling)

- [x] **Missing Metadata**: Object with no DB metadata → Row shows mod count fallback instead of element/rarity.
- [x] **Schema Load Failure**: `schema.json` entirely missing → fallback to default [Character, Weapon, UI, Other] → Log WARN.
- [x] **Orphaned Object**: Database has entry but physical folder is missing → Filtered out by `filter_existing_folders` check.
- [x] **Thumbnail Load Error**: Failed image load → Falls back to `Component` icon with `onError` handler.
- [x] **Sync No Match**: Object name doesn't match any MasterDB entry → User can switch to manual edit.

### 3. Edge Cases (Scalability & Stability)

- [x] **Massive List Scrolling**: 2000+ items → Smooth scrolling via `@tanstack/react-virtual` with dynamic size estimation.
- [x] **External Deletion**: Sidebar refreshes via file watcher events (E2 `notify` watcher) and manual Refresh button with `repair_orphan_mods`.
- [x] **Rapid Switching**: Game switch invalidates all queries → TanStack Query handles deduplication.
- [x] **Safe Mode Active**: With SFW mode ON, SQL query filters out `is_safe = false` objects.
- [x] **Active Filter Persistence**: Filter chips remain visible as badges when panel is collapsed.
- [x] **Duplicate Object Name**: `createObject` validates uniqueness before insertion.
- [x] **Reserved Filesystem Names**: `validateObjectName` blocks Windows-reserved names (CON, PRN, AUX, etc.).

### 4. Technical Metrics

- [x] **Search Latency**: Instant via Web Worker (off-main-thread) or SQL `LIKE` queries.
- [x] **Render Time**: Virtualized — only visible rows render at any time.
- [x] **Virtual Scroll**: Dynamic estimateSize per item type, overscan of 10 items.
