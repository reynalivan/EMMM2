# Storage Optimizer Dedicated Page

## Context

The Storage Optimizer (Duplicate Scanner) was previously embedded in the Settings Maintenance tab, which limited its usability and focus. The user requested a dedicated full-screen page similar to the Collections page.

## Changes

- **Backend/Store**: Added `storage-optimizer` to `WorkspaceView` in `useAppStore.ts`.
- **Frontend/Layout**: Integrated `StorageOptimizerPage` into `MainLayout.tsx` and added it to the main `TopBar` navigation.
- **Dedicated UI**: Created `StorageOptimizerPage.tsx` as a full-screen wrapper for the duplicate scanning feature with a premium design.
- **Navigation Shortcuts**: Updated the App Menu's "Dedup Scanner" shortcut to point to the new dedicated page.
- **Maintenance Tab**: Replaced the embedded scanner in `MaintenanceTab.tsx` with a "Redirect" card.

## Impacted Files

- `src/stores/useAppStore.ts` (modified)
- `src/components/layout/MainLayout.tsx` (modified)
- `src/components/layout/top-bar/index.tsx` (modified)
- `src/features/settings/tabs/MaintenanceTab.tsx` (modified)
- `src/features/scanner/StorageOptimizerPage.tsx` (added)

## Goal

Provide a focused, spacious, and premium environment for mod library optimization and storage management.

## Impact

- Improved UX by giving the duplicate scanner more screen real estate.
- Reduced clutter in the Settings menu.
- Standardized navigation patterns by making Storage Optimizer a first-class citizen alongside Collections and Dashboard.
