# Fix automation, games and library safety

## Context

Fix all ten approved AGL audit findings while preserving parallel workspace edits.

## Changes

- INI save protects/revalidates source bytes, retains the original as backup,
  publishes without overwrite and restores after validation/commit failures.
- Startup trash retry retains journal root/target identities, rejects links and
  stages the owned payload under a fresh name; uncertain cleanup warns and retains data.
- Hotkeys use OS shortcut identities, serialize settings/registration/overlay
  lifecycle and revalidate press-time game/root/focus. Failed restoration reaches UI.
- Windows launch preserves quoted arguments; loader detection uses executable
  identity; game instance validation rejects regular files.
- Mod Health resolves filenames from the declaring INI and tracks resource
  namespace plus active/inactive provenance.

## Impacted Files

Paths under `src-tauri/src/` (modified):

- `platform/fs/rename.rs`
- `modules/library/application/ini/{write.rs,tests/write_tests.rs}`
- `modules/library/application/mods/{trash/service.rs,tests/trash_tests.rs}`
- `modules/library/application/mod_health/{service.rs,tests.rs}`
- `modules/system/application/app/bootstrap.rs`
- `modules/automation/application/hotkeys/{manager.rs,cycle_preset.rs,safe_mode.rs,tests/manager_tests.rs}`
- `modules/automation/adapters/tauri/hotkey_cmds.rs`
- `modules/games/adapters/tauri/{game_cmds.rs,tests/game_cmds_tests.rs}`
- `modules/games/application/game/{validator.rs,tests/validator_tests.rs}`

Other files:

- `src-tauri/AGENT.md` (modified)
- `tests/e2e/specs/client-workspace-smoke.e2e.ts` (modified; exact backup assertion)
- `.bug-hunter/agl-fix/` (added: baseline/hash copies, strategy, plan, immutable
  scope, implementation plan, final report and validation evidence)
- This history (added)

## Goal

Prevent audited wrong-target/data-loss paths and keep failures recoverable and visible.

## Impact

No IPC, dependency, migration or fingerprint changes. Serialized hotkey updates
and ownership checks are deliberate safety costs; no speedup claim.
Legacy trash records without proof remain for manual recovery.

## Validation and limits

112 Rust tests, 44 frontend tests and one fresh-build native Windows INI E2E
passed. Strict test-inclusive Clippy, scoped lint/format/diff checks and production
TypeScript/Vite plus native debug build passed. Independent review found no
remaining actionable scoped finding after corrections.

No real game/loader launch, global shortcut registration or positive OS recycling
was tested. Linux runtime is unverified. The path-only recycle API still leaves
a narrow UUID-path swap window after handle release. No full-audit or power-loss
guarantee. Details: `.bug-hunter/agl-fix/validation.md`.
