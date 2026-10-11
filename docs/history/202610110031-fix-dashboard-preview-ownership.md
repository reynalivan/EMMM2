# Fix Dashboard and preview ownership

## Context

Async results and image/viewer work could outlive their original game or folder.
Unused preview API and UI fallbacks also duplicated active contracts.

## Changes

- Cancel obsolete Dashboard/wizard work; preserve originating callbacks and errors.
- Bind viewer reviews and image/form submissions to their original targets; update
  gallery and thumbnail caches together.
- Revalidate root/folder containment and identity after waits; retain the viewer
  lease through preparation and spawn.
- Remove unused useModInfo, DTO fallbacks, pie definitions and the flat preview
  command/helper; migrate native assertions to visibility-aware IPC.

## Impacted Files

Frontend (modified):

- `src/pages/dashboard/Dashboard.tsx`, `Dashboard.test.tsx`
- `src/pages/dashboard/components/DashboardCharts.tsx`,
  `DashboardIdentitySuggestions.tsx`, `DashboardIdentitySuggestions.test.tsx`
- `src/features/match-wizard/components/ObjectClassificationWizardHost.tsx`,
  `ObjectClassificationWizardHost.test.tsx`
- `src/features/mod-runtime/actions/modViewerExternalReviewState.ts`,
  `modViewerExternalReviewState.test.ts`, `useModContextMenuActions.ts`
- `src/features/mod-runtime/hooks/useFolderMutations.ts`
- `src/widgets/mod-preview/PreviewPanel.tsx`
- `src/widgets/mod-preview/hooks/usePreviewActions.ts`, `usePreviewData.ts`,
  `usePreviewData.test.ts`, `usePreviewRuntime.ts`
- `src/widgets/object-sidebar/hooks/useEditObjectForm.ts`
- `src/widgets/object-sidebar/modals/EditObjectModal.test.tsx`
- `src/demo/commands.ts`, `src/shared/api/tauri/bindings.gen.ts`

Frontend regression files (added):

- `src/features/mod-runtime/actions/modViewerExternalReviewState.mounted.test.tsx`,
  `modViewerReview.test-fixtures.ts`, `useModContextMenuActions.test.tsx`
- `src/features/mod-runtime/hooks/useFolderMutations.thumbnail.test.tsx`
- `src/widgets/mod-preview/hooks/usePreviewActions.test.tsx`
- `src/widgets/object-sidebar/hooks/useEditObjectForm.test.tsx`

Backend (modified):

- `src-tauri/src/platform/fs/guard.rs`, `tests/guard_tests.rs`
- `src-tauri/src/modules/library/adapters/tauri/mod_thumbnail_cmds.rs`,
  `preview_cmds.rs`
- `src-tauri/src/modules/library/application/mods/preview_ops.rs`,
  `tests/preview_ops_tests.rs`
- `src-tauri/src/modules/settings/adapters/tauri/settings_cmds.rs`
- `src-tauri/src/modules/mutation/coordinator.rs`
- `src-tauri/src/lib.rs`, `src-tauri/permissions/app-commands.toml`,
  `src-tauri/AGENT.md`

Native/docs:

- `tests/e2e/support/previewImages.ts` (added)
- `tests/e2e/specs/phase4-preview.e2e.ts`, `client-workspace-smoke.e2e.ts` (modified)
- `docs/plans/audit-dashboard-preview-fixes-20261011/implementation_plan.md`,
  `validation.md` (added)

## Goal and Impact

Stale work cannot redirect mutations or reviews to a new target. Existing recovery
and filesystem guards stay active. The deleted internal IPC has no remaining
caller. No dependency, migration, installed-app or real-library change.

67 frontend tests and 43 targeted Rust tests passed; one existing frontend skip
and one manual Rust benchmark remain. Types, scoped lint/format/architecture,
registry/export and independent review passed. Nine native Dashboard/preview
cases passed inside the 15-case fixture release capture; 1,577 inputs had zero
drift, owned processes stopped, fixtures were cleaned and installed DB was unchanged.
Real clipboard paste and external viewer execution remain outside native coverage.
