# ESLint Validation and Match Method Fallback

## Context

Repository-wide ESLint validation exposed formatting warnings and a TypeScript
build failure caused by optional generated destination match methods.

## Changes

- Applied ESLint autofixes for Prettier warnings across the linted source.
- Default missing destination match methods to `no_name_match` in both Match
  Wizard presentation paths.
- Updated Match Wizard tests to assert the current compact-path, rename, and
  recovery-action UI contracts.

## Impacted Files

- Linted source files with Prettier autofixes (modified)
- `src/features/match-wizard/components/ImportBatchWizardDestinationPanel.tsx` (modified)
- `src/features/match-wizard/components/ImportBatchWizardItemRow.tsx` (modified)
- `src/features/match-wizard/ImportBatchWizard.test.tsx` (modified)

## Goal

The frontend builds successfully while safely rendering destination suggestions
that omit a match method.

## Impact

ESLint has no errors; twelve existing `max-lines` warnings remain as a future
refactoring task. Full unit tests and production build pass.
