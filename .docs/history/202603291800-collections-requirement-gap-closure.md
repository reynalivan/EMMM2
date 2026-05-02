# Collections Requirement Gap Closure

## Context

Collections drifted from Epic 31 after refactors. Save-from-unsaved used snapshot clone semantics, apply modal still expected legacy missing-mod strings, and rename/update could return raw member counts instead of preview-tree counts.

## Changes

- Save flow now uses explicit save modes: current-state save vs snapshot clone.
- Active unsaved save routes through current corridor snapshot semantics, so saving replaces the unsaved active state with a named active collection.
- Apply modal now reads typed `MissingMods` payloads and shows the missing-path dialog before allowing `Skip & Apply`.
- Collection update summary now uses preview-tree display counts, matching list and preview semantics.
- Req-31 docs now mark undo as compatibility-only and remove apply/undo wording drift.

## Impacted Files

- `.docs/requirements/req-31-collections.md` (modified)
- `src/lib/appError.ts` (modified)
- `src/lib/bindings.ts` (modified)
- `src/features/collections/types.ts` (modified)
- `src/features/collections/CollectionsPage.tsx` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/components/CollectionList.tsx` (modified)
- `src/features/collections/components/SaveCollectionModal.tsx` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)
- `src/features/collections/components/CollectionList.test.tsx` (modified)
- `src/features/collections/components/ApplyCollectionModal.test.tsx` (added)
- `src-tauri/src/domain/collection.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)

## Goal

Collections behavior is aligned again with Epic 31 for save semantics, missing-mod apply flow, and preview-tree count semantics.

## Impact

- Active unsaved save no longer clones stale stored snapshots.
- Missing mods are surfaced through the modal flow instead of a generic toast path.
- Collection counts stay stable across list, preview, and rename/update responses.
- `undo_collection` remains available only as compatibility behavior, not product UX.
