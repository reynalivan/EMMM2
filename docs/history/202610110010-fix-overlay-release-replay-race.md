# Fix overlay release/replay race

## Context

Native run J proved original Ctrl-up interleaved separate replay events.

## Changes

- Wait for the complete chord to release; revalidate settings and foreground.
- Send one balanced Windows input batch and always restore registration.
- Correct partial cleanup ownership and layout-sensitive ASCII virtual keys.
- Deduplicate semantic modifier aliases before generating balanced input events.
- Add regression tests and retain full setup bulk outcomes; request five replays
  and eight snapshot/toggle cycles in the native fixture.

## Impacted Files

- `src-tauri/Cargo.toml`, `src-tauri/AGENT.md` (modified).
- `src-tauri/src/modules/automation/application/hotkeys/{reload.rs,manager.rs,tests/manager_tests.rs}` (modified).
- `src-tauri/src/modules/workspace/adapters/notify/identity_cache.rs` (test only).
- `tests/e2e/specs/global-hotkeys-native.e2e.ts` (modified).
- `docs/plans/mods-manager-hardening/{hotkey-replay-remediation-plan.md,global-hotkey-native-validation.md}` (modified).
- This history file (added).

## Goal

Prevent modifier release interference while preserving global Safe Mode/presets
and game-focused-only overlay replay. No focus transfer or real-library mutation.

## Impact

49 hotkey tests and strict all-target Clippy pass. Native K: five full-modifier
overlay replays pass, but overall 1 case passes / 5 fail. Watcher authority loss,
status-read EBUSY, physical keyboard and Genshin coverage remain open. Historical
bulk RolledBack was an untouched-plan abort after watcher proof failed, not a
reversed rename; the originating OS error is unknown. The modifier-alias regression
failed before its fix and now passes; independent replay review has no remaining
finding. Review corrections still need a fresh native run. Full native GREEN is
not claimed.
