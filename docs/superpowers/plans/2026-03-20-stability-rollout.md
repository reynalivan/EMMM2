# Stability Rollout Plan (Collections + Corridor Privacy)

## Scope

- Backend orchestration simplification (`apply.rs`, `privacy/mod.rs`)
- Pointer/runtime authority hardening (`corridor_runtime.rs`, `corridor_state_repo.rs`)
- Frontend invalidation + deterministic toggle flow (`useCollections.ts`, `useSafeModeToggle.ts`, `invalidateCollectionRuntime.ts`)
- Race-focused regression tests (`privacy_service_tests.rs`, `useSafeModeToggle.test.tsx`)

## Rollout Slices

### Slice A — Backend orchestration safety

- Deliver explicit phase helpers in `apply.rs` and `privacy/mod.rs`.
- Enforce strict nested preflight failure propagation.
- Gate:
  - `cargo test test_apply_collection_atomic -- --nocapture`
  - `cargo test test_switch_mode_only_disables_leaving_corridor -- --nocapture`

### Slice B — Runtime authority + stale pointer self-heal

- Clear stale active/undo pointers during preview resolution.
- Keep fallback behavior unchanged (undo snapshot preferred when active pointer stale).
- Gate:
  - `cargo test test_preview_corridor_switch_falls_back_to_undo_when_active_pointer_is_stale -- --nocapture`
  - `cargo test test_preview_corridor_switch_returns_none_when_active_and_undo_pointers_are_stale -- --nocapture`

### Slice C — Frontend determinism and scope reduction

- Narrow apply invalidation scope (exclude dashboard).
- Make invalidation deterministic with awaited Promise batching.
- Add in-flight guard to block duplicate safe-mode switch requests.
- Gate:
  - `pnpm vitest src/features/collections/hooks/useCollections.test.ts --run`
  - `pnpm vitest src/features/collections/utils/invalidateCollectionRuntime.test.ts --run`
  - `pnpm vitest src/hooks/useSafeModeToggle.test.tsx --run`

### Slice D — Full regression sweep

- Backend focused sweep:
  - `cargo test privacy_service_tests -- --nocapture`
  - `cargo test collection_cmds_tests -- --nocapture`
- Frontend focused sweep:
  - `pnpm vitest src/features/safe-mode/ModeSwitchConfirmModal.test.tsx --run`
  - `pnpm vitest src/stores/useAppStore.test.ts --run`

## Deployment Safety Rules

- Do not merge slices together unless all gates in current slice are green.
- Any failure in Slice A/B blocks frontend changes from merging.
- Preserve FS truth model (`DISABLED ` prefix) and command contracts; no UX expansion.
- Keep rollback/error semantics explicit in release notes for QA verification.

## Current Gate Status (2026-03-20)

- Slice A gate: ✅ Passed
  - `cargo test test_apply_collection_atomic -- --nocapture`
  - `cargo test test_switch_mode_only_disables_leaving_corridor -- --nocapture`
- Slice B gate: ✅ Passed
  - `cargo test test_preview_corridor_switch_falls_back_to_undo_when_active_pointer_is_stale -- --nocapture`
  - `cargo test test_preview_corridor_switch_returns_none_when_active_and_undo_pointers_are_stale -- --nocapture`
- Slice C gate: ✅ Passed
  - `pnpm vitest src/features/collections/hooks/useCollections.test.ts --run`
  - `pnpm vitest src/features/collections/utils/invalidateCollectionRuntime.test.ts --run`
  - `pnpm vitest src/hooks/useSafeModeToggle.test.tsx --run`
- Slice D gate: ✅ Passed
  - `cargo test commands::collections::collection_cmds::tests -- --nocapture`
  - `cargo test services::privacy::tests -- --nocapture`
  - `pnpm vitest src/features/safe-mode/ModeSwitchConfirmModal.test.tsx --run`
  - `pnpm vitest src/stores/useAppStore.test.ts --run`

## Notes

- Added backend regression: no-op apply path (`test_apply_collection_noop_when_state_already_matches_target`).
- Updated legacy undo regression test to align with current undo pointer contract (`corridor_state.undo_collection_id` must be set).
- Completed object-path apply hardening in `apply.rs`: single transaction for DB updates after FS phase plus deterministic FS rollback if DB commit fails.
- Revalidated backend sweeps after transactional batch change:
  - `cargo test commands::collections::collection_cmds::tests -- --nocapture`
  - `cargo test services::privacy::tests -- --nocapture`
- Existing compile warnings remain unrelated to this rollout slice (`SignatureMatchType` and signature helper dead-code warnings).
