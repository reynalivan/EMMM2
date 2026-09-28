# Single-mod disk success toast

## Context

Single mod-folder switches had no success toast, while object and bulk switches did. The optimistic switch position alone was not proof of a completed disk rename.

## Changes

- Show the existing localized mod-action success toast only when a folder switch returns `applied` with a disk revision receipt.
- Cover node, direct folder-path, parent-confirmation, and duplicate-resolution switch paths.
- Suppress success for no-op, missing receipt, superseded intent, and stale game results.
- Add regression tests for disk receipt timing, rapid last-wins clicks, no-op, and confirmation paths.

## Impacted Files

- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts` (modified)
- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.test.tsx` (modified)
- `docs/history/202609280009-single-mod-disk-success-toast.md` (added)

## Goal

Confirm a single mod enable or disable after the physical folder change is verified and committed, without waiting for runtime projection or cache refresh.

## Impact

No filesystem, database, IPC, or translation contract changes. The toast reuses existing localized bulk message formatting and strips the `DISABLED` prefix from its displayed folder name.
