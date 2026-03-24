## Modernizing Light Mode Aesthetics (Phase 2)

### Context

- Subtext and metadata labels in Light mode were too light, impacting readability.
- The overall look of Light mode needed to be more "modern and classy" per user request.

### Changes

- Refined the `light` theme in `App.css` to use a sophisticated **Slate-based** palette (`Slate 100/200` for backgrounds, `Slate 800` for accents).
- Systematically increased opacity and contrast for subtext and secondary labels across 7 key components (`TopBar`, `ContextControls`, `ExplorerEmptyState`, `FolderCard`, `DownloadsPage`, `GameSelector`).
- Increased standard subtext opacity from `40-60%` to `70-80%` in Light mode.
- Adjusted `glass-surface` for Light mode with a tighter border and slightly higher opacity background for better depth and definition.
- Reduced logo glow intensity for a subtler, more professional branding "lift".

### Impacted Files

- `src/App.css` (modified)
- `src/components/layout/top-bar/index.tsx` (modified)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)
- `src/components/layout/top-bar/GameSelector.tsx` (modified)
- `src/features/folder-grid/ExplorerEmptyState.tsx` (modified)
- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/downloads/DownloadsPage.tsx` (modified)

### Goal

- Ensure high-contrast readability and deliver a premium, modern Light mode experience.

### Impact

- Significant improvement in readability for all users, especially for metadata and secondary UI elements.
- Clean, professional aesthetic consistent with modern design standards.
