# Collections System — UX Hardening & Dead Code Cleanup

## Context

Phase 2 & 3 of the Collections System Audit Remediation Plan (ref: audit `collections_system_audit.md`).
Phase 1 (N+1 query elimination) was completed in a prior session. This session handles UX gaps and dead code.

---

## Changes

### Phase 2 — Delete Confirmation Dialog & Apply Guard

- **Before**: Trash icon in `CollectionList.tsx` directly called `onDelete` with no confirmation. Apply button was also enabled on the active collection (fragile guard).
- **After**: Clicking trash now opens `DeleteCollectionModal.tsx` — a DaisyUI dialog with collection name, Cancel/Delete buttons, and loading state. Apply button is now also disabled via `|| isActive` guard, preventing self-apply.

### Phase 3 — Dead Code Removal

- **CollectionModRow.tsx** — fully orphaned component, deleted.
- **useUndoCollection.ts** — hook defined but never called anywhere, deleted.
- **`utils/`** — empty directory, removed.
- **`undo_collection` backend cmd** — marked `#[deprecated]` in `cmds.rs`; still registered in `lib.rs` for API stability.
- **`delete_collection` cmd** — removed unused `_game_id: String` parameter from command signature.
- **`useDeleteCollection` hook** — updated `mutationFn` to not pass `gameId` to `commands.deleteCollection` (matches new signature).

---

## Impacted Files

**Frontend**
- `src/features/collections/components/DeleteCollectionModal.tsx` (added)
- `src/features/collections/components/CollectionList.tsx` (modified)
- `src/features/collections/components/CollectionModRow.tsx` (deleted)
- `src/features/collections/hooks/useUndoCollection.ts` (deleted)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/utils/` (deleted — was empty)

**Backend (Rust)**
- `src-tauri/src/commands/collections/cmds.rs` (modified — remove `_game_id`, deprecate `undo_collection`)

---

## Goal

The Collections UI now asks the user to confirm before deleting a collection. The Apply button is hardened against self-apply. Four dead code items are removed, reducing surface area and build noise.

---

## Impact

- **UX**: Delete now requires explicit confirmation — prevents accidental data loss.
- **Apply Guard**: Safer against refactor regressions — the `isActive` guard is now part of the `disabled` prop directly rather than relying on conditional JSX rendering.
- **Dead Code**: ~250 lines of orphaned code removed.
- **Backend**: `delete_collection` API signature changed (removed `gameId`). Frontend updated to match.
- **Breaking Change**: Callers of `commands.deleteCollection` must no longer pass `gameId`. The Specta-generated bindings will need regeneration on next build.
- **Deprecation Warning**: `undo_collection` emits a Rust deprecation warning at compile time — expected and intentional.

---

## Notes

- `undo_collection` is still registered in `lib.rs` and `app-commands.toml` to avoid runtime errors (frontend might still reference it from stale bindings). Full removal can happen in a dedicated cleanup pass once confirmed no callers remain.
