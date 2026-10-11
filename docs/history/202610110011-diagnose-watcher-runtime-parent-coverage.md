# Diagnose and fix watcher runtime-parent coverage

## Context

Native hotkey failures retained only PermissionDenied, obscuring the failing path.

## Changes

- Log the first cache failure's stage, path and raw OS code; keep guards unchanged.
- Retry EBUSY receiver reads twice at 50 ms; preserve parsing and ownership errors.
- Validate the latest overlay corrections with fresh isolated native run L.
- Bound cache maintenance to actual watch modes; parent refresh preserves nested
  Mods identities without enumerating unrelated siblings.

## Impacted Files

- `src-tauri/src/modules/workspace/adapters/notify/identity_cache.rs` (modified).
- `src-tauri/src/modules/workspace/application/scanner/watcher/mod.rs` (modified).
- `src-tauri/src/modules/workspace/application/scanner/tests/watcher_tests.rs` (modified).
- `src-tauri/AGENT.md` (updated).
- `tests/e2e/specs/global-hotkeys-native.e2e.ts` (modified).
- `docs/plans/mods-manager-hardening/{hotkey-replay-remediation-plan.md,global-hotkey-native-validation.md}` (updated).
- This history (added); ignored run L evidence retained in `.audit-output/`.

## Goal and Impact

L establishes metadata Windows error 5 on an unrelated runtime-parent replacement
temporary file as the cause of coverage loss. The scoped fix avoids that query
while retaining recursive/root failure guards and runtime INI event delivery.
No real-library access, guard relaxation, dependency or schema change.

## Verification

Two new regressions RED→GREEN; 284 workspace tests pass, 4 existing benchmark/
profiler cases ignored. Strict all-target Clippy and independent review pass.
L before the scope fix: 3 cases pass / 3 fail; latest overlay 5/5, background
refusal, global Safe Mode and disabled behavior pass. Owned cleanup verified.
Native revalidation of the watcher fix, physical held modifiers, focus loss and
Genshin remain pending. E2E types/lint/format and scoped Rust formatting pass.
