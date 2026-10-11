# Complete Mods preview and folder recovery

## Context

Completes MM-5/MM-6 and the added reload-notice fix, superseding their partial
status in `202610100007-fix-mods-manager-hardening.md`. Uncommitted on `main`;
concurrent edits preserved. Not a new repository-wide audit.

## Changes

- Hidden runtime staging no longer taints watcher coverage; stale moved-path
  events preserve historical IDs. Real coverage failures and guards are unchanged.
- Windows creation/recovery pins ownership, promotes without overwrite, removes
  only matching empty directories, and retains added/replacement assets. Original
  root/UUID stage checks and idempotent cleanup cover interrupted creation.
- Stopped games publish without a reload binding/false notice. Running/unknown
  state retains manual reload; automatic input rechecks focus.

## Validation

- Five targeted RED regressions became GREEN. Final Rust: library 271, mutation
  90, post-apply 51, cache 12, directory races 2, reload 2, permissions 1, path
  keys 9, images 10, watcher 79: **527 passed**. Default ignores are crash workers
  and benchmarks; creation's worker runs through parent tests at six boundaries.
- **366 frontend tests; 7/7 native Windows WDIO cases passed**, session
  `ef63d77794cfae58183cdd1c502e339d`, 2026-10-10 14:33 WIB. Covers Favorite/Pin,
  junction creation, fullscreen, hide/restore, clear-gallery/restart/right-click,
  texture toggles and stopped-game notice; image bytes unchanged. Owned driver
  stopped before two fixture removals.
- Builds, E2E typecheck, scoped lint/format, strict Clippy and whitespace passed.
  Earlier failures remain recorded: stale-revision test teardown was corrected;
  an unrelated trash canary passed rerun/final serial library suite. None excluded.

Schema-validated `docs/plans/mods-manager-hardening/completion-report.json` records
900 distinct selected passes, not full-repository coverage. Its review count
concerns the three fixes, not remaining integration/release checks below.

## Manual review / remaining boundaries

1. Explorer/INI editor, external Mod Viewer, clipboard import, native pickers
   and positive recycle-bin deletion were not exercised in this continuation.
2. Real game/loader launch, global hotkeys and focused-game reload were not
   exercised. Missing executable means unknown and may legitimately retain notice.
3. Malformed/foreign-root/nonempty journals require manual recovery review.
   Non-Windows ownership-bound recovery removal remains unsupported/fail-closed.
4. Independent final re-review hit its usage limit. Its ownership race was fixed
   and tested; author review completed, but independent sign-off remains pending.

## Impacted Files

Modified (including files already untracked before this continuation):

```text
src-tauri/AGENT.md
src-tauri/src/shared/path_key.rs
src-tauri/src/modules/workspace/application/scanner/watcher/event_filter.rs
src-tauri/src/modules/workspace/adapters/notify/identity_cache.rs
src-tauri/src/modules/mutation/api.rs
src-tauri/src/modules/mutation/journal.rs
src-tauri/src/modules/mutation/recovery.rs
src-tauri/src/platform/fs/rename.rs
src-tauri/src/modules/library/adapters/tauri/mod_core_cmds.rs
src-tauri/src/modules/system/application/app/post_apply.rs
src-tauri/src/modules/automation/application/hotkeys/reload.rs
tests/e2e/specs/client-workspace-smoke.e2e.ts
docs/plans/mods-manager-hardening/implementation_plan.md
docs/history/202610100007-fix-mods-manager-hardening.md
```

Added:

```text
src-tauri/src/modules/library/adapters/tauri/tests/mod_folder_create_recovery_tests.rs
docs/plans/mods-manager-hardening/completion-report.json
docs/history/202610100011-complete-mods-preview-and-folder-recovery.md
```

## Goal and impact

MM-5/MM-6 pass the approved Windows boundary while preserving assets. No new
dependency, IPC/schema migration, journal-version change, production reset or commit.
