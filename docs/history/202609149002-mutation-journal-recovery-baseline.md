# Mutation journal recovery and baseline

## Context

The mutation journal rewrites a full durable snapshot at every step. Crash-recovery artifacts needed validation before any write-volume optimization.

## Changes

- Added versioned, revisioned, checksummed journal snapshots that bind the exact operation ID list.
- Startup preserves a valid canonical snapshot, restores the newest valid versioned artifact only when canonical is absent, and stops with a repair error for ambiguous unversioned artifacts.
- Added simulated replacement-crash coverage, checksum tamper coverage, legacy migration coverage, and an ignored manual benchmark for bulk journal persistence.

## Impacted Files

- `src-tauri/src/modules/mutation/journal.rs` (modified)
- `src-tauri/src/modules/mutation/tests.rs` (modified)
- `src-tauri/AGENT.md` (modified)
- `docs/history/202609149002-mutation-journal-recovery-baseline.md` (added)

## Goal

Journal recovery no longer silently starts empty when replacement artifacts are present, and P2 has a reproducible write baseline.

## Impact

- No mutation guard, `sync_all`, atomic write, rollback, or reconcile behavior was relaxed.
- The benchmark remains ignored because its full matrix intentionally performs many synchronous temporary-file writes.
- Baseline medians: 500 steps wrote 76.7 MB at history 1 and 127.4 MB at history 256; 1,000 steps wrote 305.5 MB at history 1 and 406.5 MB at history 256. Successful-commit p50 ranged from 4.2 s to 20.2 s.

## Notes

The active bulk plan is the dominant cost. Splitting terminal history alone will not solve P2; a later immutable-plan plus compact per-step-state design can retain per-step durability while reducing write volume.
