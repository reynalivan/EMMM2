# Harden frontend reconcile consistency

## Context

Collection and runtime queries were retaining previous-game results while the next game's request was pending. Partial moves could rethrow before refreshing successful rewrites, a completed reconcile could leave progress active, and restoring Last Changes ignored returned path rewrites.

## Changes

- Added regression tests for previous-game collection and runtime object data remaining visible while the next game query is pending.
- Removed cross-query placeholder retention from the game-keyed collection and runtime hooks.
- Added regression coverage requiring successful move rewrites to refresh before partial failures surface.
- Added terminal-progress coverage and clear `Completed` progress without clearing user-action-required reconcile state.
- Applied restore-result path rewrites through the same workspace descriptor as normal collection apply.
- Added a RED test that requires successful move rewrites to publish a workspace refresh before a partial failure is rethrown.
- Added a RED test that requires terminal reconcile completion to clear running progress when rename confirmation is required.

## Impacted Files

- `src/features/collections/hooks/useCollections.test.ts` (modified)
- `src/features/collections/hooks/useCollectionRuntime.test.ts` (added)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/hooks/useCollectionRuntime.ts` (modified)
- `src/features/mod-runtime/operations/sharedOperations.test.ts` (modified)
- `src/features/file-watcher/reconcileProgress.test.ts` (modified)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/features/file-watcher/reconcileProgress.ts` (modified)

## Goal

Prevent stale cross-game data, refresh successful partial moves, and clear terminal reconcile progress without hiding required rename confirmation.

## Impact

The game-B loading state is empty rather than actionable with game-A data. Same-game query caching remains intact. Successful partial moves refresh before their error is surfaced, completion clears only running progress and leaves the result-driven confirmation state intact, and Last Changes restoration keeps selected paths synchronized with disk rewrites.
