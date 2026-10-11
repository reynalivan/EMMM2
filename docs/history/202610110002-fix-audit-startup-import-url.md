# Fix startup recovery, import ownership and browser URLs

## Context

The approved audit found a retained startup splash, a source-replacement race
during import, and case-sensitive URL normalization that changed the host.

## Changes

- Dismiss the splash when the real recovery screen becomes ready.
- Bind import publication to the planned source identity and the same native
  source handle. Retain source/parent pins through copied publication and retire
  the original source through that handle. Preserve target proof and rollback.
- Recognize HTTP(S) schemes case-insensitively without changing path spelling.
- Add regression tests and private native recovery/URL scenarios. Keep the
  generic fallback wrapper available only to its remaining tests.

## Impacted Files

- `src/app/entrypoint/App.tsx`, `App.test.tsx` (modified)
- `src/pages/browser/utils/browserUrl.ts`, `browserUrl.test.ts` (modified)
- `src-tauri/src/modules/browser/application/browser/browser_service/settings.rs` (modified)
- `src-tauri/src/modules/mutation/application/workspace_mutation/import_commit.rs`, `import_commit_tests.rs` (modified)
- `src-tauri/src/platform/fs/file_utils.rs` (modified)
- `src-tauri/AGENT.md` (modified)
- `tests/e2e/specs/audit-fixes.e2e.ts` (added)
- `docs/plans/audit-fixes-20261010/implementation_plan.md` (added)
- This history file (added)

## Goal and Impact

Recovery remains actionable, imports reject replacement folders, and valid URLs
keep their intended host. The source-handle guarantee is verified on Windows;
Linux retains the existing path-based rename primitive. No IPC, migration or
dependency change.

## Verification

30 frontend tests and 32 targeted Rust tests passed. App/E2E typechecks, scoped
lint/architecture/format, strict all-target Clippy, and production/private native
builds passed. All six native cases passed across the focused import and audit
specs, including recovery and address-field/direct-native URL navigation. Owned
fixtures were cleaned after native process shutdown. Actual movement between two
physical volumes was not exercised; the Windows unit test forces the cross-device
path.

Initial native attempts exposed an incorrect Ignore button locator and the
expected abnormal-exit notification after a forced driver restart. Both test
flows were corrected using actual UI actions. A retry stopped before tests
because another native session held `emmm-e2e.exe` (`EBUSY`). After that slot was
released, the fresh private build passed all three audit cases. No browser
substitute was used.
