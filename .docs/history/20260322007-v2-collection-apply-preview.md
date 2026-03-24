# V2 Collection Apply Preview Modal

## Context

The user requested that the "Apply Collection" action show a preview modal before actually applying, exactly how V1 behaved.

## Changes

- Created new backend domain type `ApplyPreview`.
- Added new backend service `preview_apply` in `collection_service.rs` to fetch active vs target collection mods.
- Added new Tauri command `preview_apply_collection` returning the before/after preview.
- Registered command in `lib.rs` and added permission to `app-commands.toml`.
- Added React Query hook `useApplyCollectionPreview` to frontend.
- Built a dual-panel `ApplyCollectionModal.tsx` displaying the current state (Before) vs target state (After) using `ModGroupList`.
- Integrated `ApplyCollectionModal` into `ContextControls.tsx`.

## Impacted Files

- `src-tauri/src/domain/collection.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src/types/collection.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/queryKeys.ts` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (added)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)

## Goal

The application now requires user confirmation via a dual-panel preview before applying a collection/corridor switch, preventing accidental loss of current state and fulfilling parity with V1 behavior.

## Impact

- Adds one more click to applying a collection, but significantly improves safety.
- Utilizes the standardized V2 `ModGroupList` component for identical UI representation of active and target mods.
