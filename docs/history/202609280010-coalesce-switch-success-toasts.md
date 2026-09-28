# Coalesce switch success toasts

## Context

Disk-verified single-mod switches emitted one success toast per commit. Rapid clicks and switches across several mods could fill the toast stack despite the switcher's last-wins intent handling.

## Changes

- Collect verified folder-switch commits for a short 500 ms settle window, then emit one localized success summary.
- Keep only the latest committed state for each physical folder identity, including enabled and `DISABLED` path spellings.
- Summarize mixed enable/disable bursts as updated mods, while single-direction bursts retain enabled or disabled wording.
- Discard pending success feedback after switching games.
- Add regression tests for rapid multi-mod switching and stale-game feedback.

## Impacted Files

- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts` (modified)
- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.test.tsx` (modified)
- `docs/history/202609280010-coalesce-switch-success-toasts.md` (added)

## Goal

Show one truthful disk-verified success notification per short switch burst without delaying disk mutation, optimistic switch state, or runtime refresh.

## Impact

No disk, database, IPC, or translation contract changes. Only success-toast presentation is delayed by the settle window; switch execution is unchanged.
