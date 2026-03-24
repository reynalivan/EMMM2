## Finish runtime wiring cleanup for collections and corridor preview

### Context

- Collections, apply, and safe-mode switch were already using runtime snapshots, but legacy IPC/hooks and old refresh paths were still exposed.
- That left dead code, stale permission entries, and inconsistent BE→FE refresh behavior after save/apply/switch.

### Changes

- Removed legacy collection IPC surface for `get_collection_preview` and `get_corridor_overview`.
- Removed stale backend adapters and old active-state/signature helpers that were no longer used after the runtime-snapshot refactor.
- Removed unused frontend hooks for old collection/object/overview fetch paths.
- Added one shared runtime refetch helper so save-current, save-snapshot, apply, and safe-mode switch all refresh the same runtime/list/preview data.
- Added shared switch-preview query keys and invalidation coverage.
- Switched save modal preview to the same grouped object-aware renderer used by workspace/apply/switch preview.
- Removed leftover local-storage compatibility for pre-typed corridor selection state.
- Migrated backend integration tests away from legacy preview/overview APIs to runtime snapshot/runtime preview.

### Impacted Files

- `src/features/collections/queryKeys.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/components/SaveCollectionModal.tsx` (modified)
- `src/features/collections/components/ApplyCollectionModal.test.tsx` (modified)
- `src/features/collections/utils/invalidateCorridorRuntime.ts` (modified)
- `src/features/collections/utils/refetchCollectionRuntime.ts` (added)
- `src/features/collections/utils/groupMods.ts` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
- `src/stores/useAppStore.ts` (modified)
- `src/lib/corridorSelection.ts` (modified)
- `src/types/collection.ts` (modified)
- `src-tauri/src/commands/collections/collection_cmds.rs` (modified)
- `src-tauri/src/commands/collections/tests/collection_cmds_tests.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src-tauri/src/services/corridor_runtime.rs` (modified)
- `src-tauri/src/services/collections/mod.rs` (modified)
- `src-tauri/src/services/collections/types.rs` (modified)
- `src-tauri/src/services/collections/storage.rs` (modified)
- `src-tauri/src/services/collections/runtime_snapshot.rs` (modified)
- `src-tauri/src/services/collections/effective_state.rs` (modified)
- `src-tauri/src/services/collections/root_resolution.rs` (modified)
- `src-tauri/tests/collections_service.rs` (modified)

### Goal

- Collections/save/apply/switch now read and refresh from the same canonical runtime data, without keeping obsolete preview/overview APIs alive.

### Impact

- Save/apply/safe-mode flows now refetch runtime snapshot, collections list, and runtime preview through the same helper.
- Safe-mode preview cache is invalidated together with corridor runtime cache.
- Legacy frontend and backend code paths were removed, reducing drift risk and maintenance cost.
- No schema change was introduced in this cleanup pass.

### Notes

- Strict topbar state still comes only from `CorridorRuntimeSnapshot`; workspace selection remains separate UI state by design.
