# Harden corridor preview stale fallback and target filtering

## Context

- Safe-mode switch preview could fail when corridor_state pointed to stale active/undo collection ids.
- Target preview semantics needed deterministic filtering:
  - respect disabled object-state members in target collection
  - hide disabled runtime-derived nested roots
  - still show explicit collection members even when currently disabled on disk (restore intent).
- Preview naming needed stable runtime-facing labels derived from folder names.

## Changes

- Added resilient target preview resolution fallback in corridor runtime:
  - active pointer preview load failure now warns and falls back to undo pointer
  - undo pointer preview load failure now warns and falls back to none-state
- Added target preview filtering layer:
  - excludes roots tied to disabled object states
  - applies disabled-path filtering only for runtime-derived roots (`runtime-root:*`)
- Normalized preview display names from folder path basename using existing display normalizer.
- Added stale-pointer regressions in privacy preview test suite:
  - `test_preview_corridor_switch_falls_back_to_undo_when_active_pointer_is_stale`
  - `test_preview_corridor_switch_returns_none_when_active_and_undo_pointers_are_stale`

## Impacted Files

- src-tauri/src/services/corridor_runtime.rs (modified)
- src-tauri/src/services/privacy/tests/privacy_service_tests.rs (modified)

## Goal

- Make safe-mode switch preview deterministic and resilient when remembered corridor pointers are stale.

## Impact

- Full preview corridor test set is green with fallback and filtering semantics preserved.
- Prior switch_mode hardening regressions remain green.

## Notes

- Verified with:
  - `cargo test --lib test_preview_corridor_switch -- --nocapture`
  - `cargo test --lib test_switch_mode_preserves -- --nocapture`
  - `cargo test --lib test_switch_mode_only_disables_leaving_corridor -- --nocapture`
