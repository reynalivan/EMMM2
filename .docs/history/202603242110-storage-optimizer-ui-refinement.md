# Storage Optimizer Navigation & Redirect Fixes

## Context

Fixed a regression where "Storage Optimizer" appeared twice in the TopBar menu and verified dashboard functionality after the feature migration.

## Changes

- **TopBar Navigation**: Removed the redundant manual shortcut for "Storage Optimizer" in `src/components/layout/top-bar/index.tsx`. It is now correctly handled solely via the `NAV_ITEMS` array.
- **Dashboard Verification**: Confirmed `src/features/dashboard/Dashboard.tsx` correctly redirects both the Quick Action tile and the Duplicate Waste alert to the `storage-optimizer` view.
- **Lint Cleanup**: Applied formatting fixes to `StorageOptimizerPage.tsx` to satisfy project coding standards and resolve ESLint/Prettier warnings.

## Impacted Files

- `src/components/layout/top-bar/index.tsx` (modified)
- `src/features/scanner/StorageOptimizerPage.tsx` (modified)
- `src/features/dashboard/Dashboard.tsx` (verified/finalized)

## Goal

Ensure a clean, single-entry navigation experience and verified functional entry points from the dashboard.

## Impact

- Removed UI clutter in the App Menu.
- Guaranteed all user entry points lead to the correct dedicated feature page.
