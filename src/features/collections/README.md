# Collections Feature

This feature manages named mod-state snapshots and the current collection runtime.

## Feature Boundaries & Architectural Overlap

The feature has two responsibilities:

1. **Collections**: Named snapshots of mod states (`CollectionSummary`, `CollectionPreview`).
2. **Collection runtime**: The disk-derived active state, baseline, missing members, and optional Last changes draft (`CollectionRuntimeSnapshot`).

SAFE/UNSAFE classification is display metadata. The global filter never changes collection contents or disk state.

### Export Barrel
Consumers outside this feature should import hooks from `src/features/collections/hooks/index.ts`.
