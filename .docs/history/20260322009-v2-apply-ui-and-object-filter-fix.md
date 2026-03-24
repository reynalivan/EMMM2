### Title

Fix Apply Collection UI and Switch Pipeline Disabling

### Context

User reported multiple bugs: "Save Collection" modal was trapped in topbar layout, dirty collection states showed asterisk instead of "Unsaved Preset", Apply Collection dialog stood blank and applied 0 changes, and switching Safe/Unsafe modes incorrectly reported no active targets. Deeper audit discovered frontend arguments dropped, rendering constraints ignored, and a major backend flaw where depth-1 Object folders were incorrectly disabled during switch.

### Changes

- Replaced inline modal array with `createPortal(..., document.body)` in `SaveCollectionModal.tsx`.
- Refactored `ContextControls.tsx` to unconditionally output `[Unsaved Preset]` when dirty.
- Injected `isSafe: boolean` to `useApplyCollectionPreview` React Query to pass backend validation.
- Fixed `disable_leaving` in `switch_pipeline.rs`: Now processes relative folder paths using `strip_prefix(&mods_path)` to correctly determine `components().count() > 1`, completely avoiding disabling/renaming Object containers!

### Impacted Files

- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src/features/collections/components/SaveCollectionModal.tsx` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)

### Goal

To bring the entire Collection Apply & Mode Switch UI flows to 100% parity with V1 expectations, whilst blocking destructive backend folder operations from renaming root objects.

### Impact

The backend switch pipeline now strictly guards Object-level folders from being affected by mod-toggle loops. The frontend modals accurately capture and represent Context states across operations unconditionally.
