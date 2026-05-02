# 202603241905-mod-navigation-sync-stability

## Title
Mod Navigation Synchronization & UI Stability

## Context
Folder renames (e.g., toggling `DISABLED ` prefix) were causing "Not Found" errors and expensive synchronization loops because the frontend state became stale immediately after the disk change.

## Changes
- **Frontend (Store)**: Added `correctExplorerPath` to `useAppStore.ts` to manually sync `explorerSubPath`, breadcrumbs, and selected folder state during mutations.
- **Frontend (UI)**: Refined `FolderGridBanners` with compact, sticky styling and depth-specific labels ("Disabled Object Mods" for depth 1).
- **Backend (Listing)**: Updated `list_mod_folders_inner` to detect if the mod root directory itself is renamed/disabled, returning it as an `ancestor_disabled_by` lock rather than an error.
- **Cleanup**: Reverted unnecessary `ObjectList` banner integration and fixed Dashboard TypeScript/lint regressions.

## Impacted Files
- `src/stores/useAppStore.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src-tauri/src/services/explorer/listing.rs` (modified)
- `src/features/folder-grid/FolderGridBanners.tsx` (modified)
- `src/features/dashboard/Dashboard.tsx` (modified)
- `src/locales/en/grid.json` (modified)
- `src/features/object-list/ObjectList.tsx` (modified)
- `src/features/object-list/ObjectListBanners.tsx` (removed)

## Goal
Eliminate navigation flickers/errors during folder toggles and provide clearer, more compact feedback when directories are locked.

## Impact
- Instant navigation updates when renaming/toggling folders.
- Improved error resilience when the mod root folder is disabled manually.
- Cleaned up Dashboard and ObjectList code.
