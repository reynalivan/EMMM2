## Title

Object runtime descriptor cleanup and compatibility trim

## Context

The `mods` runtime still had object-side mutations and shared object handlers publishing refreshes manually, while `useObjects` still used its own refresh wrapper internally. That left object flows behind the newer descriptor-driven runtime model.

## Changes

- Moved object-side shared action refresh publishing to runtime descriptors.
- Moved ObjectList bulk object handlers to runtime descriptors instead of direct event publishing.
- Reduced `useObjects` internal mutation flows to descriptor-driven refresh publishing and left `refreshObjectListQueries()` as compatibility-only.
- Hardened the workspace store bridge so legacy tests that mock `useAppStore` non-hook style still work.

## Impacted Files

- `.docs/history/202603292255-object-runtime-descriptor-cleanup-and-compat-trim.md` (added)

- `src/hooks/useObjects.ts` (modified)
- `src/features/object-list/useObjHandlersBulk.ts` (modified)
- `src/features/workspace-runtime/actions/useSharedObjectActions.ts` (modified)
- `src/features/workspace-runtime/state/workspaceStoreBridge.ts` (modified)

## Goal

Object CRUD and bulk flows in the `mods` runtime now follow the same descriptor-first refresh direction as the newer mod-side runtime actions, and the remaining refresh wrapper is reduced to compatibility use.

## Impact

- Object mutations are less likely to drift from the runtime event bus.
- Compatibility tests keep working while the store bridge remains tolerant of older mocks.
- Breaking changes: none intended; changes are internal runtime orchestration cleanup.

## Notes

- `refreshObjectListQueries()` still exists for compatibility paths outside the main `mods` runtime.
- Repo-wide invalidation cleanup is still not complete; settings and some non-`mods` surfaces still use direct invalidation patterns.
