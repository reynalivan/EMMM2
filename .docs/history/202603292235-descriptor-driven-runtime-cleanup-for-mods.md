## Title

Descriptor-driven cleanup for mods runtime refresh and watcher selection

## Context

The `mods` runtime still had several high-traffic mutation paths using mixed refresh styles: direct query invalidation, thumbnail query removal, legacy object-list refresh wrappers, and watcher selection updates that bypassed the workspace machine.

## Changes

- Extended runtime effect descriptors to cover targeted query invalidation and query removal.
- Moved preview detail mutations to descriptor-driven query invalidation plus runtime refresh publishing.
- Replaced several `useFolderMutations` refresh/remove flows with runtime descriptors and refresh-bus publishing.
- Moved watcher path rewrite and stale-selection cleanup to workspace runtime events instead of direct selection store setters.
- Added test coverage for descriptor-driven query invalidation/removal effects.

## Impacted Files

- `.docs/history/202603292235-descriptor-driven-runtime-cleanup-for-mods.md` (added)

- `src/features/workspace-runtime/optimistic/descriptor.ts` (modified)
- `src/features/workspace-runtime/optimistic/descriptorBuilders.ts` (modified)
- `src/features/workspace-runtime/optimistic/applyOptimisticEffects.ts` (modified)
- `src/features/workspace-runtime/optimistic/applyOptimisticEffects.test.ts` (modified)

- `src/features/preview/hooks/usePreviewData.ts` (modified)
- `src/features/preview/hooks/usePreviewData.test.ts` (modified)

- `src/hooks/useFolderMutations.ts` (modified)
- `src/hooks/useFolders.ts` (modified)

- `src/features/file-watcher/hooks.ts` (modified)

## Goal

The main `mods` runtime now routes more mutation side effects through one descriptor shape, and watcher-driven selection repair now respects the workspace state machine instead of mutating selection fields directly.

## Impact

- Preview detail updates, thumbnail changes, and several folder mutations are more consistent with the runtime event bus.
- Watcher rename/delete reconciliation is less likely to drift from runtime selection state.
- Breaking changes: none intended; this is internal cleanup of runtime orchestration.

## Notes

- This does not remove every remaining legacy refresh path in the repo yet. It focuses on the highest-traffic `mods` runtime flows and leaves broader compatibility cleanup for the next phase.
