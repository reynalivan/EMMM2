# Fix activation acceptance and bounded switch projection wait

## Context

After onboarding, a clean watcher result could still be blocked by an unfinished disk commit or suppressor repair. Initial activation and recovery used different readiness checks, so the gate could remain stuck or become Ready too early. A switch that had already renamed its folder could also retain its optimistic UI state indefinitely if the database projection or a snapshot request stalled.

## Changes

- Use the same disk-authority, pending-commit, and watcher-repair acceptance check for initial activation and recovery.
- Keep a clean activation pending while a commit settles, then recheck without another disk scan; repair dropped watcher events with a full scan.
- Preserve diagnostic results for unavailable sources and revalidate recovered readiness against stale activation generations or newly dirty watcher authority before completing onboarding and publishing Ready.
- Bound each frontend projection snapshot call to two seconds and retry with backoff, while preserving the disk-receipt switch state until the database projection is actually verified.
- Add regression tests for activation barriers, transient projection errors, and delayed projection or stalled native snapshot calls.

## Impacted Files

- `src-tauri/src/modules/workspace/application/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/AGENT.md` (modified)
- `src/features/workspace-runtime/actions/workspaceSwitchOps.ts` (modified)
- `src/features/workspace-runtime/actions/workspaceSwitchOps.test.ts` (modified)
- `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.test.tsx` (modified)
- `docs/history/202609290004-fix-activation-acceptance-and-switch-wait.md` (added)

## Goal

Activation and the first switch after onboarding should use one consistent disk-backed readiness rule. A committed folder rename remains visible and interactive while projection retries in the background.

## Impact

Disk commit polling yields to foreground operations. Snapshot retries do not hold the switch interaction; the disk receipt is retained until projection is verified. No schema or dependency change.
