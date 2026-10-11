# Validate native global hotkey input

## Context

The user requested validation after global workspace hotkeys were implemented.
Coordinate the shared compiler/native slot and use synthetic Mods only.

## Changes

Run the existing native spec with actual Computer Use OS keyboard input. Retain
receipts, disk/runtime state, replay logs, source hashes and cleanup evidence.
Update the validation report; do not modify production behavior or weaken tests.

## Impacted Files

- `docs/plans/mods-manager-hardening/global-hotkey-native-validation.md` (updated).
- This history entry (added).
- Generated `.audit-output/global-hotkeys-native-20261011-f.log` and sibling
  evidence directory (ignored artifacts).

## Goal and Impact

Establish actual global listener behavior without using IPC/DOM action dispatch
as keyboard evidence. No real library mutations, installation, dependency,
schema or application-source changes. Owned fixture removed after verified
native shutdown; compiler/driver/GUI slot released.

## Verification

Fresh private debug build and frontend typecheck passed. Native result: 4 passed,
1 failed, 0 skipped; 12 OS chords sent. Safe Mode minimized/background,
next/previous/both wraps, configured-game-stopped and disabled input passed.
Background overlay refusal passed, but focused modifier overlay replay recorded
Ctrl absent; second replay was not reached. Root cause remains unproven. Actual
Genshin foreground and 3DMigoto rendering were not tested because game was absent.
