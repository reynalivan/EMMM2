# Revise indexing and readiness implementation plan

## Context

The latest audit found overlapping readiness gates and refresh request paths. The user requested a simpler implementation plan before changing application behavior.

## Changes

- Replace the prior plan with the current audit baseline, six ordered work packages, and explicit acceptance gates.
- Specify one backend readiness owner, reusable onboarding handoff, disk-first last-wins settlement, scoped conflict validation, and revision-aware refresh.
- Require native race tests and measured baselines; preserve crash recovery and distinguish optional runtime work from mutation safety.

## Impacted Files

- `docs/plans/priority-game-indexing/implementation_plan.md` (modified)
- `docs/history/202609280011-indexing-readiness-refactor-plan.md` (added)

## Goal

Provide an actionable plan that reduces competing state owners without weakening filesystem safety.

## Impact

Documentation only. No application behavior, dependencies, database schema, release, or installer changes. Implementation awaits approval.
