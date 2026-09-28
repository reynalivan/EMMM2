# First-game indexing and switch readiness

## Context

Onboarding could display background game indexing as blocking progress. After the first game indexed, activation and a redundant Mods-view scan could keep its switches disabled; rapid toggles could briefly mark the workspace unready while disk authority caught up.

## Changes

- Onboarding progress now tracks only the first game and waits for its applied activation revision before starting the remaining games and opening the dashboard. Snapshot workers for later games remain parked until that handoff, while an explicitly activated game can wake its own worker.
- Activation-listener setup must succeed before onboarding activates the first game; the optional runtime listener does not gate it. A concurrent activation claim no longer invalidates the background handoff, starts a duplicate background scan, or resurrects an already-ready game in the durable pending queue. Pending-queue removals now use compare-and-swap retries so concurrent completions cannot restore each other's IDs.
- The first-game full-scan path establishes watcher coverage before reconciliation and requires a trusted handoff before reporting an applied result.
- A just-activated game no longer receives an immediate duplicate Mods-view reconcile.
- Workspace readiness permits a completed core index to present as usable while watcher authority is repairable by mutation preflight; incomplete core indexing remains locked. A switch with no physical rename follows that same core-index gate, since it cannot change storage.
- Added regression tests for progress isolation, activation readiness, duplicate scanning, and handoff/readiness gates.

## Impacted Files

- `src/pages/onboarding/WelcomeScreen.tsx`, `src/pages/onboarding/WelcomeScreen.test.tsx`, `src/pages/onboarding/hooks/useOnboardingDiskProgress.ts` (modified)
- `src/app/entrypoint/App.tsx`, `src/app/entrypoint/App.test.tsx` (modified)
- `src/app/entrypoint/waitForGameActivationReady.ts`, `src/app/entrypoint/waitForGameActivationReady.test.ts` (added)
- `src/app/store/appStore/gameSlice.ts` (modified), `src/app/store/appStore/gameSlice.activationListener.test.ts`, `src/app/store/appStore/gameSlice.runtimeListener.test.ts` (added)
- `src/features/file-watcher/hooks/useFileWatcher.ts`, `src/features/file-watcher/hooks/useFileWatcher.test.ts` (modified)
- `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs`, `src-tauri/src/modules/reconciliation/application/disk_reconcile/onboarding_session.rs`, `src-tauri/src/modules/reconciliation/application/disk_reconcile/onboarding_recovery.rs`, `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs`, `src-tauri/AGENT.md` (modified)

## Goal

Open the dashboard after only the first game's trusted core index and activation, with its switches immediately available; continue remaining games in the background without duplicate scanning or transient toggle locks.

## Impact

The first game may remain on onboarding briefly while activation proves watcher continuity. Runtime/optional work remains asynchronous. No schema or API change.
