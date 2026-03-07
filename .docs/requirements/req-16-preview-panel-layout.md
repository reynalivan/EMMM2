# Epic 16: Preview Panel Layout & State

## 1. Executive Summary

- **Problem Statement**: The Preview Panel must dynamically adapt between three distinct states (Object summary, Single mod detail, Multi-select placeholder) based on what the user has selected in the objectlist and grid — without UI flicker, stale data, or wrong context after rapid selections.
- **Proposed Solution**: A `PreviewPanel` orchestrator that reads `selectedFolders` from Zustand — renders `ObjectSummary` when empty, `ModDetails` (with Metadata, INI, Gallery sub-sections) when exactly 1 is selected, and a "Multiple Items Selected" placeholder for ≥ 2 — with `react-resizable-panels` for width persistence.
- **Success Criteria**:
  - Panel state transition (click card → mod detail view) completes in ≤ 100ms (no backend call needed — data from React Query cache).
  - Panel collapse/expand animation ≤ 200ms.
  - Preview Panel width persisted in `localStorage` and restored in ≤ 50ms on mount.
  - Switching active game clears `selectedFolders` — panel reverts to empty state in ≤ 100ms.
  - Stale mod detail (for a mod just deleted) is never shown — panel reverts to ObjectSummary within ≤ 200ms of the `['folders']` cache invalidation.

---

## 2. User Experience & Functionality

### User Stories

#### US-16.1: Dual Context Rendering

As a user, I want the Preview Panel to show Object stats with no mod selected, and mod details when I click a folder card, so that contextually relevant information is always shown.

| ID        | Type        | Criteria                                                                                                                                                                                                |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-16.1.1 | ✅ Positive | Given an Object selected in the objectlist but no folder selected in the grid, then the Preview Panel shows `ObjectSummary`: object name, total mod count, enabled count, thumbnail if available           |
| AC-16.1.2 | ✅ Positive | Given I click a `FolderCard`, then the panel switches to `ModDetails` in ≤ 100ms, loading Metadata, INI, and Gallery sub-sections in parallel                                                           |
| AC-16.1.3 | ✅ Positive | Given I click empty grid space (deselect), then `selectedFolders` resets to `[]` and the panel reverts to `ObjectSummary` in ≤ 100ms                                                                    |
| AC-16.1.4 | ❌ Negative | Given the selected mod no longer exists in the `['folders']` cache (deleted externally), then the panel automatically reverts to `ObjectSummary` without showing a broken detail view or error boundary |
| AC-16.1.5 | ⚠️ Edge     | Given the user switches active game while a mod is selected, then `selectedFolders` clears and the panel shows the new game's `ObjectSummary` (or empty state if no Object is selected)                 |
| AC-16.1.6 | ✅ Positive | Given `ModDetails` is active, the panel header displays a prominent Enable/Disable switch directly connected to the `toggle_mod` command (Epic 13) for instant status management without using the grid |

---

#### US-16.2: Resizable & Collapsible Panel

As a user, I want to resize or collapse the Preview Panel to give the folder grid more space when I don't need mod details.

| ID        | Type        | Criteria                                                                                                                                                                                                      |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-16.2.1 | ✅ Positive | Given the divider between Grid and Preview Panel, when dragged, then the panel width updates at ≥ 60fps within bounds (min: 240px, max: 50% of workspace width) and persists in `localStorage`                |
| AC-16.2.2 | ✅ Positive | Given the collapse button on the Panel header, when clicked, then the Panel collapses to 0px (or minimum icon strip) with a CSS animation ≤ 200ms; clicking "expand" or the strip restores the previous width |
| AC-16.2.3 | ⚠️ Edge     | Given the OS window is resized narrower than Panel min + Grid min combined, then the Panel collapses automatically — the Grid never drops below its minimum                                                   |

---

#### US-16.3: Multi-Selection State

As a user, I want clear feedback when I have multiple mods selected, so that I understand why no single mod's details are shown.

| ID        | Type        | Criteria                                                                                                                                                                     |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-16.3.1 | ✅ Positive | Given `selectedFolders.length ≥ 2`, then the Preview Panel shows "{N} items selected" placeholder with available bulk action icons — no individual mod metadata is displayed |
| AC-16.3.2 | ⚠️ Edge     | Given the user goes from multi-select to single-select (deselects all but one), then the panel transitions to `ModDetails` for the remaining selected folder in ≤ 100ms      |

---

### Non-Goals

- No multi-window Preview Panel detach (secondary OS window) in this phase.
- Preview Panel does not persist selected folder across app restarts — always starts empty.
- No "Pin" or "Compare" mode for side-by-side mod details.

---

## 3. Technical Specifications

### Architecture Overview

```
PreviewPanel.tsx
  └── reads: useAppStore.selectedFolders (string[])
  ├── selectedFolders.length === 0 → <ObjectSummary objectId={selectedObjectId} />
  ├── selectedFolders.length === 1 → <ModDetails folderPath={selectedFolders[0]} />
  │   ├── <MetadataSection folderPath />  (Epic 17)
  │   ├── <IniEditorSection folderPath /> (Epic 18)
  │   └── <GallerySection folderPath />   (Epic 19)
  └── selectedFolders.length >= 2  → <MultiSelectPlaceholder count={selectedFolders.length} />

react-resizable-panels:
  Panel minSize=240px maxSize=50% → onLayout → localStorage['previewPanelWidth']
```

### Integration Points

| Component       | Detail                                                                                                                        |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Selection State | `useAppStore.selectedFolders: string[]` (folder_paths)                                                                        |
| Object Summary  | `useObject(selectedObjectId)` — from Epic 07 React Query cache, no extra IPC call                                             |
| Panel Width     | `react-resizable-panels` `onLayout` → `localStorage['previewPanelWidth']`                                                     |
| Child Sections  | Epic 17 (Metadata), Epic 18 (INI Viewer), Epic 19 (Gallery) — all receive `folderPath` prop                                   |
| Stale Guard     | `selectedFolders` is validated against `['folders', gameId, subPath]` cache on cache invalidation — stale entries are removed |

### Security & Privacy

- **Panel layout (width/collapse state) is stored in `localStorage`** — no mod paths or user data stored client-side.
- **`folderPath` passed to child sections is always sourced from the React Query cache** — never from URL params or user text input; no injection risk.
- **Safe Mode**: When Safe Mode is enabled and `selectedFolders[0]` resolves to a mod with `is_safe = false`, the panel reverts to `ObjectSummary` — mod details are not displayed.

---

## 4. Dependencies

- **Blocked by**: Epic 05 (Workspace Layout — panel container), Epic 14 (Bulk Ops — multi-select state), Epic 07 (Object List — `selectedObjectId`).
- **Blocks**: Epic 17 (Metadata Editor), Epic 18 (INI Viewer), Epic 19 (Image Gallery) — all mounted inside this panel.
