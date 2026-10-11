# Audit file watcher and suppression stability

## Context

User requested bug/race/stability audit and architecture planning. Read-only
production scope; preserve the dirty shared checkout and concurrent work.

## Changes

- Record four independently challenged Medium bugs: importer sibling noise
  invalidates authority; suppressed/expired evidence has no idle-loop wake;
  invalid inactive cache is adopted; backend repair cannot settle frontend
  pending after a failed retry IPC.
- Dismiss a successor-echo race blocked by the operation lease.
- Propose correctness patches first, then cohesive lifecycle extraction. Retain
  ownership/journal guards and existing queue/lock; no production changes.

## Impacted Files

Added: `docs/plans/file-watcher-stability/implementation_plan.md`, this history.
Added local audit artifacts in `.bug-hunter/watcher-audit/`: triage/hash/coverage,
Hunter/Skeptic/Referee/report and plan-only strategy; isolated Rust/native and
frontend reproduction fixtures. No application source or configuration edited.

## Validation

Existing watcher79 and frontend55 passed. Three isolated Windows notify/resource
probes reproduce the native boundary failures; one actual-hook contract test is
intentionally RED for pending-state recovery. Canonical schemas validate.
No full application native E2E or complete repository audit claimed. Ten core
production files plus focused caller slices inspected; partial/pending coverage
and five concurrent out-of-focus source changes are preserved in the ledger.
Final frontend typecheck passed; full build/native UI E2E was not run.

## Goal and Impact

Evidence-backed implementation plan awaiting approval. No mod/game mutation,
dependency/schema/IPC change, new worktree, commit, service or installed binary.
