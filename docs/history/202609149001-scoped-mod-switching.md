# Scoped mod switching and journal safety

## Context

Folder conflicts must not interrupt unrelated toggles. Disk remains authoritative; projection and UI must report actual results.

## Changes

- Passive conflict reports keep banners and per-folder protection without opening a modal. Explicit Resolve and rename-confirmation flows remain.
- Bulk no-ops preserve an empty backend rewrite list instead of inventing path changes.
- Force Enable leaves siblings alone. Enable Only This disables effective siblings even when their duplicate warning was ignored; canonical identity excludes the target itself.
- Prepared target paths match canonical rename paths on Windows.
- Single/bulk toggle journals require an applied projection, including AppliedWithFolderConflicts. Blocked projection triggers rollback; failed rollback/compensation retains a repair record.
- Pending runtime-effect warnings survive successful command settlement.

## Impacted Files

- Frontend: `src/features/file-watcher/hooks/useFileWatcher.ts` and `.test.ts`; `src/features/mod-runtime/utils/folderMutationPayloads.ts` and new `.test.ts`.
- Backend: `src-tauri/src/modules/workspace/application/workspace/switch.rs`, new `tests/prepared_switch_tests.rs`; `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs`; `src-tauri/src/modules/library/adapters/tauri/mod_bulk_cmds.rs`; `src-tauri/src/modules/reconciliation/application/disk_reconcile/emit.rs`.
- Tracking: `docs/plans/scoped-mod-switching/implementation_plan.md`; this history entry.

## Goal and Impact

Conflict scope stays local. Filesystem prefixes determine own enabled state; disabled ancestors still affect effective state. SQLite stays a disk projection. Metadata, mod classification, and collection snapshots are not rewritten by this change. Existing refresh descriptors still update preview, collections/runtime counts, dashboard, conflicts, and keybindings.

No dependency, schema, IPC shape, visual redesign, or user mod-storage changes. Warning propagation also benefits existing callers of the shared settlement helper.

## Validation

- Regression tests reproduced automatic modal opening, invented no-op rewrites, Force Enable disabling siblings, canonical-path mismatch, dropped runtime warning, and ignored-warning interference with Enable Only This.
- Frontend: 218 tests passed; typecheck, focused ESLint, and Vite production build passed.
- Rust: 1,073 tests passed, 7 ignored; binding-export test excluded from the final run (passed earlier). Clippy completed with existing warnings outside these changes; targeted rustfmt and `git diff --check` passed.
- Read-only review found an additional absolute/relative Windows self-exclusion gap; regression reproduced it and passed after root-aware normalization.

## Remaining Gaps

Native Tauri end-to-end failure injection was not run. Command-level rollback/compensation tests would strengthen the existing preparation, projection, and journal tests. FailedNeedsRepair is retained for repair, not automatically retried by startup recovery. Legacy switch-path consolidation and shared path-normalization cleanup are deferred to a separate scoped change.
