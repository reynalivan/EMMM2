# Epic 12: Folder Grid UI

## 1. Executive Summary

- **Problem Statement**: The center panel needs to present hundreds to thousands of mod folders as both thumbnail-rich cards and a dense list table — without freezing the UI — while supporting node-type-aware navigation (navigate into containers, open details for mod packs, open variant picker for variant sets), sorting, and clear empty states.
- **Proposed Solution**: A virtualized grid/list hybrid powered by `@tanstack/react-virtual`, with node-type-driven double-click behavior (`ContainerFolder` → navigate; `ModPackRoot` → open Details; `VariantContainer` → open Variant Picker; `InternalAssets` → no-op), distinct visual badges per type, context menu actions scoped by type (including "Open content mods (Advanced)" for `ModPackRoot`), client-side sorting within node-type groups, a breadcrumb navigation bar for deep sub-paths, and a "ADVANCED" breadcrumb indicator when browsing mod internals.
- **Success Criteria**:
  - Grid renders 1,000 cards at ≥ 60fps, measured via Chrome DevTools Performance tab.
  - View mode toggle (Grid ↔ List) completes in ≤ 100ms — no layout jitter.
  - Client-side sort of 1,000 items completes in ≤ 50ms (localeCompare or timestamp diff), applied within groups (Folders group sorted separately from Mod Packs group).
  - Breadcrumb click navigates to the target sub-path in ≤ 200ms (React Query cache hit).
  - `VariantContainer` double-click opens Variant Picker modal in ≤ 150ms.
  - Empty state renders within ≤ 100ms of receiving an empty array — no blank white screen flash.

---

## 2. User Experience & Functionality

### User Stories

#### US-12.1: Grid vs List Modes

As a user, I want to toggle between a thumbnail grid and a dense text list, so that I can browse visually or manage large volumes efficiently.

| ID        | Type        | Criteria                                                                                                                                                                                                           |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-12.1.1 | ✅ Positive | Given the folder grid, when I click the view toggle, then the layout switches between "Card Grid" (thumbnail cards with 3–5 columns) and "List" (compact rows with name, enabled status, modified date) in ≤ 100ms |
| AC-12.1.2 | ✅ Positive | Given a view mode change, the preference is written to `localStorage['gridViewMode']` and persists across game context switches and app restarts                                                                   |
| AC-12.1.3 | ❌ Negative | Given a layout switch while the virtualized list is mid-scroll, then the scroll position resets to top and no permanent white-space gap is rendered                                                                |
| AC-12.1.4 | ⚠️ Edge     | Given a window resize that changes available column count mid-render in Grid Mode, the grid recalculates column widths within one `ResizeObserver` callback — no overflow or clipped cards                         |

---

#### US-12.2: Node-Type-Aware Navigation

As a user, I want double-clicking a folder to do the right thing depending on what type it is, so that I can browse containers freely without accidentally entering mod internals.

| ID        | Type        | Criteria                                                                                                                                                                                                     |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-12.2.1 | ✅ Positive | Given a `ContainerFolder`, when I double-click it, then `currentSubPath` updates to include that folder and the breadcrumb advances — grid refreshes with children                                           |
| AC-12.2.2 | ✅ Positive | Given a `ModPackRoot`, when I double-click it, then the Preview Panel / Details view opens — **no folder navigation** occurs; breadcrumb does not change                                                     |
| AC-12.2.3 | ✅ Positive | Given a `VariantContainer`, when I double-click it, then the Variant Picker modal opens in ≤ 150ms — **no folder navigation** occurs                                                                         |
| AC-12.2.4 | ✅ Positive | Given an `InternalAssets` folder (only visible in Advanced mode), when I double-click it, then no action is taken — it is a static read-only entry                                                           |
| AC-12.2.5 | ⚠️ Edge     | Given a `ContainerFolder` that has no navigable children (all children are `InternalAssets`), then the grid shows an empty state "This folder contains only internal mod assets" — not a generic empty state |

---

#### US-12.3: Sorting the Grid

As a user, I want to sort mods by name or modified date, so that I can find recent additions or locate specific mods alphabetically.

| ID        | Type        | Criteria                                                                                                                                                                                                                 |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-12.3.1 | ✅ Positive | Given the sort control set to "Date Modified", then items are re-ordered by `fs::metadata().modified()` timestamp descending — no IPC refetch; pure client-side sort in ≤ 50ms                                           |
| AC-12.3.2 | ✅ Positive | Given sorting is active, it applies **within each visual group**: `ContainerFolder` entries are sorted among themselves; `ModPackRoot` + `VariantContainer` entries are sorted among themselves — groups do not intermix |
| AC-12.3.3 | ✅ Positive | Given the direction toggle (Asc / Desc), the sort direction applies immediately and persists in `localStorage['gridSort']`                                                                                               |
| AC-12.3.4 | ❌ Negative | Given some items lack a valid `modified` timestamp (OS restriction or missing metadata), they are sorted to the end of their group — no crash or NaN sort instability                                                    |
| AC-12.3.5 | ⚠️ Edge     | Given multiple files share the exact same timestamp, secondary sort by name (A-Z) breaks the tie — stable, deterministic order                                                                                           |
| AC-12.3.6 | ✅ Positive | Given the toolbar Search input, when I type a query, then the grid filters its items purely client-side by exact/fuzzy substring match on folder name or metadata name in real-time without fetching from backend        |

---

#### US-12.4: Breadcrumb Navigation

As a user, I want a breadcrumb bar showing my current folder path, so that I can navigate back up the hierarchy without losing my place.

| ID        | Type        | Criteria                                                                                                                                                                                        |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-12.4.1 | ✅ Positive | Given I am in a nested path (e.g., `Characters/Albedo/Variants`), then the breadcrumb renders each segment as a clickable link: `Root > Characters > Albedo > Variants`                         |
| AC-12.4.2 | ✅ Positive | Given I click a parent breadcrumb segment, then `currentSubPath` in Zustand updates and the grid navigates to that parent path in ≤ 200ms (React Query cache hit)                               |
| AC-12.4.3 | ✅ Positive | Given I used "Open content mods (Advanced)" on a `ModPackRoot`, then the breadcrumb appends an `[ADVANCED]` badge after the pack name to indicate internal browsing mode                        |
| AC-12.4.4 | ❌ Negative | Given a directly injected or programmatically set `sub_path` that doesn't exist on disk, then after backend returns `IO: NotFound`, the grid renders an explicit "Folder not found" error state |
| AC-12.4.5 | ⚠️ Edge     | Given a path with ≥ 10 segments that exceeds the breadcrumb bar width, then middle segments are collapsed into an "…" dropdown — first and last segments always visible                         |
| AC-12.4.6 | ✅ Positive | Given the app is restarted, it automatically opens the last visited category/subfolder using persisted state in `useAppStore` — preventing the need to re-navigate every time                   |

---

#### US-12.5: Variant Picker

As a user, I want to pick which variant of a mod to use when a folder contains mutually exclusive sub-mods, so that I can switch looks without leaving the grid.

| ID        | Type        | Criteria                                                                                                                                                                                                                                   |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AC-12.5.1 | ✅ Positive | Given a `VariantContainer` is double-clicked, when the Variant Picker modal opens, then all `variants[]` from the `FolderEntry` are listed with their names and thumbnails (if available)                                                  |
| AC-12.5.2 | ✅ Positive | Given I click a variant, then: (1) that variant is enabled (strip `DISABLED ` prefix), (2) all other variants in the same `variant_group_id` are disabled (add `DISABLED ` prefix), (3) the `VariantContainer`'s parent state is unchanged |
| AC-12.5.3 | ✅ Positive | Given Safe Mode is active when the Variant Picker is open, then variants with `is_safe = false` are blurred and their names masked to "[Hidden Variant]" — selection is blocked for those variants                                         |
| AC-12.5.4 | ❌ Negative | Given a variant's rename (enable/disable) fails (disk error), then the Variant Picker shows an error toast "Could not switch variant: {reason}" and rolls back any partial changes                                                         |
| AC-12.5.5 | ⚠️ Edge     | Given a `VariantContainer` with only 1 variant remaining (others deleted externally), then the Variant Picker shows that single variant with an info note "Only 1 variant available" — no crash                                            |

---

#### US-12.6: Context Menu Actions by Node Type

As a user, I want the context menu to show only relevant actions for each folder type, so that I don't see confusing options for the wrong type.

| ID        | Type        | Criteria                                                                                                                                                                                             |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-12.6.1 | ✅ Positive | Given any folder type, the context menu includes "Open in Explorer" — always available                                                                                                               |
| AC-12.6.2 | ✅ Positive | Given a `ModPackRoot` or `VariantContainer`, the context menu includes: "Enable / Disable", "Open Details", and "Open content mods (Advanced)"                                                       |
| AC-12.6.3 | ✅ Positive | Given "Open content mods (Advanced)" is clicked, then the grid navigates inside the pack's folder with all child entries visible (including `InternalAssets`), and the breadcrumb shows `[ADVANCED]` |
| AC-12.6.4 | ✅ Positive | Given a `VariantContainer`, the context menu also includes "Choose variant…" (opens Variant Picker) and optional "Next variant" / "Previous variant" shortcuts                                       |
| AC-12.6.5 | ❌ Negative | Given a `ContainerFolder`, the "Enable/Disable" and "Open content mods (Advanced)" options are **not** shown in the context menu                                                                     |

---

#### US-12.7: Visual Language per Node Type

As a user, I want folder cards to visually communicate their type so I know at a glance which ones I can browse and which are complete mod packs.

| ID        | Type        | Criteria                                                                                                                                                                    |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-12.7.1 | ✅ Positive | Given a `ContainerFolder`, its card shows a standard folder icon and no type badge — it looks like a normal navigable directory                                             |
| AC-12.7.2 | ✅ Positive | Given a `ModPackRoot`, its card shows a package/box icon and a `MOD PACK` badge in the card's top-right corner                                                              |
| AC-12.7.3 | ✅ Positive | Given a `VariantContainer`, its card shows a package icon and a `VARIANTS` badge                                                                                            |
| AC-12.7.4 | ✅ Positive | Given an `InternalAssets` folder visible in Advanced mode, its card is visually dimmed (opacity 0.6) with an `INTERNAL` badge                                               |
| AC-12.7.5 | ⚠️ Edge     | Given hovering over a card badge, a tooltip renders the `classification_reasons[]` strings from the `FolderEntry` — e.g., "Classified as Mod Pack: has-mod-ini, has-assets" |

---

#### US-12.8: Empty States

As a user, I want clear visual feedback when a folder has no mods or no search results, so that I know the data pipeline is working correctly.

| ID        | Type        | Criteria                                                                                                                                                                                     |
| --------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-12.8.1 | ✅ Positive | Given the backend returns an empty array for an actual empty folder, then an EmptyState renders an illustration + "No mods here yet — drop mod folders to begin" message                     |
| AC-12.8.2 | ❌ Negative | Given the empty array is caused by a search filter returning zero matches, then the EmptyState shows "No results for '{query}'" — distinct from the physically-empty folder state            |
| AC-12.8.3 | ⚠️ Edge     | Given a slow scan that populates the folder after initial render, then when the React Query cache updates, the EmptyState transitions to the populated grid in ≤ 200ms with no layout jitter |

---

### Non-Goals

- Sorting is entirely client-side — no backend sort parameter in `list_folders`.
- No infinite scroll pagination; all items for a given `sub_path` are loaded at once (virtualization handles rendering performance).
- No columns customization (add/remove columns) in the List view in this phase.
- "Open content mods (Advanced)" is a read-only browsing mode — no bulk enable/disable for InternalAssets from within it.
- Grid card thumbnail display relies on Epic 41 (Thumbnail Cache); this epic handles only the layout.

---

## 3. Technical Specifications

### Architecture Overview

```
FolderGrid (React)
  ├── Toolbar
  │   ├── ViewToggle → localStorage['gridViewMode']
  │   ├── SortSelect + DirectionToggle → client-side sort in useMemo (within-group)
  │   └── Breadcrumbs → currentSubPath segments + optional [ADVANCED] badge
  ├── VirtualizedGrid / VirtualizedList (@tanstack/react-virtual)
  │   ├── Group: "Folders" (ContainerFolder entries)
  │   └── Group: "Mod Packs" (ModPackRoot + VariantContainer entries)
  │       └── ModCard / ModRow → node_type determines icon + badge + click handler
  ├── VariantPickerModal (shown on VariantContainer double-click)
  │   └── lists FolderEntry.variants[] → invoke('toggle_variant', { group_id, chosen_path })
  └── EmptyState (3 variants: folder-empty | filter-no-results | internal-assets-only)

FolderCard click handlers (by node_type):
  ContainerFolder → setCurrentSubPath(folder_path)
  ModPackRoot     → openPreviewPanel(folder_path)
  VariantContainer → openVariantPicker(entry)
  InternalAssets  → noop

Context menu (by node_type):
  All types: "Open in Explorer"
  ModPackRoot | VariantContainer: + "Enable/Disable", "Open Details", "Open content mods (Advanced)"
  VariantContainer only: + "Choose variant…", "Next variant", "Previous variant"

toggle_variant(game_id, variant_group_id, chosen_folder_path) → ():
  1. Acquire OperationLock(game_id) + WatcherSuppression(all variant paths)
  2. Disable all variants in group except chosen_folder_path
  3. Enable chosen_folder_path
  4. Return success
```

### Integration Points

| Component      | Detail                                                                                                                                                                                |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| Data Source    | `invoke('list_folders', { gameId, subPath })` (Epic 11) — provides `node_type`, `variants[]`, `is_navigable`, `classification_reasons[]`                                              |
| Virtualization | `@tanstack/react-virtual` — `useVirtualizer` with `estimateSize` = 220px (grid) / 48px (list)                                                                                         |
| Sort           | `Array.prototype.sort()` within each group separately — `localeCompare` (name) or numeric diff (timestamp)                                                                            |
| Path State     | `currentSubPath` in Zustand — updated on ContainerFolder double-click or breadcrumb click                                                                                             |
| Advanced Mode  | `isAdvancedMode: bool` in Zustand — set when "Open content mods (Advanced)" is triggered; breadcrumb shows `[ADVANCED]` badge; `list_folders` receives `include_internals: true` flag |
| Variant Picker | `VariantPickerModal` → `invoke('toggle_variant', ...)` under OperationLock + WatcherSuppression                                                                                       |
| View Persist   | `localStorage['gridViewMode']` = `'grid'                                                                                                                                              | 'list'` |
| Thumbnail      | `convertFileSrc(thumbnailPath)` from `@tauri-apps/api` — renders inside `<img>` with fallback icon                                                                                    |

### Security & Privacy

- **`subPath` is validated backend-side** (Epic 11 path traversal guard) — the frontend passes the raw segment but the backend's `canonicalize()` + `starts_with(mods_path)` check prevents any traversal.
- **Safe Mode**: Items with `is_safe = false` are excluded by the backend from `list_folders` response. In Variant Picker, `is_safe = false` variants are shown but blurred/blocked — not removed, to avoid implying a variant doesn't exist.
- **No user-supplied strings** are passed to `dangerouslySetInnerHTML` or evaluated as code; all mod names and classification reasons are rendered as `textContent` only.
- **OperationLock on `toggle_variant`** prevents concurrent variant switches from creating conflicting rename states.

---

## 4. Dependencies

- **Blocked by**: Epic 11 (Folder Listing — data source + classification), Epic 05 (Workspace Layout — panel container).
- **Blocks**: Epic 13 (Core Mod Ops — toggle/rename actions on cards), Epic 14 (Bulk Operations — multi-select), Epic 15 (Explorer Interactions — context menu, DnD), Epic 41 (Thumbnail — card image display).
