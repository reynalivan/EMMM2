# Fix watcher convergence and plan architecture cleanup

## Context

User approved the four audited watcher/suppressor bugs first, then requested a
separate architecture plan. Preserve existing dirty/shared workspace changes.

## Changes

- Gate coverage relevance before authority observation; runtime sibling noise
  no longer requests Mods repair. Preserve runtime INI/root-loss/rescan evidence.
- Session-owned coalesced wake covers suppressed observations, guard release
  and failed/expired echo proof; the receiver rereads existing dirty generations.
  Clean wakes retain activation retry deadlines; stale sessions stay isolated.
- Reject invalid identity caches in all three inactive handoff eligibility paths.
- Follow accepted background recovery with a current-epoch frontend check after
  failed retry; retain failure-epoch, context and duplicate-revision guards.
- Separate the four-point refactor plan; no Tauri orchestration extraction yet.

## Impacted Files

Modified:

- `src-tauri/src/modules/workspace/application/scanner/watcher/{mod,suppressor,lifecycle}.rs`.
- `src-tauri/src/modules/workspace/application/scanner/tests/watcher_tests.rs`.
- `src/features/file-watcher/hooks/useFileWatcher{,.test}.ts`.
- `src-tauri/AGENT.md`.
- `docs/plans/file-watcher-stability/implementation_plan.md`.

Added: `docs/plans/file-watcher-stability/refactor_plan.md`, this history and
`.bug-hunter/watcher-fix/` scope/baseline/plan/report/validation artifacts.

## Validation

180 scoped Rust tests,61 frontend tests and1 fresh-build native Windows case
passed;1 manual benchmark ignored. Native case covers three301-mod process
restarts with one watcher/baseline each; owned shutdown and fixture cleanup
verified. Strict Clippy, scoped lint/format/whitespace, typecheck and frontend/
native builds passed. Independent review found no scoped Medium+ blocker.

Native idle-suppression UI/DB coverage remains narrower than the native signal
tests; no complete repository audit, Linux or real game integration claim.

## Goal and Impact

Four verified convergence/availability defects fixed; broader four-stage plan
awaits approval. No dependency, IPC/schema/journal change, new queue/daemon,
production reset, installed binary replacement, worktree or commit.
