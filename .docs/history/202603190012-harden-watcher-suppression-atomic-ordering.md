# Harden watcher suppression atomic ordering

## Context

- Watcher suppression flag reads/writes used relaxed atomic ordering across threads.
- Mutation phases depend on deterministic suppression visibility to avoid racey watcher events.

## Changes

- Updated suppression guard writes to Release ordering when enabling and disabling suppression.
- Updated watcher callback suppression check to Acquire ordering.
- Updated watcher suppression test unsuppress store to Release ordering for consistency.

## Impacted Files

- src-tauri/src/services/scanner/watcher/mod.rs (modified)
- src-tauri/src/services/scanner/tests/watcher_tests.rs (modified)

## Goal

- Make suppression visibility deterministic between mutation threads and watcher callback thread.

## Impact

- Reduces risk of stale suppression reads during high-frequency mutation operations.
- No schema, command surface, or UI behavior changes.

## Notes

- Focused verification run passed for watcher suppression behavior.
