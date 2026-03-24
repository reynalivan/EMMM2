# V2 Filewatcher Sync and Preview Panel Fix

### Context

The backend `apply_collection` pipeline triggered a storm of Filewatcher "modified" events because it lacked an active `SuppressionGuard` during batch folder renames. Simultaneously, the frontend `ApplyCollectionModal` failed to render because the React Query hook omitted the required `isSafe` boolean argument, causing the Tauri command to panic.

### Changes

- Updated `ApplyContext` and `batch_rename.rs` to include `suppressor: Arc<AtomicBool>` and instantiate a `SuppressionGuard`.
- Re-wired `apply_collection` and `undo_collection` in `cmds.rs` to extract `WatcherState` and pass its suppressor down.
- Re-wired `switch_pipeline.rs`'s internal `apply_collection` call and `hotkeys/manager.rs`'s preset cycler to pass the suppressor.
- Updated `useCollections.ts` to include `isSafe` in the `preview_apply_collection` invoke.
- Updated `ApplyCollectionModal.tsx` to read `safeMode` from `useAppStore` and pass it to the hook.

### Impacted Files

- `src-tauri/src/pipeline/apply_pipeline.rs` (modified)
- `src-tauri/src/pipeline/steps/batch_rename.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/services/hotkeys/manager.rs` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)

### Goal

Ensure atomic loadout switching without triggering secondary overlapping DB sync threads (Filewatcher isolation), and restore visual functionality to the frontend previews.

### Impact

- Filewatcher races during Collection Applies are eliminated.
- Corridor Transition and Application pipelines now uniformly protect disk state.
- `ApplyCollectionModal` successfully fetches and displays the Before/After delta UI.
