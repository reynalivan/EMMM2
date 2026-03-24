## Optimistic active collection after apply

### Context

- After applying a named collection, the topbar could still fall to `Unsaved Preset` before the strict runtime refetch settled.
- Users expect the freshly applied named collection to become the active topbar state immediately, unless the target is an unsaved snapshot.

### Changes

- `useApplyCollection` now seeds the current corridor runtime query cache with the target named collection snapshot immediately after apply succeeds.
- The optimistic cache seed uses the already-loaded target runtime preview and only applies to named collections.
- Strict runtime refetch still runs in the background to verify backend truth.
- Added a backend regression for named -> unsaved -> named transitions after manual mod disable/re-enable, and kept hook-level refresh coverage for manual drift.

### Impacted Files

- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)
- `src-tauri/tests/collections_service.rs` (modified)

### Goal

- Applying a named collection now updates the active collection UI immediately while preserving strict backend verification afterward.

### Impact

- Faster perceived topbar update after apply.
- Background refetch still corrects the UI if backend runtime verification disagrees.
- No schema or command changes.

### Notes

- Unsaved snapshot targets are excluded from the optimistic named-state seed.
