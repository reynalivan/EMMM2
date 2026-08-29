# Add guided enabled-mod conflict resolution

## Context

The enabled-mod conflict dialog explained conflicts but could not resolve them. Users had to leave the dialog, locate each mod, and manually decide which participant to disable without seeing the combined impact across overlapping conflict groups.

## Changes

- Added a path-keyed decision model shared across all conflict groups.
- Added `Keep`, `Disable`, and `Open folder` actions for each participating mod.
- Added conservative guidance for potential conflicts with no automatic winner.
- Added resolved/unresolved group state and a global impact summary.
- Added an inline review step showing exact paths before bulk disable.
- Reused the existing bulk-toggle mutation and its workspace/conflict refresh contract.
- Preserved failed paths and errors for retry after partial success.
- Blocked duplicate actions and dialog close while a mutation is pending.
- Reset stale decisions when the dialog closes and reopens.
- Added translated EN, ID, and ZH labels and focused regression tests.

## Impacted Files

- `src/features/conflict-report/ConflictModal.tsx`
- `src/features/conflict-report/ConflictGroupCard.tsx`
- `src/features/conflict-report/ConflictResolutionSummary.tsx`
- `src/features/conflict-report/conflictResolution.ts`
- Related conflict modal and decision-model tests
- `src/features/launch-bar/LaunchBar.tsx`
- `src/locales/{en,id,zh}/scanner.json`
- `.docs/tasks/archives/2026-08-28-guided-conflict-resolution-design.md`
- `tasks/guided-conflict-resolution-{plan,todo}.md`

## Goal

Let users safely resolve conflicts at whole-mod scope without hidden automatic choices or direct INI/ShaderFixes rewriting.

## Verification

- Focused conflict, LaunchBar, and bulk-toggle suites: 4 files, 16 tests passed.
- Full Vitest: 140 files passed; 2 unrelated existing files failed (`FolderCard.test.tsx` safe-mode masking and `useSettings.test.ts` removed PIN API expectation).
- Full ESLint: 0 errors and 14 pre-existing warnings outside this feature.
- i18n lint: passed.
- Production build remains blocked by unrelated safe-mode/settings contract drift; no reported TypeScript error points to the guided conflict files.
