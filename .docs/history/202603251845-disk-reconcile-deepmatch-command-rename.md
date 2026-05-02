# Disk Reconcile and Deep Match Scanner Rename

## Context

Runtime disk truth and canonical matching were still named too similarly, which made watcher/runtime flows easy to confuse with the explicit deep matching pipeline.

## Changes

- Renamed the public runtime command and result types to the Disk Reconcile terminology.
- Renamed the public scan/import commands to Deep Match Scanner terminology.
- Updated frontend bindings, scan service wrappers, watchers, onboarding, and object-list repair flows to use the new names.
- Renamed the frontend runtime event channel to `disk_reconcile:result`.
- Added inline boundary comments so watcher/focus/mods-entry paths are clearly Disk Reconcile only, while canonical matching stays in the Deep Match Scanner flow.

## Impacted Files

- `src-tauri/src/services/runtime_sync/types.rs` (modified)
- `src-tauri/src/services/runtime_sync/reconcile.rs` (modified)
- `src-tauri/src/services/runtime_sync/orchestrator.rs` (modified)
- `src-tauri/src/commands/scanner/runtime_sync_cmds.rs` (modified)
- `src-tauri/src/commands/scanner/sync_cmds.rs` (modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/src/services/scanner/sync/preview.rs` (modified)
- `src-tauri/src/services/browser/import_service.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src/lib/bindings.ts` (modified)
- `src/lib/services/scanService.ts` (modified)
- `src/lib/services/scanService.test.ts` (modified)
- `src/stores/useAppStore.ts` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/file-watcher/ExternalChangeHandler.tsx` (modified)
- `src/features/onboarding/WelcomeScreen.tsx` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/useObjHandlersArchive.ts` (modified)
- `src/features/object-list/useObjHandlersDrop.ts` (modified)
- `src/features/object-list/useObjectListHandlers.test.ts` (modified)
- `src/features/settings/tabs/GamesTab.tsx` (modified)
- `src/features/settings/tabs/GamesTab.test.tsx` (modified)

## Goal

The app now exposes two clearly separated public flows:
- Disk Reconcile for filesystem truth synchronization
- Deep Match Scanner for explicit canonical matching/import

## Impact

- Breaking change: old public command names and frontend wrappers were removed.
- Watcher, focus, mods-entry, onboarding-final-refresh, and manual repair now read as Disk Reconcile flows.
- Full scan/import flows now read as Deep Match Scanner flows.
- Performance behavior is unchanged; this is primarily a naming and boundary-clarity refactor.

## Notes

- `cargo check` and `pnpm exec tsc --noEmit` passed after the rename.
- `hooks.test.ts`, `scanService.test.ts`, and `GamesTab.test.tsx` passed after updating the renamed APIs and i18n-key expectations.
