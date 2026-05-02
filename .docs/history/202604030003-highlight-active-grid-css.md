### Title

Implement Active Focus Highlights and Dynamic Grid Auto-fit layout

### Context

After migrating from auto-selection UX to explicit checkbox selection, clicking a mod removed the "Selected" state highlight causing users to lose track of what they clicked since the preview pane active highlight disappeared. Furthermore, the virtual grid used a rigid static `flex` wrapper setting fixed explicit `px` widths onto cards causing unnecessary complexity and occasional sub-pixel edge gaps against the true wrapper width.

### Changes

- Expose `selectedModPath` from `WorkspaceRuntimeState` store bridging layer into the grid root orchestrator `useFolderGrid`.
- Pipelined `isActive` into `FolderListRow` and `FolderCard` using strict equality matching with `selectedModPath`.
- Inverted `FolderCard` and `FolderListRow` tailwind styles to render primary borders and color overlays whenever `isActive` evaluates true preventing users from losing contextual tracking.
- Refactored `isGridView` virtual row wrapper from `flex gap-3` into `grid gap-3` combined with CSS Grid's `repeat(count, minmax(0,1fr))`.
- Purged outdated localized React mathematical inline styles forcing static widths onto individual cards.

### Impacted Files

- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)

### Goal

Ensure the active interface context remains fully visible regardless of selection checkbox-state semantics, whilst cleaning up TanStack virtualization structural boundaries substituting redundant flex-basis JS maths with pure reliable responsive nested CSS-Grids.

### Impact

- Active modifications reliably stand out structurally identical regardless of the explicitly bulk-selected status array.
- Virtual Rows structurally mirror flexbox stretch features fully distributing leftover pixels amongst the mod cards ensuring flush layout presentation (Auto-fit/Auto-fill pattern).
