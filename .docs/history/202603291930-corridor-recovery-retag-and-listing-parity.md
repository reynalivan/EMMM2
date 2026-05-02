# Corridor recovery, retag cleanup, and listing parity guards

## Context

Recent corridor/privacy refactors fixed the main runtime path, but three gaps remained:
- interrupted switch/apply recovery still exposed only resume-or-clear behavior
- switch preview dialog still derived the empty target label by parsing a translated string
- safe/unsafe retag flows still relied on generic refresh timing to clear stale preview/selection

## Changes

- Recovery flow now uses a structured action contract (`RETRY`, `ROLLBACK`, `IGNORE`) instead of direct frontend re-invocation or blanket clear.
- Backend recovery resolution now marks the original pending task resolved and supports rollback for `switch_corridor` plus best-effort rollback for `apply_collection` using corridor pointers.
- Collection apply now preserves the previous active collection as `undo_collection_id` for recovery use.
- Switch preview dialog now uses explicit corridor label formatting for `all disabled`, `system fallback`, and unsaved labels.
- Safe/unsafe retag flows now publish path invalidation effects so preview and runtime selection clear deterministically when a mod exits the active corridor.
- Added backend parity guard for runtime corridor filtering to preserve req-11 metadata and prune conflicts correctly.

## Impacted Files

- `.docs/history/202603291930-corridor-recovery-retag-and-listing-parity.md` (added)
- `src/App.tsx` (modified)
- `src/lib/bindings.ts` (modified)
- `src/lib/corridorLabels.ts` (modified)
- `src/types/task.ts` (modified)
- `src/features/collections/components/RecoveryDialog.tsx` (modified)
- `src/features/collections/components/RecoveryDialog.test.tsx` (added)
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
- `src/features/safe-mode/ModeSwitchConfirmModal.test.tsx` (modified)
- `src/features/mod-runtime/actions/sharedModEffects.ts` (modified)
- `src/features/mod-runtime/actions/sharedModEffects.test.ts` (added)
- `src/features/workspace-runtime/actions/sharedRuntimeResultMapper.ts` (modified)
- `src/features/workspace-runtime/optimistic/applyOptimisticEffects.test.ts` (modified)
- `src/hooks/useFolderMutations.ts` (modified)
- `src/locales/en/collections.json` (modified)
- `src/locales/en/safe_mode.json` (modified)
- `src/locales/id/collections.json` (modified)
- `src/locales/id/safe_mode.json` (modified)
- `src/locales/zh/collections.json` (modified)
- `src/locales/zh/safe_mode.json` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/domain/task.rs` (modified)
- `src-tauri/src/pipeline/steps/update_corridor.rs` (modified)
- `src-tauri/src/services/explorer/tests/helpers_tests.rs` (modified)

## Goal

Corridor switch, recovery, listing parity, and safe/unsafe retag behavior now align more closely with req-30 and req-11 end-to-end, without relying on brittle UI parsing or timing-based cleanup.

## Impact

- Recovery dialog can now retry, roll back, or ignore pending critical tasks in a structured way.
- Retagging a mod across corridors clears stale preview/selection immediately instead of waiting for query refresh alone.
- Switch dialog labels are translation-safe.
- No DB migration was needed; existing corridor undo pointer is now reused for recovery.

## Notes

- `apply_collection` rollback is best-effort and uses existing corridor pointers/restorable state; it is not a full historical snapshot system.
