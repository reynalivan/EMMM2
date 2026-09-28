# Indexing watcher takeover and clean refresh

## Context

First-game onboarding could appear stuck at finishing, while post-index focus refreshes and activation handoff could re-scan or briefly block the switcher. Replacing an inactive watcher could also discard buffered debounce events.

## Changes

- Onboarding now reports first-game activation readiness instead of leaving the old finishing-stage label visible.
- Clean focus/Mods-view checks reuse an applied disk revision only when the operation lock, journal, watcher repair, and authority proof agree; dirty/ambiguous cases retain reconciliation.
- Inactive watcher takeover moves the same watcher, session, and bounded event receiver to active use. Changed root identity rejects takeover and triggers a new watcher/full catch-up.
- Startup may establish watcher coverage for its saved selected game before activation; background prewarm remains restricted to other games.
- Prewarm and activation ownership are serialized; an older result cannot replace the latest authority revision. Pending onboarding marker cleanup follows readiness asynchronously.
- Frontend refresh merges queued work and rejects stale revisions; the grid does not relock on an older workspace-sync state after activation is Ready.

## Impacted Files

- Backend authority and tests: `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/state.rs`, `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/tests.rs`, `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs` (modified).
- Backend watcher and tests: `src-tauri/src/modules/workspace/application/scanner/watcher/lifecycle.rs`, `src-tauri/src/modules/workspace/application/scanner/watcher/mod.rs`, `src-tauri/src/modules/workspace/application/scanner/tests/watcher_tests.rs` (modified).
- Backend journal, settings, startup: `src-tauri/src/modules/mutation/coordinator.rs`, `src-tauri/src/modules/mutation/journal.rs`, `src-tauri/src/modules/mutation/tests.rs`, `src-tauri/src/modules/settings/adapters/tauri/settings_cmds.rs`, `src-tauri/src/modules/system/application/app/bootstrap.rs` (modified).
- Frontend and tests: `src/features/file-watcher/hooks/useFileWatcher.ts`, `src/features/file-watcher/hooks/useFileWatcher.test.ts`, `src/pages/onboarding/WelcomeScreen.tsx`, `src/pages/onboarding/WelcomeScreen.test.tsx`, `src/widgets/mod-explorer/hooks/useFolderGridViewModel.ts` (modified); `src/widgets/mod-explorer/hooks/useFolderGridViewModel.test.ts` (added).
- Copy and architecture: `src/shared/i18n/locales/en/onboarding.json`, `src/shared/i18n/locales/id/onboarding.json`, `src/shared/i18n/locales/zh/onboarding.json`, `src-tauri/AGENT.md`, `docs/plans/priority-game-indexing/implementation_plan.md` (modified).

## Goal

First-game readiness can open the dashboard without a redundant clean scan; ordinary clean refresh does not lock the switcher; watcher handoff retains pending filesystem evidence.

## Impact

Untrusted continuity still requires a full scan. A prewarm operation may hold activation for at most its 250 ms budget. No schema, dependency, or release artifact change.
