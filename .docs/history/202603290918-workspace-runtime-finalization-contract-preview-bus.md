# Workspace Runtime Contract, Preview Runtime, and Query Bus Finalization

## Context

Runtime `mods` still had three structural gaps: backend semantics were partly English string literals, preview transition state still had a local fallback path, and non-`mods` feature refresh still used direct query invalidation.

## Changes

- Backend workspace semantics now emit structured reason and warning codes with payload args instead of raw English strings.
- Frontend runtime types and renderers now consume structured workspace reasons/warnings through shared formatting helpers.
- Preview runtime now owns workspace summary + lazy detail fetching, and preview unsaved transition uses the runtime machine instead of local pending-transition state.
- App query refresh bus was extended beyond `mods` and now handles settings, browser downloads/import queue, dedup, scanner, pins, dashboard, and browser homepage refresh scopes.
- Remaining direct `invalidateQueries(...)` calls were removed from feature code; only centralized infra layers still perform invalidation.
- `AutoSetupModal` now reuses the shared `DbEntryFull` type instead of maintaining a local duplicate shape.
- Broken tests were updated to match the current UI contract for preview title rendering, edit modal validation styling, and settings mutation behavior.
- Type mismatch in `WorkspaceExplorer.inactive_reason` was aligned with the new structured contract.

## Impacted Files

- `src-tauri/src/domain/workspace.rs` (modified)
- `src-tauri/src/services/workspace_service.rs` (modified)
- `src/types/workspace.ts` (modified)
- `src/features/workspace-runtime/workspaceSemantics.ts` (added)
- `src/features/preview/hooks/usePreviewRuntime.ts` (added)
- `src/features/preview/hooks/usePreviewPanelState.ts` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)
- `src/features/runtime-sync/queryRefresh.ts` (modified)
- `src/hooks/useSettings.ts` (modified)
- `src/features/browser/hooks/useDownloads.ts` (modified)
- `src/features/browser/hooks/useImportQueue.ts` (modified)
- `src/features/scanner/hooks/useDedup.ts` (modified)
- `src/features/scanner/ScannerFeature.tsx` (modified)
- `src/features/collections/hooks/usePin.ts` (modified)
- `src/features/dashboard/hooks/useDashboardStats.ts` (modified)
- `src/features/settings/tabs/BrowserTab.tsx` (modified)
- `src/features/settings/tabs/GamesTab.tsx` (modified)
- `src/App.tsx` (modified)
- `src/features/object-list/AutoSetupModal.tsx` (modified)
- `src/features/object-list/hooks/useMasterDbSync.ts` (modified)
- `src/locales/en/common.json` (modified)
- `src/locales/id/common.json` (modified)
- `src/locales/zh/common.json` (modified)
- `src/hooks/useSettings.test.ts` (modified)
- `src/features/object-list/EditObjectModal.test.tsx` (modified)
- `src/features/preview/PreviewPanel.test.tsx` (modified)
- `src/features/workspace-runtime/useWorkspaceViewModel.contract.test.ts` (verified)
- `src/features/preview/hooks/usePreviewPanelState.test.ts` (verified)
- `src/features/object-list/ObjectRowItem.test.tsx` (verified)
- `src/features/runtime-sync/queryRefresh.test.ts` (verified)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (verified)

## Goal

Workspace runtime now carries structured semantics from backend to frontend, preview transition state is machine-driven, and feature-level refresh runs through a unified scope bus instead of scattered invalidation calls.

## Impact

- User-facing inactive/warning text is now i18n-safe and derived from contract codes.
- Preview unsaved-change flow has fewer drift paths because the runtime machine owns the transition target.
- Query refresh behavior is more audit-able app-wide, with only centralized infra performing invalidation.
- No DB migration was required.

## Notes

- Heavy preview content remains lazy-loaded, but the orchestration moved behind preview runtime helpers.
- Centralized infra still owns low-level invalidation/removal for descriptor application and query scope publishing.
