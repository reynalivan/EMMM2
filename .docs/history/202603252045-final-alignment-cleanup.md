# 2026-03-25 20:45 - Final alignment cleanup

## Summary

- Removed implicit onboarding Deep Match Scanner execution. Onboarding now saves games and runs Disk Reconcile only.
- Unified `delete_mod_thumbnail` with the `InternalMutation` Disk Reconcile path under watcher suppression.
- Removed legacy public runtime command surface for `sync_objects_cmd` and `gc_lost_objects_cmd`.
- Clarified legacy collection preview fallback as compatibility-only and synced onboarding/object-list/runtime boundary docs.

## Validation

- `cargo check --manifest-path src-tauri/Cargo.toml`
- `pnpm exec tsc --noEmit`
- `pnpm exec vitest run src/features/settings/tabs/GamesTab.test.tsx src/lib/services/scanService.test.ts src/features/file-watcher/hooks.test.ts`

## Residual Notes

- `runDeepmatchScanner` still exists only for explicit Deep Match Scanner flows such as Settings scan/import and scan service tests.
- `mapLegacyNodes` remains as a documented compatibility fallback for older collection preview payloads; it is no longer presented as the primary path.
