# Harden switch_mode corridor depth and membership

## Context

- Privacy corridor switch disable planning had two stability risks:
  - depth classification used raw path components and could misclassify object-root folders when DB stored absolute paths.
  - leaving-corridor enabled-mod lookup could include opposite-corridor mods due permissive corridor-source conditions.

## Changes

- Added root-aware depth classification helper `is_corridor_sub_mod_path` in privacy switch flow:
  - resolves folder paths against `mods_path`
  - strips normalized `mods_path` prefix when possible
  - only treats depth > 1 as sub-mod candidate
- Added strict corridor-enabled query in collection repo:
  - `get_enabled_mod_id_and_paths_for_strict_corridor`
  - filters by `status = 'ENABLED'` and `COALESCE(is_safe, 1) = ?`
- Updated `disable_enabled_mods_in_corridor` to use strict corridor query and root-aware depth filter.
- Added regression coverage for absolute-path depth handling:
  - `test_switch_mode_preserves_object_root_with_absolute_paths`

## Impacted Files

- src-tauri/src/services/privacy/mod.rs (modified)
- src-tauri/src/database/collection_repo.rs (modified)
- src-tauri/src/services/privacy/tests/privacy_service_tests.rs (modified)

## Goal

- Make safe/unsafe corridor switch disable phase deterministic and corridor-correct for both relative and absolute folder paths.

## Impact

- Prevents accidental disabling of object-root folders in absolute-path DB scenarios.
- Prevents cross-corridor disable drift during mode switch.
- Existing preserve and corridor-specific switch regressions are green.

## Notes

- Verified with targeted tests:
  - `cargo test test_switch_mode_only_disables_leaving_corridor -- --nocapture`
  - `cargo test test_switch_mode_preserves -- --nocapture`
