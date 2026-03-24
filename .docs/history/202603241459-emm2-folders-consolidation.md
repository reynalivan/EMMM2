# Consolidate EMM2 Folders

### Context

Dual artifacts generation folders (`EMM2_System` and `EMM2`) caused unnecessary filesystem clutter. Consolidation into a single implicit `.emmm_data` folder streamlines artifact generation and leverages existing scanner exclusion logic.

### Changes

- Replaced hardcoded `EMM2_System` and `EMM2` paths with `.emmm_data` across Rust backend configs.
- Updated KeyViewer generator relative pathing for INI resources.
- Updated default frontend hotkey settings configuration.

### Impacted Files

- `src-tauri/src/services/app/post_apply.rs` (modified)
- `src-tauri/src/services/hotkeys/manager.rs` (modified)
- `src-tauri/src/services/hotkeys/mod.rs` (modified)
- `src-tauri/src/services/keyviewer/tests/generator_tests.rs` (modified)
- `src-tauri/src/services/hotkeys/tests/hotkey_tests.rs` (modified)
- `src/features/settings/tabs/HotkeyTab.tsx` (modified)
- `.docs/requirements/req-42-ingame-hotkeys.md` (modified)
- `.docs/requirements/req-43-dynamic-keyviewer.md` (modified)

### Goal

A clean, single-point artifact write directory (`.emmm_data`) that naturally bypasses the ObjectList scanner due to dot-prefix rules without new exception logic.

### Impact

- Artifact writing now lands safely in `.emmm_data`.
- No ObjectList clutter from generated artifact files.
- No scanner regressions.

### Notes

- Leveraged `walker.rs` line 114 which implicitly ignores `.` folders.
