# Frontend Type Synchronization

## Context

Migrated `game_type` and `status` from string-based values to numeric enums to align with the backend schema refactor. Added support for `hash_db` and `custom_skins` fields in the local database.

## Changes

- **Enums**: Migrated `GameType` and `ItemStatus` to numeric enums (0, 1) in `bindings.ts`.
- **Scan Service**: Refactored `scanService.ts` to use `GameType` enum for all matching and syncing commands.
- **Object Types**: Updated `GameObject` and `ObjectSummary` with `status`, `hash_db`, and `custom_skins`.
- **Edit Modal**: Added a status toggle (Enabled/Disabled) to `EditObjectTabManual.tsx` and synchronized form handling in `useEditObjectForm.ts`.
- **Create Modal**: Defaulted new manual objects to `ItemStatus.Enabled`.

## Impacted Files

- `src/lib/bindings.ts` (modified)
- `src/lib/services/scanService.ts` (modified)
- `src/hooks/useObjects.ts` (modified)
- `src/types/object.ts` (modified)
- `src/features/object-list/EditObjectModal.tsx` (modified)
- `src/features/object-list/EditObjectTabManual.tsx` (modified)
- `src/features/object-list/hooks/useEditObjectForm.ts` (modified)
- `src/features/object-list/hooks/useMasterDbSync.ts` (modified)
- `src/features/object-list/useObjHandlersArchive.ts` (modified)
- `src/features/object-list/hooks/useObjHandlersBulk.ts` (modified)

## Goal

Achieve full type safety and alignment between the backend and frontend for object metadata and status management.

## Impact

- Solidified type safety in synchronization and CRUD workflows.
- Improved performance and consistency by using numeric identifiers.
- Enabled persistent storage of MasterDB metadata (`hash_db`, `custom_skins`) within the local DB.
