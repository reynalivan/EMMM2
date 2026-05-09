# Epic 16: Preview Panel Layout & State

## 1. Executive Summary

- **Problem Statement**: The Preview Panel must dynamically adapt between three distinct states (Object summary, Single mod detail, Multi-select placeholder) based on what the user has selected in the objectlist and grid — without UI flicker, stale data, or wrong context after rapid selections.
- **Proposed Solution**: A `PreviewPanel` runtime consumer that reads its selected node and summary from the shared `WorkspaceViewModel` + workspace machine. It renders object summary when no mod is selected and mod details when a single runtime-selected node is active, while heavy detail sections load lazily through preview-runtime hooks.
- **Success Criteria**:
  - Panel state transition (click card → mod detail view) completes in ≤ 100ms (no backend call needed — data from React Query cache).
  - Panel collapse/expand animation ≤ 200ms.
  - Preview Panel width persisted in `localStorage` and restored in ≤ 50ms on mount.
  - Switching active game clears runtime preview selection — panel reverts to object summary or empty state in ≤ 100ms.
  - Stale mod detail (for a mod just deleted) is never shown — panel reverts to ObjectSummary within ≤ 200ms of the Disk Reconcile result invalidating folder/detail queries.

---

## 2. User Experience & Functionality

### User Stories

#### US-16.1: Dual Context Rendering

As a user, I want the Preview Panel to show Object stats with no mod selected, and mod details when I click a folder card, so that contextually relevant information is always shown.

| ID        | Type        | Criteria                                                                                                                                                                                                  |
| --------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-16.1.1 | ✅ Positive | Given an Object selected in the objectlist but no folder selected in the grid, then the Preview Panel shows `ObjectSummary`: object name, total mod count, enabled count, thumbnail if available          |
| AC-16.1.2 | ✅ Positive | Given I click a `FolderCard`, then the panel switches to `ModDetails` in ≤ 100ms, loading Metadata, INI, and Gallery sub-sections in parallel                                                             |
| AC-16.1.3 | ✅ Positive | Given I click empty grid space (deselect), then runtime preview selection clears and the panel reverts to `ObjectSummary` in ≤ 100ms                                                                      |
| AC-16.1.4 | ❌ Negative | Given the selected mod no longer exists after Disk Reconcile refresh (deleted externally), then the panel automatically reverts to `ObjectSummary` without showing a broken detail view or error boundary |
| AC-16.1.5 | ⚠️ Edge     | Given the user switches active game while a mod is selected, then runtime preview selection clears and the panel shows the new game's `ObjectSummary` (or empty state if no Object is selected)           |
| AC-16.1.6 | ✅ Positive | Given `ModDetails` is active, the panel header displays a prominent Enable/Disable switch connected to the shared workspace switch engine for instant status management without using the grid            |

---

#### US-16.2: Resizable & Collapsible Panel

As a user, I want to resize or collapse the Preview Panel to give the folder grid more space when I don't need mod details.

| ID        | Type        | Criteria                                                                                                                                                                                                      |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-16.2.1 | ✅ Positive | Given the divider between Grid and Preview Panel, when dragged, then the panel width updates at ≥ 60fps within bounds (min: 240px, max: 50% of workspace width) and persists in `localStorage`                |
| AC-16.2.2 | ✅ Positive | Given the collapse button on the Panel header, when clicked, then the Panel collapses to 0px (or minimum icon strip) with a CSS animation ≤ 200ms; clicking "expand" or the strip restores the previous width |
| AC-16.2.3 | ⚠️ Edge     | Given the OS window is resized narrower than Panel min + Grid min combined, then the Panel collapses automatically — the Grid never drops below its minimum                                                   |

---

#### US-16.3: Runtime Selection State

As a user, I want the Preview Panel to follow the canonical runtime selection model, so that detail rendering never drifts from ObjectList and FolderGrid.

| ID        | Type        | Criteria                                                                                                                                                                  |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-16.3.1 | ✅ Positive | Given runtime preview selection is empty, then the Preview Panel shows object summary or empty state — no stale mod detail is displayed                                   |
| AC-16.3.2 | ⚠️ Edge     | Given runtime selection transitions rapidly between two mods, then the panel follows the final machine-selected node in ≤ 100ms without showing stale intermediate detail |

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
  └── usePreviewRuntime() → WorkspaceViewModel.preview + heavy detail queries
  ├── no selected runtime mod → <ObjectSummary />
  ├── selected runtime mod → <ModDetails folderPath={selected_path} />
  │   ├── <MetadataSection />  (Epic 17)
  │   ├── <IniEditorSection /> (Epic 18)
  │   └── <GallerySection />   (Epic 19)
  └── preview transitions + unsaved changes handled by workspace machine

ResizableWorkspace:
  right panel min width 240px, max constrained by grid minimum → persisted through the app store's debounced localStorage
```

### Integration Points

| Component       | Detail                                                                                                                       |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Selection State | `WorkspaceViewModel.preview.selected_node` + workspace runtime machine                                                       |
| Object Summary  | `useObject(selectedObjectId)` — from Epic 07 React Query cache, no extra IPC call                                            |
| Panel Width     | `ResizableWorkspace` right-panel drag state → app store debounced localStorage persistence                                   |
| Child Sections  | Epic 17 (Metadata), Epic 18 (INI Viewer), Epic 19 (Gallery) — all receive `folderPath` prop                                  |
| Stale Guard     | Runtime preview selection is validated against workspace refresh and preview detail invalidation — stale entries are removed |

### Security & Privacy

- **Panel layout (width/collapse state) is stored in `localStorage`** — no mod paths or user data stored client-side.
- **`folderPath` passed to child sections is always sourced from the React Query cache** — never from URL params or user text input; no injection risk.
- **Safe Mode**: When Safe Mode is enabled and the selected runtime mod resolves to hidden corridor content, the panel reverts to object summary — mod details are not displayed.

---

## 4. Dependencies

- **Blocked by**: Epic 05 (Workspace Layout — panel container), Epic 14 (Bulk Ops — multi-select state), Epic 07 (Object List — `selectedObjectId`).
- **Blocks**: Epic 17 (Metadata Editor), Epic 18 (INI Viewer), Epic 19 (Image Gallery) — all mounted inside this panel.
