# Fix Corridor Switch Blank Screen

## Context
Corridor switches (Safe <-> Unsafe) caused the UI to go blank (data flicker) and the filesystem watcher to stop without restarting.

## Changes
- **Query Stability**: Added `placeholderData: keepPreviousData` to `useCorridor` and `useCollections` to maintain UI state during refetches.
- **Watcher Lifecycle**: Added `safeMode` as a dependency to `useWatcherLifecycle` in `hooks.ts` to ensure restarts on corridor transitions.
- **Defensive Logging**: Added `console.log` to `useWatcherLifecycle` to track lifecycle events in the browser console.
- **UI Guard**: Refined `effectiveSelectedId` in `CollectionsPage.tsx` to handle fetching states correctly.

## Impacted Files
- `src/features/collections/hooks/useCorridor.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/file-watcher/hooks.ts` (modified)
- `src/features/file-watcher/ExternalChangeHandler.tsx` (modified)
- `src/features/collections/CollectionsPage.tsx` (modified)

## Goal
A seamless, flicker-free corridor transition with a reliably restarting filesystem watcher.

## Impact
- **Flicker-Free UI**: The collections list and preview panel stay stable during switches.
- **Watcher Reliability**: Filesystem synchronization now survives corridor transitions.
- **Observability**: Watcher start/stop cycles are now visible in the console.

## Notes
- `keepPreviousData` from `@tanstack/react-query` v5 was used as the best-practice for this scenario.
