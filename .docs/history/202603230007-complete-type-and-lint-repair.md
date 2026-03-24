# 202603230007-complete-type-and-lint-repair

## Title

Complete Type and Lint Repair (Frontend & Backend)

## Context

Multiple TypeScript errors in frontend tests and components, and type inference/schema issues in the backend `bulk_ops.rs`, were preventing clean builds and test execution.

## Changes

- **Frontend Test Repairs**: Added missing `ObjectSummary` properties (`status`, `created_at`, `hash_db`, `custom_skins`, `metadata`, etc.) to mocks in `EditObjectModal.test.tsx` and `useObjectListHandlers.test.ts`.
- **Scanner Test Repairs**: Removed non-existent `modId` from `DupScanGroup` and added required `action` to `ResolutionError` in `dedupService.test.ts` and component tests.
- **Frontend Logic Fix**: Resolved null-to-non-nullable assignment issues in `EditObjectTabAuto.tsx` for custom skin fields.
- **Backend Schema Sync**: Updated `bulk_ops.rs` to use `INTEGER` status (1/0) instead of strings, aligning with the new database schema.
- **Type Inference**: Added explicit type hints to `map_err` closures in `bulk_ops.rs` and fixed transaction dereferencing to resolve `cannot infer type` errors.
- **Linting**: Fixed trailing newline warnings (EOF) in multiple Rust and TypeScript files.
- **Cleanup**: Removed unused imports in `bulk_ops.rs`.

## Impacted Files

- `src/features/object-list/EditObjectModal.test.tsx` (modified)
- `src/features/object-list/useObjectListHandlers.test.ts` (modified)
- `src/features/object-list/EditObjectTabAuto.tsx` (modified)
- `src/features/scanner/components/DuplicateReport.test.tsx` (modified)
- `src/features/scanner/components/ResolutionModal.test.tsx` (modified)
- `src/lib/services/dedupService.test.ts` (modified)
- `src-tauri/src/services/mods/bulk_ops.rs` (modified)
- `src-tauri/src/services/app/tests/app_service_tests.rs` (modified - lint)
- `src-tauri/src/services/objects/tests/query_tests.rs` (modified - lint)

## Goal

Establish a 100% build-stable and test-passing baseline for both Frontend and Backend, ensuring strict adherence to the modernized DB schema and typing.

## Impact

- Clean `pnpm exec tsc --noEmit` results.
- Clean `cargo check --tests` results.
- Zero known lint warnings in the modified areas.
