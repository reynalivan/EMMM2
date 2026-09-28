# Preset status overlay toggle

## Context

The preset status overlay was forcibly hidden even though KeyViewer keybind panels remained available. The two features needed independent controls.

## Changes

- Added a default-off preset status overlay switch under Global hotkeys, separate from the KeyViewer switch.
- Persisted the setting with a backward-compatible false default for older configuration files.
- Included the setting in generic settings-save runtime-change detection so that path also republishes the overlay.
- Generated and published the status text only when enabled; KeyViewer panels and their F7 gate remain independent.
- Added UI, configuration, and generator regression coverage for the separate states.

## Impacted Files

- `src/pages/settings/components/tabs/HotkeyTab.tsx` (modified)
- `src/pages/settings/components/tabs/HotkeyTab.test.ts` (modified)
- `src/shared/i18n/locales/en/settings.json` (modified)
- `src/shared/i18n/locales/id/settings.json` (modified)
- `src/shared/i18n/locales/zh/settings.json` (modified)
- `src/shared/api/tauri/bindings.gen.ts` (modified)
- `src-tauri/src/modules/automation/application/hotkeys/mod.rs` (modified)
- `src-tauri/src/modules/automation/application/hotkeys/tests/hotkey_tests.rs` (modified)
- `src-tauri/src/modules/automation/application/keyviewer/generator/ini.rs` (modified)
- `src-tauri/src/modules/automation/application/keyviewer/tests/generator/ini_tests.rs` (modified)
- `src-tauri/src/modules/system/application/app/post_apply.rs` (modified)
- `src-tauri/src/modules/settings/adapters/tauri/settings_cmds.rs` (modified)
- `docs/history/202609280007-preset-status-overlay-toggle.md` (added)

## Goal

Users can opt into the in-game preset status display without altering KeyViewer availability or requiring its F7 toggle.

## Impact

Existing installations keep the preset status display off until explicitly enabled. Changing either option republishes the generated runtime overlay; no database migration is needed.

## Notes

Rust unit tests pass. The separate DAL architecture audit still reports pre-existing cross-module adapter imports and SQL in inbound adapters outside this change.
