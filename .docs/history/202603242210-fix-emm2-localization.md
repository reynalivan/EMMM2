# Fix EMM2 Frontend Localization

## Context

The EMM2 application had incomplete Indonesian translations and several UI areas (Downloads Manager, Empty States, Maintenance Success Messages) were using hardcoded English strings. Additionally, the `layout` namespace was missing from the i18n configuration, causing navigation items to fail translation.

## Changes

- **i18n Infrastructure**: Registered `layout` namespace in `src/lib/i18n.ts` and unified `action.cancel` in `common.json`.
- **Indonesian Coverage**: Expanded `id/settings.json`, `id/browser.json`, and `id/layout.json` to achieve 100% parity with English.
- **Component Refactoring**:
  - `DownloadsPage.tsx`: Replaced hardcoded status badges and action labels with `browser:downloads.*` keys.
  - `ExplorerEmptyState.tsx`: Replaced hardcoded "No Object Selected" text with `layout:empty.*` keys using `Trans` component.
  - `MaintenanceTab.tsx` & `useSettings.ts`: Refactored to handle raw counts from backend for localized success toasts.
- **Backend Commands**: Updated `run_maintenance` and `clear_old_thumbnails` in Rust to return `u64` counts instead of formatted English strings.

## Impacted Files

- `src-tauri/src/commands/app/settings_cmds.rs` (modified)
- `src-tauri/src/services/app/maintenance_service.rs` (modified)
- `src/lib/i18n.ts` (modified)
- `src/locales/en/common.json` (modified)
- `src/locales/id/common.json` (modified)
- `src/locales/en/layout.json` (modified)
- `src/locales/id/layout.json` (modified)
- `src/locales/id/settings.json` (modified)
- `src/locales/id/browser.json` (modified)
- `src/features/downloads/DownloadsPage.tsx` (modified)
- `src/features/folder-grid/ExplorerEmptyState.tsx` (modified)
- `src/features/settings/tabs/MaintenanceTab.tsx` (modified)
- `src/hooks/useSettings.ts` (modified)

## Goal

Achieve 100% localization coverage for the targeted pages and ensure a consistent multi-language user experience.

## Impact

- Sidebar and Navbar items now translate correctly in all supported languages.
- All Settings tabs are fully localized in Indonesian.
- Downloads Manager and Empty States are no longer hardcoded in English.
- Backend reporting is now localization-agnostic (returns raw data).
