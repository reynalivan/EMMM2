# Fix audit follow-up and remove unused pipelines

## Context

The user authorized all nine findings and the verified cleanup. Preserve the
existing shared workspace and unrelated concurrent changes.

## Changes

- Route drops by the outer grid and derive object-relative destinations; dispose
  late listeners. Native evidence also corrected game-relative object lookup.
- Enforce randomizer locks throughout selection/payloads, settle invalidated
  reviews, and make the clipped confirmation footer reachable by scrolling.
- Cancel pre-commit collection reads before refresh; persist ordinary staging
  failures as retryable state. Folder copies poll cancellation and retain metadata.
- Fix Windows physical mount matching. Move archive coverage to active staging
  before removing the unused direct pipeline, exclusive helpers and frontend exports.

## Impacted Files

Modified frontend (paths grouped):

- `src/features/randomizer/{RandomizerModal.tsx,RandomizerModal.test.tsx}`
- `src/pages/collections/hooks/{useCollections.ts,useCollections.test.ts}`
- `src/shared/lib/hooks/useFileDrop.ts`; `src/shared/lib/dropClassification.ts`
- `src/features/workspace-runtime/state/workspaceDialogs.ts`
- `src/widgets/mod-explorer/FolderGrid.tsx`
- `src/widgets/mod-explorer/hooks/{useFolderGrid.ts,useFolderGrid.test.ts,useFolderGridImport.ts,useFolderMutations.ts}`
- `src/widgets/mod-explorer/components/FolderCardContextMenu.test.tsx`
- `src/widgets/object-sidebar/utils/dropUtils.test.ts`
- `src/shared/i18n/locales/{en,id,zh}/common.json`

Added frontend tests: `src/shared/lib/hooks/useFileDrop.test.tsx` and
`src/widgets/mod-explorer/hooks/useFolderGridImport.test.tsx`.

Deleted: `src/pages/{dashboard,mod-inbox}/index.ts`,
`src/widgets/{mod-preview,object-sidebar}/index.ts`, and
`src-tauri/src/modules/library/application/mods/archive/extract.rs`.

Modified backend:

- `src-tauri/src/modules/ingestion/application/import_batch/{staging.rs,tests.rs}`
- `src-tauri/src/modules/library/application/mods/archive/{classify.rs,destination.rs,mod.rs,progress.rs,staging.rs,types.rs,tests/mod_tests.rs}`
- `src-tauri/AGENT.md`

Added: `tests/e2e/specs/audit-followup.e2e.ts`, this history and
`docs/plans/audit-followup-fixes-20261011/{implementation_plan.md,validation.md}`.

## Goal and Impact

Fewer unsupported paths, correct user intent and retry/cancellation behavior.
No dependency, migration, IPC-registry, journal, branch or commit changes.
Synthetic native fixtures only; real GIMI remains outside this task.

## Verification

99 Rust tests passed; two manual benchmarks ignored. The frontend suite passed
79 tests, plus a subsequently added relative-path regression passed after RED.
Randomizer after the CSS change passed 18 tests. App/E2E types, strict scoped lint,
architecture lint, shared all-target Clippy and fresh private builds passed.

Native focused runs passed thirteen distinct cases: six audit UI/IPC/disk cases,
three imports, three sync/KeyViewer cases, and one case with three process restarts.
The two final randomizer cases passed after the scroll fix; 1,422 build input
hashes had zero drift and owned native processes/fixtures were cleaned. The invalid
readonly-bridge test instrumentation was removed; pending-response ordering is
covered by deferred frontend regressions. See the validation report for versions,
initial failures, source integrity and native limits.
