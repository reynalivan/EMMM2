# Epic 4: Folder Grid & Advanced Explorer Logic (Final Version)

**Focus:** Providing the primary work interface that supports seamless folder navigation, secure file management, and an intelligent matching system to maintain a tidy mod structure.

## Dependencies

| Direction    | Epic      | Relationship                                     |
| ------------ | --------- | ------------------------------------------------ |
| ⬆ Upstream   | Epic 3    | Sidebar selection drives grid content            |
| ⬆ Upstream   | Epic 2    | Uses file watcher service for real-time sync     |
| ⬇ Downstream | Epic 5    | Context menu triggers core operations            |
| ⬇ Downstream | Epic 6    | Grid click opens preview panel                   |
| ⬇ Downstream | Epic 8, 9 | Provides trash service for all delete operations |

## Cross-Cutting Requirements

- **Trash System:** Custom soft-delete to `./app_data/trash/{uuid}/` with `metadata.json`-based restore (NOT OS Recycle Bin).
- **Thumbnails:** Generated as WebP using `image` crate (256×256). 2-tier cache: folder-keyed LRU L1 (memory, 200 items) + disk L2 (`app_data/thumbnails/`).
- **info.json Lifecycle:** This Epic is the **creator** of `info.json`. E6 edits it. E5/E7/E8 read it.
- **Default info.json:** `{ actual_name: <folder_name>, author: "Unknown", description: "", version: "1.0", tags: [], is_safe: true, is_favorite: false }`
- **Virtualization:** Grid uses `@tanstack/react-virtual` for all item counts. `estimateSize` adapts per view mode (Grid: `cardH`, List: `52px`).
- **File Watcher:** Suppressed for in-app operations via `SuppressionGuard` (per TRD §3.5).
- **Operation Lock:** All destructive operations (toggle, rename, delete) acquire `OperationLock` to prevent concurrency conflicts.

---

## 1. User Stories & Acceptance Criteria

### US-4.1: Pro-Level Navigation & Sorting ✅

**As a** user, **I want to** explore subfolders freely with the help of _breadcrumbs_ and _sorting_ features, **So that** I have full control similar to using Windows Explorer.

- **Acceptance Criteria:**
  - **Breadcrumbs**: ✅ Interactive navigation bar (`Breadcrumbs.tsx`) to jump to any parent folder. Home (ROOT) button returns to root. Overflow truncation when depth > 4 segments. Displays logical `name` (aliases) but drives navigation via physical `folder_path`.
  - **Deep Navigation**: ✅ Supports entering subfolders via _double-click_ without level restrictions. Back button returns to parent folder.
  - **Advanced Sorting**: ✅ Sortable by **Name** (A-Z / Z-A) and **Date Modified** (Newest / Oldest) via dropdown in the toolbar.
  - **Favorites System**: ✅ Users can mark folders as "Favorite" (Star icon), which are _pinned_ to the top of the grid list. Synced to both DB (`is_favorite`) and `info.json`.
  - **Search Bar**: ✅ _(Added Value)_ Real-time client-side search filtering by folder name directly in the toolbar.
  - **Grid/List View Toggle**: ✅ _(Added Value)_ Users can switch between Grid (card thumbnails) and List (compact row) view modes via the toolbar. State persisted.

### US-4.2: Instant Thumbnail & Lazy Load Cache ✅

**As a** user, **I want to** replace thumbnails instantly via the _clipboard_ and view thousands of mods without performance bottlenecks, **So that** my collection remains visual and responsive.

- **Acceptance Criteria:**
  - **Clipboard Support**: ✅ Right-click on a folder > "Paste Thumbnail" (takes the image from the _clipboard_). Validates clipboard content and rejects non-image data.
  - **Image Import**: ✅ Right-click > "Import Thumbnail..." to manually select a file (PNG/JPG/JPEG/WebP filter).
  - **Naming Standard**: ✅ New images are saved as `preview_custom.png` in the mod folder. Preview image naming uses `preview_{object_name}.webp` convention with auto-increment suffix for duplicates.
  - **Hybrid Cache Strategy (Rust/Tauri)**: ✅
    - **L1 Cache (Memory)**: Folder-keyed `LruCache` (capacity 200 items, `NonZeroUsize`). Uses `OnceLock<Mutex<ThumbnailCache>>` singleton. Each entry has a `CachedEntry` struct with TTL timestamp.
    - **L2 Cache (Disk)**: Thumbnails stored as physical WebP files (256×256) in `app_data/thumbnails/` folder. SHA-256 hash of path used as cache key filename.
    - **Smart Validation**: Before loading from cache, checks `mtime` of original file via `validate_mtime()`. If original is newer, triggers regeneration.
    - **Concurrency Control**: ✅ _(Added Value)_ Semaphore limits concurrent thumbnail generations to 4 max, preventing I/O storm.
    - **Double-check Pattern**: ✅ _(Added Value)_ After acquiring semaphore permit, re-checks L1 to avoid redundant work when another task already resolved.
    - **Orphan Pruning**: ✅ _(Added Value)_ `prune_orphans()` cleans up thumbnails that don't correspond to any valid mod path.
  - **Skeleton UI**: ✅ Shimmer animation (`skeleton bg-base-300`) displayed while images are loading.
  - **Lazy Resolution**: ✅ _(Added Value)_ `get_mod_thumbnail` is called per-card from the frontend via `useThumbnail` hook. Thumbnails are NOT included in `list_mod_folders` response — resolved individually and lazily.
  - **Size Validation**: ✅ Clipboard image > 10MB is rejected with toast "Image too large. Max 10MB."

### US-4.X: UI State Persistence ✅

**As a** user, **I want** the application to remember my last position, **So that** I don't need to re-navigate every time I open the application.

- **Acceptance Criteria:**
  - **Last Active Folder**: ✅ Upon restart, the application automatically opens the last accessed category/subfolder via `useAppStore` (Zustand with persist middleware).
  - **View Mode Persistence**: ✅ _(Added Value)_ Grid/List view mode is persisted across sessions.

### US-4.3: Secure File System Watcher ✅

**As the** system, **I must** monitor folder changes in _real-time_ without conflicting with internal application operations, **So that** data remains accurate and no _crashes_ occur during synchronization.

- **Acceptance Criteria:**
  - **Context Awareness**: ✅ The Watcher uses `SuppressionGuard` pattern — all in-app rename/toggle/delete operations acquire a suppression guard so watcher doesn't trigger unnecessary re-scans.
  - **Sync Protection**: ✅ External changes trigger automatic re-scan for synchronization.

### US-4.4: Soft Delete (Trash System) ✅

**As a** user, **I want to** delete mods safely without permanently removing them from the disk, **So that** I can perform a _recovery_ if a mistake occurs.

- **Acceptance Criteria:**
  - **Trash Location**: ✅ Each trashed item stored in `./app_data/trash/{uuid}/` with `metadata.json` containing: `original_path`, `folder_name`, `trashed_at` (ISO-8601), `game_id`, `trash_id`.
  - **Bulk Move**: ✅ Supports bulk moving of selected folders to Trash via Bulk Context Menu.
  - **Restore**: ✅ `restore_from_trash` command moves the item back to its original location.
  - **List Trash**: ✅ `list_trash` command enumerates all trashed items with metadata.
  - **Empty Trash**: ✅ `empty_trash` command permanently deletes all trashed items and returns count of removed entries.
  - **Cross-Device Support**: ✅ _(Added Value)_ `copy_dir_recursive` fallback for cross-device moves when `fs::rename` fails.

### US-4.5: Mod CRUD & Context Menu ✅

**As a** user, **I want to** perform basic file operations through right-clicking on a mod folder.

- **Acceptance Criteria:**
  - **Context Menu Actions** (via `FolderCardContextMenu.tsx`):
    - **Open in Explorer**: ✅ Opens folder in Windows Explorer using `/select,` flag to highlight the specific folder.
    - **Rename**: ✅ In-place rename with keyboard support (Enter confirms, Escape cancels). Preserves `DISABLED ` prefix for disabled mods. Updates `info.json` `actual_name` field.
    - **Toggle Enable/Disable**: ✅ Adds/removes `DISABLED ` prefix with regex standardization (`standardize_prefix`) to handle messy variants (`disabled_`, `DISABLED-`, `Disable `, `dis_`, etc.). DB synced after rename.
    - **Enable Only This**: ✅ _(Added Value)_ Context menu action to enable a mod while disabling all others in the same object. Available only when the mod is currently disabled.
    - **Favorite/Unfavorite**: ✅ Toggle star icon, synced to both DB and `info.json`.
    - **Paste Thumbnail**: ✅ Paste image from clipboard.
    - **Import Thumbnail**: ✅ Select image file via dialog.
    - **Move to Object...**: ✅ _(Added Value)_ Opens `MoveToObjectDialog` to reassign a mod to a different object. Includes searchable object list, status control (Set Disabled/Only Enable This/Keep Status), and current object highlighting.
    - **Delete to Trash**: ✅ Soft deletes to custom trash system.
  - **Info.json Auto-Generation**: ✅ System reads/displays fields from `info.json` including: `actual_name`, `author`, `description`, `version`, `tags`, `is_safe`, `is_favorite`. Auto-created with default template when missing during scan.
  - **Conflict Detection**: ✅ _(Added Value)_ When enabling a mod that conflicts with other enabled mods in the same object, the `DuplicateWarningModal` warns the user with three options: Force Enable, Enable Only This, or Cancel.
  - **Pre-Delete Check**: ✅ _(Added Value)_ `pre_delete_check` command reports folder contents (item count) so frontend can show confirmation for non-empty folders.

---

## 2. Technical Specifications (Rust/Tauri Implementation)

### A. Bulk Action Management ✅

Supports managing multiple items at once through _multi-select_ (Ctrl+Click / Shift+Click):

- **Enable/Disable Selected**: ✅ Mass activation according to the `"DISABLED "` prefix standard. Emits `bulk-progress` events as each item is processed.
- **Add Tags**: ✅ Opens `BulkTagModal` to add labels/tags to multiple mods at once (saved to each `info.json` via `bulk_update_info`).
- **Bulk Move to Trash**: ✅ Moves all selected items to the application's soft delete folder.
- **Bulk Progress Bar**: ✅ _(Added Value)_ `BulkProgressBar` component shows real-time progress (`current/total`) during bulk operations. Auto-hides when complete.

### B. Dual View Mode ✅ _(Added Value)_

The explorer supports **two view modes**, toggleable via toolbar button:

- **Grid View** (`FolderCard.tsx`): Card layout with thumbnails (256×256 WebP), folder name, enable/disable toggle, favorite star. Responsive column count based on container width (`CARD_MIN_W: 160px`, `CARD_MAX_W: 280px`).
- **List View** (`FolderListRow.tsx`): Compact row layout with small thumbnail (40×40), folder name, favorite button, enable/disable toggle. Fixed row height (52px).

Both views are memoized (`React.memo`) to prevent unnecessary re-renders from virtualizer recalculations.

### C. Info.json Standardization ✅

Every mod folder should have a valid `info.json` file. The Rust struct:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModInfo {
    pub actual_name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
    #[serde(default = "default_true")]
    pub is_safe: bool,
    #[serde(default)]
    pub is_favorite: bool,
}
```

Partial updates supported via `ModInfoUpdate` struct (only specified fields change, others preserved).

### D. Hybrid Cache Strategy (Rust) ✅

```rust
use lru::LruCache;
use std::sync::{Mutex, OnceLock};

struct CachedEntry {
    webp_path: PathBuf,
    inserted_at: Instant,
}

struct ThumbnailCache {
    folder_cache: LruCache<String, CachedEntry>,  // L1: folder-keyed
    image_cache: LruCache<PathBuf, CachedEntry>,   // L1: image-path-keyed (legacy)
    disk_cache_dir: Option<PathBuf>,                // L2: app_data/thumbnails/
}

static THUMBNAIL_CACHE: OnceLock<Mutex<ThumbnailCache>> = OnceLock::new();
```

**Resolution Pipeline** (`resolve()`):

1. Check folder-keyed L1 (fast, no I/O)
2. Acquire semaphore permit (caps concurrent generations to 4 max)
3. Double-check L1 (another task may have resolved while waiting)
4. Cold-resolve in `spawn_blocking` (FS traversal + image processing)

### E. Self-Healing DB Sync & Structure ✅ _(Added Value)_

The system now operates with the **Filesystem as the Source of Truth**. The DB acts as a metadata indexing layer.

- `get_objects_cmd` actively scans the disk directory to construct the object list.
- **Visual Name vs Physical Path**: UI elements (breadcrumbs, sidebar) display the object's `name` (DB alias), while internal filesystem interactions use `folder_path` (physical directory name).
- When a DB path is stale (folder moved/renamed externally), `try_resolve_alternate()` checks if the alternate-prefixed version exists (add/remove `DISABLED ` prefix).
- If found, DB is auto-corrected with the new path and status.
- If truly gone, stale DB rows are cleaned up automatically on read.
- Safe Mode filter (`apply_safe_mode_filter`) hides NSFW content based on `is_safe` flag and configurable keywords.

### F. Drag & Drop Import ✅ _(Added Value)_

- **DragOverlay** (`DragOverlay.tsx`): Full-area overlay with animated upload icon when files are dragged over the grid.
- **useFileDrop Hook**: Handles file drop events from Windows Explorer. Supports both folder imports and archive files.

### G. Explorer Empty State ✅ _(Added Value)_

When no object is selected, `ExplorerEmptyState.tsx` displays a helpful message guiding the user to select a category from the sidebar.

### H. Move to Object Dialog ✅ _(Added Value)_

`MoveToObjectDialog.tsx` provides a full dialog for reassigning mods:

- Searchable object list with current object highlighted/disabled.
- Status control with three options: Set Disabled (default), Only Enable This, Keep Status.
- Backend command `move_mod_to_object` handles the DB update with conditional status logic.

---

## 3. Thumbnail Discovery Rules (Hierarchy)

The system loads images with the following priority order:

1.  **Custom Thumbnail**: `preview_custom.*` file (Result of import/paste).
2.  **Standard Preview**: A file named `preview*` in the mod's root folder.
3.  **Automatic Discovery**: The first image found in the root folder.
4.  **Deep Discovery**: Scan for image files in subfolders up to a maximum depth of 4 levels.
5.  **Fallback**: Display a default placeholder icon (`Folder` icon from lucide-react) if no images are found.

---

## 4. Implemented Component Map

| Component                 | File                        | Description                                   |
| ------------------------- | --------------------------- | --------------------------------------------- |
| **FolderGrid**            | `FolderGrid.tsx`            | Main explorer view with toolbar & virtualizer |
| **FolderCard**            | `FolderCard.tsx`            | Grid view card (memoized)                     |
| **FolderListRow**         | `FolderListRow.tsx`         | List view row (memoized)                      |
| **FolderCardContextMenu** | `FolderCardContextMenu.tsx` | Single-item context menu                      |
| **BulkContextMenu**       | `BulkContextMenu.tsx`       | Multi-select context menu                     |
| **Breadcrumbs**           | `Breadcrumbs.tsx`           | Path navigation with overflow                 |
| **BulkTagModal**          | `BulkTagModal.tsx`          | Tag editor for bulk operations                |
| **BulkProgressBar**       | `BulkProgressBar.tsx`       | Real-time bulk progress indicator             |
| **DuplicateWarningModal** | `DuplicateWarningModal.tsx` | Conflict warning on enable                    |
| **MoveToObjectDialog**    | `MoveToObjectDialog.tsx`    | Reassign mod to different object              |
| **DragOverlay**           | `DragOverlay.tsx`           | Drop zone visual feedback                     |
| **ExplorerEmptyState**    | `ExplorerEmptyState.tsx`    | Empty state guidance                          |
| **useFolderGrid**         | `hooks/useFolderGrid.ts`    | Core hook: data, selection, virtualizer, ops  |

---

## 5. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [x] **Navigation Flow**: Double-click folder → Enters subfolder. Click Breadcrumb → Returns to parent. Home button → Returns to root.
- [x] **Thumbnail Rendering**: Thumbnails load lazily per-card via `useThumbnail` hook (Skeleton → Image) without freezing the UI.
- [x] **Custom Thumbnail**: Right-click folder → "Paste Thumbnail" → `preview_custom.png` created → UI updates on next load.
- [x] **State Persistence**: Restart App → Grid opens at the exact last visited folder via Zustand persist.
- [x] **View Mode Toggle**: Click Grid/List icon → Switches view mode. Persisted across sessions.
- [x] **Bulk Operations**: Select multiple → Right-click → Enable/Disable/Tag/Delete all selected with progress bar.
- [x] **Move to Object**: Right-click → "Move to Object..." → Select target → Choose status → Mod reassigned.
- [x] **Enable Only This**: Right-click disabled mod → "Enable Only This" → All others disabled, this one enabled.
- [x] **Duplicate Warning**: Enable a mod when another is already active → DuplicateWarningModal shows with Force/OnlyThis/Cancel options.

### 2. Negative Cases (Error Handling)

- [x] **Invalid Paste**: User attempts "Paste Thumbnail" with non-image in clipboard → Error message shown.
- [x] **Naming Conflict**: Rename to an existing name → Error: "A folder named 'X' already exists".
- [x] **Missing Assets**: Folder has no image source → Displays folder icon placeholder (no broken image icon).
- [x] **Large Image Paste**: Clipboard image > 10MB → Rejected with "Image too large. Max 10MB."
- [x] **Invalid Characters**: Rename with reserved characters (/, \, :, \*, ?, ", <, >, |) → Error: "Invalid folder name".
- [x] **Nonexistent Path**: Toggle/Rename/Delete on a path that no longer exists → Descriptive error returned.

### 3. Edge Cases (Stability & Performance)

- [x] **Virtualized Scroll**: Grid uses `@tanstack/react-virtual` for all item counts → smooth scroll performance.
- [x] **Memoized Components**: `FolderCard` and `FolderListRow` wrapped in `React.memo` → prevents re-renders from virtualizer recalculations.
- [x] **Deep Hierarchy**: Navigate to deep folder → Breadcrumb truncates middle segments with `…` (max 4 visible segments).
- [x] **Watcher Suppression**: ✅ In-app rename/toggle/delete uses `SuppressionGuard` → does NOT trigger watcher re-scan.
- [x] **Concurrent Safety**: `OperationLock` prevents simultaneous destructive operations.
- [x] **Messy Prefix Handling**: Regex standardization handles `disabled_`, `DISABLED-`, `Disable `, `dis_`, `DIS-` etc. → normalized to `DISABLED `.
- [x] **Self-Healing DB**: Stale mod paths auto-corrected when alternate-prefixed version found on disk.

### 4. Technical Metrics

- [x] **Thumbnail Size**: Generated at 256×256 WebP.
- [x] **Memory Cache**: L1 caps at 200 items (`lru::LruCache`); L2 disk cache as WebP.
- [x] **Concurrency**: Thumbnail generation capped at 4 concurrent tasks via semaphore.
- [x] **Unique IDs**: All grid items have unique `id={grid-item-${item.path}}` for testing.
