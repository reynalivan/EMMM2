# Implementation Plan: Filewatcher Sync & Preview Panel Fixes

## 1. Filewatcher Sync Gap (External Modifications Bug)

**Analysis**: During `apply_collection` (which is used both standalone and during corridor switches), the `ApplyContext` struct does not include a `WatcherState` or `Arc<AtomicBool>`. As a result, when `batch_rename.rs` rapidly renames mod folders, it triggers the background filewatcher, which then interprets these renames as external user modifications, potentially causing race conditions and database desyncs.

**Fix**:

1. Add `pub suppressor: Arc<AtomicBool>` to `ApplyContext` in `src-tauri/src/pipeline/apply_pipeline.rs`.
2. Wrap the `batch_rename::rename` step in a `SuppressionGuard::new(&ctx.suppressor)`.
3. Update `collection_service::apply_collection` to accept `suppressor: Arc<AtomicBool>` and pass it to `ApplyContext::new`.
4. Update `cmds.rs` `apply_collection` to pass `watcher_state.suppressor.clone()`.
5. Update `undo_collection` in `cmds.rs` and `collection_service.rs` to also pass the suppressor.
6. Update `switch_pipeline.rs` -> `restore_target` to pass `ctx.suppressor.clone()` into the embedded `apply_collection` call.

---

## 2. Apply Collection Preview Panel "Not Functional" Gap

**Analysis**: The `ApplyCollectionModal` relies on `useApplyCollectionPreview` which calls the `preview_apply_collection` rust command. The rust command requires `is_safe: bool` as an argument, but the frontend `invoke` call is missing this argument entirely. This causes the Tauri command to error out instantly, leaving the query stuck and the UI "unfunctional" (empty).

**Fix**:

1. Update `useApplyCollectionPreview` in `src/features/collections/hooks/useCollections.ts` to accept `isSafe: boolean` and include it in the `queryKey` and `invoke` payload.
2. Update `ApplyCollectionModal.tsx` to read `safeMode` from `useAppStore` and pass it to `useApplyCollectionPreview`.

---

## 3. Corridor Switch Preview Panel "Not Functional" Gap

**Analysis**: For `ModeSwitchConfirmModal`, the command `preview_corridor_switch` seems fully wired up and takes `targetSafe`. However, let's also verify that it isn't suffering from any UI layout breakdowns (e.g. empty target members when falling back).
_Question for User_: If fixing the above parameters does not make it "functional", do you mean that you want the ability to manually select/check specific mods inside the preview panel before confirming? (V2 currently mirrors exactly what will happen without individual checkboxes).

### Next Steps:

If you approve this plan, I'll execute the `SuppressionGuard` plumbing across the pipeline and fix the frontend `invoke` signatures to get the preview panels working properly.
