# Runtime duplicate side-effects cleanup

## Context

Disk Reconcile was already the main runtime path, but several frontend and backend flows still duplicated invalidation, overlay refresh, and disabled-prefix logic.

## Changes

- Removed redundant collection/corridor overlay refresh calls in command handlers because apply/switch pipelines already run post-apply refresh.
- Reduced duplicate frontend refetches for runtime-file mutations that already emit `disk_reconcile:result`.
- Reordered manual Deep Match apply flows so canonical relation is written before metadata/thumbnail updates, then invalidated only where Disk Reconcile does not already cover it.
- Replaced remaining manual `DISABLED ` string stripping/toggling in runtime FE hooks with shared disabled-prefix utilities.
- Added boundary comments clarifying that boot-time passive startup is read-only and that physical renames only come from recovery/apply/switch flows.
- Updated collection/corridor hooks to write backend-authoritative corridor snapshots into query cache immediately after apply/undo/switch.

## Impacted Files

- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/object-list/ObjectList.tsx` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
- `src/App.tsx` (modified)
- `src/features/collections/components/RecoveryDialog.tsx` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/pipeline/apply_pipeline.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)

## Goal

Runtime mutations now have clearer ownership: Disk Reconcile handles runtime refresh, collection/corridor pipelines handle intended physical renames, and frontend global invalidation is no longer duplicated for the same event.

## Impact

- Fewer duplicate refetches for ObjectList, FolderGrid, corridor, and dashboard-related runtime updates.
- Immediate corridor snapshot cache alignment after apply/undo/switch.
- No business-rule change to collection apply or corridor switch rename behavior.
- Disabled-prefix handling in runtime FE hooks is now centralized in one utility.

## Notes

- Backend flows that do not emit `disk_reconcile:result` still keep their direct invalidation/dirty-state paths.
