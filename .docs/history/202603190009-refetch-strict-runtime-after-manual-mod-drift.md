## Refetch strict runtime after manual mod drift

### Context

- Active collection UI could stay named after manual mod enable/disable because folder mutations only invalidated corridor queries and did not force an immediate strict runtime refetch.
- The expected contract is that any manual drift from a named collection should immediately surface as `Unsaved Preset`.

### Changes

- Added a small helper to refetch the current corridor runtime using the current `safeMode`.
- Updated corridor-affecting folder mutations to eagerly refetch strict runtime after success:
  - single mod toggle
  - safe/unsafe toggle
  - bulk toggle
  - delete / restore / bulk delete
  - fallback sync path
- Added backend regression for named -> unsaved -> named transition after manual mod disable/re-enable.
- Added frontend hook regression to ensure `useToggleMod` triggers the strict runtime refetch helper.

### Impacted Files

- `src/features/collections/utils/refetchCurrentCorridorRuntime.ts` (added)
- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/hooks/useFolders.test.tsx` (modified)
- `src-tauri/tests/collections_service.rs` (modified)

### Goal

- Manual mod drift now updates strict active collection state immediately so topbar, collections page, and switch preview can fall to `Unsaved Preset` without waiting for a later refetch.

### Impact

- Slightly more eager runtime refetching after corridor-affecting mod mutations.
- No public API or schema changes.
- Existing optimistic folder UI behavior stays intact.

### Notes

- The backend regression uses a nested real mod root fixture so runtime root detection matches actual app behavior.
