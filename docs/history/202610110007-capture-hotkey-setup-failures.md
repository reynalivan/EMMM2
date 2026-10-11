# Capture hotkey setup failures and input provenance

## Context

The user approved diagnosing bulk setup first, then fixing confirmed overlay
replay defects. Preserve concurrent startup measurements and production guards.

## Changes

- Retain and immediately assert complete per-operation setup BulkResult or IPC
  errors instead of losing failures behind a later disk timeout.
- Retain final passive receiver status through verified cleanup.
- Record keyboard message metadata losslessly alongside bounded modifier events.
- Document the approved remediation sequence and the remaining native prerequisite.

## Impacted Files

- `tests/e2e/specs/global-hotkeys-native.e2e.ts` (modified).
- `tests/e2e/support/hotkey-input-window.cs` (modified).
- `docs/plans/mods-manager-hardening/hotkey-replay-remediation-plan.md` (added).
- This history (added).

## Goal and Impact

Identify real step failures and distinguish original input from replay evidence.
No production behavior, dependency, schema, real-library or assertion relaxation.
One read-only Explorer mapped cache failures and intentional zero-applied rollback.

## Verification and Limits

E2E typecheck, scoped lint/format and whitespace checks passed. Receiver API was
checked against official Windows documentation; C# compilation and native run H
are pending the shared slot. No new OS input or production fix is claimed.
