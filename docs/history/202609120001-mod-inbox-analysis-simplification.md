# Mod Inbox atomic analysis and review gate

## Context

Import analysis could persist intermediate fields, use a boolean-style review signal, and repeatedly scan installed payloads during one batch.

## Changes

- Analysis now persists one typed result in a SQLite transaction, advancing its revision once and clearing stale acknowledgement.
- Typed review reasons gate bulk actions and backend commit acknowledgement.
- Target manifests are reused in memory per batch and revalidated from metadata; commit hashes source payloads once inside its lock.
- Review reasons, target comparisons, and duplicate context are shown in the import wizard.

## Impacted Files

- `src-tauri/build.rs` (modified)
- `src-tauri/migrations/20260912000001_import_review_gate.sql` (added)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/test_utils.rs` (modified)
- `src-tauri/src/modules/ingestion/application/import_batch/{types,analyze,coordinator,mod,target_manifest_index}.rs` (modified/added)
- `src-tauri/src/modules/ingestion/adapters/sqlite/import_batch/{mod,tests}.rs` (modified)
- `src-tauri/src/modules/ingestion/adapters/tauri/tauri.rs` (modified)
- `src-tauri/src/modules/mutation/application/workspace_mutation/import_commit.rs` (modified)
- `src/features/match-wizard/{ImportBatchWizard.tsx,ImportBatchWizard.test.tsx,components/ImportBatchWizardItemRow.tsx}` (modified)
- `src/shared/api/tauri/bindings.gen.ts` and `src/shared/i18n/locales/{en,id,zh}/match_wizard.json` (modified)

## Goal

The preview and commit paths now require an acknowledged, current analysis result and avoid unnecessary target hashing during a batch.

## Impact

Existing unfinished batches must be analyzed again before commit. Target cache is process-local and clears when a batch reaches a terminal state or is cancelled.

## Notes

Target cache is deliberately not persisted; commit still revalidates source and target state under the operation lock.
