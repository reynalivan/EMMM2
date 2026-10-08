# Fix import clarification state

## Context

Changing from a conflicted target to a clean target could retain the first target's conflict state. Items awaiting a category also had no explicit category control.

## Changes

- Clear target-specific comparison and review reasons when a later destination decision succeeds.
- Add an explicit category selector and confirmation action for items awaiting classification.
- Stop rename and retry actions from silently assigning the first suggested category or Other.
- Keep E2E workspace settlement focused on stable published runtime state after watcher debounce.

## Impacted Files

- src-tauri/src/modules/ingestion/application/import_batch/coordinator.rs (modified)
- src-tauri/src/modules/ingestion/adapters/sqlite/import_batch/mod.rs (modified)
- src-tauri/src/modules/ingestion/application/import_batch/tests.rs (modified)
- src-tauri/src/modules/ingestion/adapters/sqlite/import_batch/tests.rs (modified)
- src/features/import-batches/ImportBatchWizardHost.tsx (modified)
- src/features/match-wizard/ImportBatchWizard.tsx (modified)
- src/features/match-wizard/components/ImportBatchWizardItemRow.tsx (modified)
- src/features/match-wizard/ImportBatchWizard.test.tsx (modified)
- tests/e2e/support/data.ts (modified)

## Goal

The clarification flow keeps decisions bound to the currently selected target and requires a user-confirmed category before matching continues.

## Impact

No schema or API contract changes. Existing non-target review reasons remain intact when the target changes.
