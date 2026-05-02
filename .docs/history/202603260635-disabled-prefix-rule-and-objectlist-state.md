# Strict disabled prefix rule and object list state

## Context

Disabled detection was too permissive and ObjectList also treated objects as disabled when they had zero enabled child mods. That caused false-disabled states and made the UI disagree with the intended disk rule.

## Changes

- Tightened disabled detection to the canonical `DISABLED ` prefix only in frontend and backend helpers.
- Removed the old fuzzy `dis|disable|disabled` prefix matching that could flag names like `distance_mod`.
- ObjectList rows now treat an object as disabled only when the physical object folder itself has the disabled prefix.
- Context-menu object enabled state in ObjectList now follows `is_object_disabled` directly instead of recomputing from the folder name.
- Updated disabled-prefix unit tests to match the canonical rule.

## Impacted Files

- `src-tauri/src/services/scanner/core/normalizer.rs` (modified)
- `src-tauri/src/services/scanner/core/tests/normalizer_tests.rs` (modified)
- `src-tauri/src/services/mods/core_ops.rs` (modified)
- `src/lib/disabledPrefix.ts` (modified)
- `src/lib/disabledPrefix.test.ts` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/object-list/ObjectListContent.tsx` (modified)

## Goal

Disabled state now follows one rule only: folders with the canonical `DISABLED ` prefix are disabled, and object rows stay enabled unless the object folder itself is disabled.

## Impact

- Prevents false positives from names starting with `dis...`.
- Stops ObjectList from striking through objects just because child enabled count is zero.
- Breaking change: old non-canonical disabled naming variants are no longer auto-detected as disabled.
