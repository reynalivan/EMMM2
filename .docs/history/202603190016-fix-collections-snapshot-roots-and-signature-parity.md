# Fix collections snapshot roots and signature parity

## Context

- Collections apply/undo regression failed because unsaved snapshot targets were materialized with empty runtime roots.
- Runtime preview regression included disabled unicode nested paths and produced incorrect root counts.
- Root cause was strict runtime root derivation relying on classifier-only detection for empty test folders, plus unstable root signature parity across DISABLED prefix transitions.

## Changes

- Added runtime root fallback in collections snapshot derivation: when classifier cannot identify a root, use the resolved mod folder path itself.
- Normalized runtime root resolution guardrails:
  - require resolved path under mods root
  - skip effectively disabled paths during runtime root materialization
- Normalized signature root key generation to strip DISABLED prefix semantics per path segment so named/runtime parity stays stable across enable/disable renames.
- Removed temporary diagnostics from apply pipeline after verification.

## Impacted Files

- src-tauri/src/services/collections/runtime_snapshot.rs (modified)
- src-tauri/src/services/collections/apply.rs (modified)

## Goal

- Ensure apply/undo snapshots always carry resolvable runtime roots and preserve strict active-state signature parity.

## Impact

- Fixed failing regressions:
  - collections_apply_then_undo_restores_state
  - collections_preview_filters_disabled_unicode_nested_path
  - collections_apply_keeps_named_active_state_on_follow_up_reads
- Restored green status for full collections integration target without changing command/schema contracts.

## Notes

- Verification included full collections_service integration tests and targeted privacy/watcher regression checks.
