# Prepare native global hotkey verification

## Context

User requested real OS hotkey verification and clarified that the game should
remain foreground. Test data stays synthetic; actual game is input source only.

## Changes

- Add native operator-driven spec and owned passive receiver, unique test chords,
  three distinct presets, exact chord receipts, foreground and cleanup evidence.
- Correct receiver GUI launch and reproduce/fix its status-file replacement race
  with bounded coalesced publication, without changing app filesystem behavior.
- Add explicit foreground-game source mode, keeping the selected fake executable
  so automatic reload cannot target the actual game. Document skipped cases.
- Correct stale hotkey command comments about focus requirements.

## Impacted Files

- `tests/e2e/specs/global-hotkeys-native.e2e.ts` (added).
- `tests/e2e/support/hotkey-input-window.cs` (added).
- `src-tauri/src/modules/automation/adapters/tauri/hotkey_cmds.rs` (comments only).
- `docs/plans/mods-manager-hardening/global-hotkey-native-validation.md` (added).
- This history entry (added).
- Generated evidence: `.audit-output/global-hotkeys-native-20261010-{a,b,c,d}/`.

## Goal and Impact

Verify native shortcut dispatch without touching the real mod library or
pretending IPC/DOM keys are global input. No dependency, public API, schema,
production behavior, installation, worktree or commit changes.

## Validation

Typecheck, scoped lint/format and source review pass. Receiver lifetime and
held-read RED→GREEN probes pass after the transport fix. Attempts A–D remain
failed/aborted with no OS key sent; tool activation failed while Genshin was
foreground, not a proven listener defect. Game-source attempt E also sends no key
because its Genshin process/window disappears before input: 0 passed/3 failed/2
skipped. Both observers exit 0, fixture removed after native shutdown. No global
hotkey or 3DMigoto success is claimed; user action must restore foreground game.

[Evidence and remaining coverage](../plans/mods-manager-hardening/global-hotkey-native-validation.md).
