# Stabilize Active Conflict Review

## Context

An uncommitted Keep/Disable selection could survive after its conflict group was ignored or restored.

## Changes

- Clear pending decisions and per-path errors for groups after a successful ignore or restore.
- Leave review mode when that state is cleared.
- Removed the no-longer-needed topbar wrapper after the overlay removal.
- Added a regression test for the stale-decision flow.

## Impacted Files

- `src/features/conflict-report/ConflictModal.tsx` (modified)
- `src/features/conflict-report/ConflictModal.test.tsx` (modified)
- `src/widgets/launch-bar/LaunchBar.tsx` (modified)

## Goal

Ignoring a conflict group is a pure deferral action and cannot carry an old disable plan into a later review.

## Impact

No persisted ignore or mod activation behavior changed; only transient dialog state is reset after a completed action.

