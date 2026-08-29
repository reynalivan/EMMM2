# Browser Inbox Unification

## Title
Unified Browser Downloads into Mod Inbox

## Context
The user found the dual-path download flow (Browser Downloads -> Match Wizard / Import Queue -> Mod Inbox) to be confusing and bloated. They requested that files downloaded via the internal browser should go straight to the active game's Mod Inbox, removing the need for a separate queue or manual import prompts.

## Changes
- Updated compute_download_path and get_downloads_root in the backend. Downloads now resolve the active game from app_settings and route directly to its ModInboxRoot (a flat structure), falling back to a global BrowserDownloads folder if no game is active.
- Removed the import_service from the backend entirely. Files are now naturally picked up by the existing mod_inbox_watcher upon download completion.
- Removed the auto_import and browser_import_selected backend endpoints.
- Removed GamePickerModal and ImportQueuePanel components from the frontend since files auto-route.
- Removed bulk selection checkboxes and import action buttons from the DownloadManagerPanel.
- Cleaned up now-unused settings from BrowserTab and their corresponding state variables in useBrowserStore.

## Impacted Files
- src-tauri/src/services/browser/browser_service/paths.rs (modified)
- src-tauri/src/services/browser/download_service.rs (modified)
- src-tauri/src/services/browser/import_service/* (removed)
- src-tauri/src/commands/browser/browser_cmds.rs (modified)
- src-tauri/src/lib.rs (modified)
- src-tauri/src/services/app/bootstrap.rs (modified)
- src/features/browser/components/BrowserPage.tsx (modified)
- src/features/browser/components/DownloadManagerPanel.tsx (modified)
- src/features/browser/components/DownloadManagerPanel.test.tsx (modified)
- src/features/browser/components/DownloadsPage.tsx (modified)
- src/features/browser/components/GamePickerModal.tsx (removed)
- src/features/browser/components/GamePickerModal.test.tsx (removed)
- src/features/browser/components/ImportQueuePanel.tsx (removed)
- src/features/browser/hooks/useImportQueue.ts (removed)
- src/features/settings/components/tabs/BrowserTab.tsx (modified)
- src/core/lib/runtimeEffects.ts (modified)
- src/features/runtime-sync/queryRefresh.ts (modified)

## Goal
A highly streamlined download UX where users browse mods, click download, and the mod immediately appears in their Game's Mod Inbox, completely bypassing any intermediate queueing or wizard prompts.

## Impact
- Significantly lighter UI (removed Modals, Checkboxes, and Toolbar clutter).
- Complete elimination of the redundant import_service codebase (reduced total SLOC).
- Faster time-to-play since mods hit the Inbox instantly via the existing Watcher infrastructure.
