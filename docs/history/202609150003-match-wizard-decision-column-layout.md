# Match Wizard decision column layout

## Context

Long decision labels overflowed the fixed-height action buttons and overlapped nearby actions in the review table.

## Changes

- Widened the Decision column from `w-32` to `w-48`.
- Made decision buttons full-width, shrinkable, and content-sized so long labels wrap without overlap.

## Impacted Files

- `src/features/match-wizard/ImportBatchWizard.tsx` (modified)
- `src/features/match-wizard/components/ImportBatchWizardItemRow.tsx` (modified)
- `docs/history/202609150003-match-wizard-decision-column-layout.md` (added)

## Goal

Keep Proceed, Keep separately, Skip, and Retry readable and operable in the review table.

## Impact

No decision logic, IPC contract, or backend behavior changed. Rows with long labels may grow naturally to fit their content.

## Validation

- Focused Match Wizard tests passed: 11 tests.
- Targeted ESLint passed.
- TypeScript check and production build passed.
