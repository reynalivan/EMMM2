# Fix file-lock follow-up regressions

## Context and changes

User approved fixing the follow-up audit's three bugs and diagnostic drift.
Metadata-only Windows pins now request data/list access to prevent namespace
rename. Dialog and nested bulk diagnostic I/O failures can recover without
unblocking stale/rollback/mutation failures. Bulk chunks share one operation
retry budget. Safe INI reads/restored publication retain structured lock errors;
failed restore still requires recovery rather than generic replay.

## Impacted files

- Backend: `src-tauri/src/platform/fs/{locking,atomic_file}.rs`;
  `modules/library/application/ini/{write.rs,tests/write_tests.rs}`;
  `modules/library/application/mods/bulk/toggle.rs`;
  `modules/library/adapters/tauri/mod_bulk_cmds.rs`.
- Frontend: `src/features/file-watcher/dialogs/FileInUseDialog{.tsx,.test.tsx}`;
  `src/features/workspace-runtime/actions/fileInUseRetry.ts`;
  `src/shared/lib/appError{.ts,.test.ts}`.
- Evidence: `.bug-hunter/file-lock-fixes/` scope, native probes, source snapshots
  and canonical remediation report.
- Docs: `docs/plans/mods-manager-file-locks/{validation,followup-fixes}.md` and
  this history entry.

## Goal, impact and validation

Safe continued manual recovery and bounded bulk waiting. No dependency/schema/
IPC payload changes, elevation, commit or unrelated watcher edits.
26 frontend tests and seven current locking-module Windows tests pass;
the native regression probe demonstrated RED then GREEN. Typecheck/frontend
build/scoped lint/architecture/format checks pass. Full Rust: 1,710 passed,
two failed, 26 ignored; all eight added Rust regressions passed. The collection
failure passed alone; watcher recheck/strict Clippy remain affected by concurrent
watcher changes. Fresh isolated native build/E2E passes 4/4 (session
`2000c0468382b59029f9a9f3b47243e0`); the owned fixture was cleaned after shutdown.

[Detailed verification and limits](../plans/mods-manager-file-locks/followup-fixes.md).
