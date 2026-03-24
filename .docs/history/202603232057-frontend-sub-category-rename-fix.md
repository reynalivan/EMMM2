# Align Frontend with Modernized DB Schema

## Context

Recent backend modernization renamed `sub_category` to `sub_category` in the `objects` table and Rust structs. The frontend was still using the legacy name, causing potential IPC and UI grouping failures.

## Changes

- **Core Types**: Renamed `sub_category` to `sub_category` in `GameObject`, `ObjectSummary`, and mutation inputs (`src/types/object.ts`).
- **Modals**: Updated `CreateObjectModal.tsx` and `AutoSetupModal.tsx` to handle the new field name.
- **Hooks**: Updated `useEditObjectForm.ts` (Zod schema and default values) and `useObjectListVirtualizer.ts` (grouping logic).
- **Tests**: Refactored mock data in `EditObjectModal.test.tsx`, `useObjectListHandlers.test.ts`, and `useObjectListVirtualizer.test.ts`.

## Impacted Files

- `src/types/object.ts` (modified)
- `src/features/object-list/CreateObjectModal.tsx` (modified)
- `src/features/object-list/AutoSetupModal.tsx` (modified)
- `src/features/object-list/hooks/useEditObjectForm.ts` (modified)
- `src/features/object-list/useObjectListVirtualizer.ts` (modified)
- `src/features/object-list/EditObjectModal.test.tsx` (modified)
- `src/features/object-list/useObjectListHandlers.test.ts` (modified)
- `src/features/object-list/useObjectListVirtualizer.test.ts` (modified)

## Goal

Achieve full end-to-end field name consistency between the React frontend and the modernized SQLite/Rust backend.

## Impact

- Fixed potential data persistence issues when creating/editing objects.
- Restored correct UI grouping by sub-category in the virtualized list.
- Passed 72 unit/component tests in the `features/object-list` module.
