# File-lock recovery validation

2026-10-10. This validates the approved file-lock feature; it is not a new claim of complete repository or Mods Manager coverage.

Follow-up audit found three bugs and diagnostic drift after these historical checks. Their scoped fixes and current validation limitations are recorded in [follow-up fixes](followup-fixes.md); the native session below predates those fixes.

## Final checks

- Full Rust library suite: **1,679 passed, zero failed, 22 ignored**, 87.10 seconds. Default ignored tests were not enabled.
- Additional final `file_lock` filter: **17 passed**; strict all-target Clippy, scoped Rust formatting and whitespace checks passed.
- Final frontend selections: **98 passing tests** and **72 passing tests**; these selections overlap and are not summed. Prior worker runs additionally cover workspace-switch hooks and object deletion. Full TypeScript check, scoped ESLint, frontend architecture checks, locale/document formatting and the production frontend build passed.
- New native E2E spec typechecks independently. The broader E2E TypeScript configuration encountered existing NodeNext/import errors in `onboarding-indexing-native.e2e.ts` and the bindings facade; those files were not rewritten for this feature.
- Independent review's transactional move, ancestor rollback and scope-replacement findings were corrected and re-reviewed; no additional concrete unsafe replay was reported in those boundaries.

## Native Windows application

`mods-manager-file-locks.e2e.ts`: **4/4 passed**, 29 seconds, fresh isolated Tauri/WebView2 build. Session `82f686e7527e70c8e6cb2a631bbdbe89`.

1. A real owned directory/file handle denies delete sharing. Initial rename and two separate manual retries fail; the same overlay remains visible and the source remains unchanged. Releasing the handles then Retry succeeds and closes it.
2. Cancel closes the overlay; releasing the lock afterwards does not execute the cancelled rename.
3. Restart Manager reports the owned Rust holder. Explicit force confirmation terminates only that holder; the overlay stays open and mutation requires a separate manual Retry.
4. Actual IPC rejects a forged recovery capability.

The holder executable was compiled into an owned temporary fixture outside Windows. No user's application was terminated. The harness confirmed shutdown before removing its one scheduled fixture. No installed application or production configuration was replaced. The shared build output can subsequently be overwritten by other tasks; the session's harness verified its private identifier and build identity at launch.

## Earlier failures retained

- One library build encountered `LNK1104` on an in-use test executable; a subsequent build/test run succeeded without broad deletion or terminating unrelated processes.
- One complete run reported **1,675 passes / one watcher failure / 22 ignored** in `watcher_fix_runtime_siblings_do_not_reach_authority_observer`. Its isolated rerun and the final complete suite passed. The shared checkout contained concurrent watcher work; this feature did not modify the watcher implementation or weaken that assertion.
- Initial frontend typecheck/format failures caused by command argument migration and formatting were corrected; final checks pass.

## Limits

Process attribution covers bounded regular-file resources, not every kernel or directory handle. A partial/unknown report never certifies unlock. System, services, protected/security/shell/self/runtime and unverified processes cannot be force-terminated through this feature. Permission denial remains distinct from known sharing/lock violations. Cross-volume publication, recycle-bin cleanup and committed synchronization are not blindly replayed.

Native UI evidence above concerns rename/recovery/process control. Other structural and transactional entry points are supported by Rust and frontend tests and focused code review; every row in the planning matrix was not separately exercised through the native UI.
