# Centralized Formatting Utilities & Vite Fix

## Context

`DuplicateTable.tsx` was failing to load due to a missing `src/utils/formatters.ts` file. Additionally, multiple components were using duplicated inline `formatBytes` logic.

## Changes

- Created `src/utils/formatters.ts` with centralized `formatBytes` and `formatSize` alias.
- Created `src/utils/formatters.test.ts` for logic verification.
- Refactored 6 components to remove inline duplication and use the centralized utility.

## Impacted Files

- `src/utils/formatters.ts` (added)
- `src/utils/formatters.test.ts` (added)
- `src/features/scanner/components/DuplicateTable.tsx` (resolved import)
- `src/features/dashboard/Dashboard.tsx` (modified)
- `src/features/folder-grid/ConflictResolveDialog.tsx` (modified)
- `src/features/settings/tabs/UpdateTab.tsx` (modified)
- `src/features/file-management/TrashManagerModal.tsx` (modified)
- `src/features/downloads/DownloadsPage.tsx` (modified)
- `src/features/browser/components/DownloadManagerPanel.tsx` (modified)

## Goal

Fix Vite transform error and adhere to DRY/Single Truth principles.

## Impact

- Build error resolved.
- 100% consistency in byte formatting across the app.
- Reduced code duplication.
