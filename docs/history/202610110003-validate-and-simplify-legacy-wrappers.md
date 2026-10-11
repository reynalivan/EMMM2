# Validate and simplify legacy wrappers

## Context

The user authorized the seven audit candidates only after current caller,
contract and test validation, including untracked source.

## Changes

- Remove orphaned conflict toast/barrel, three bulk hooks and stale mocks.
- Consume required backend path rewrites directly; move empty/mixed-response
  coverage into the actual bulk hook tests.
- Consolidate General settings on the entity custom-theme hook and remove unused
  object wrappers, including the counts service left orphaned by hook removal.
- Move eleven raw-hash parsing tests to typed harvesting, then delete isolated
  compatibility APIs and two unused overlay trigger wrappers.
- Correct a native assertion to accept proven `not_required` reload diagnostics
  while still requiring successful publication and the actual F7 artifact.

## Impacted Files

Modified frontend:

- `src/features/mod-runtime/hooks/useBulkModMutations.ts`, `useBulkModMutations.test.tsx`
- `src/features/mod-runtime/index.ts`
- `src/widgets/mod-explorer/hooks/useFolderGrid.test.ts`
- `src/pages/settings/components/tabs/GeneralTab.tsx`, `GeneralTab.test.tsx`
- `src/widgets/object-sidebar/hooks/useObjectQueries.ts`
- `src/entities/game-object/api/objectService.ts`, `src/entities/game-object/index.ts`

Deleted:

- `src/features/scanner/components/ConflictToast.tsx`, `src/features/scanner/index.ts`
- `src/features/mod-runtime/utils/folderMutationPayloads.ts`, `folderMutationPayloads.test.ts`
- `src/widgets/mod-explorer/hooks/folderMutationPayloads.test.ts`
- `src/pages/settings/hooks/useCustomThemes.ts`

Backend/verification:

- `src-tauri/src/modules/automation/application/keyviewer/harvester.rs` (modified)
- `src-tauri/src/modules/automation/application/keyviewer/tests/harvester_tests.rs` (modified)
- `src-tauri/src/modules/system/application/app/post_apply.rs` (modified)
- `tests/e2e/specs/normal-sync-keyviewer.e2e.ts` (modified)
- `docs/plans/validated-cleanup-20261011/implementation_plan.md` (added)
- This history file (added)

## Goal and Impact

Reduce unsupported paths and duplicate cache definitions while preserving active
behavior and parsing coverage. IPC registrations, persisted migrations, journal
recovery and filesystem guards retain their contracts. No dependency changes.

## Verification

95 frontend tests and 120 Rust tests passed; three preexisting manual benchmarks
remain ignored. The 21 harvester tests passed before deletion after migration.
App/E2E typechecks, scoped lint/architecture/format, strict all-target Clippy,
production/private native builds and review passed.

Nine native cases passed: General theme selection, five lifecycle cases, and
three normal-sync/KeyViewer cases. An initial KeyViewer assertion failed because
publication was already unchanged and reload was not required. The revised test
checks native diagnostics and artifact contents; renderer failure warnings remain
mandatory. The focused native rerun passed; owned fixtures were cleaned after
native process shutdown.
