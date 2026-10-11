# Recheck native overlay preconditions

## Context

The user requested continued validation after the focused modifier overlay failed.
Serialize the one overlay-only rerun with the startup benchmark task.

## Changes

Add bounded passive fixture key-transition diagnostics without altering input or
assertions. Run a fresh private native overlay-only case and retain its failed
setup evidence. No production changes or speculative fix.

## Impacted Files

- `tests/e2e/support/hotkey-input-window.cs` (passive diagnostics).
- `docs/plans/mods-manager-hardening/global-hotkey-native-validation.md` (results).
- This history entry (added).
- Ignored `.audit-output/global-hotkeys-native-20261011-g.log` and evidence directory.

## Goal and Impact

Collect reliable modifier ordering evidence once fixture setup succeeds. No new
dependency, schema, production input behavior or real library access. Helpers and
one owned fixture cleaned after verified shutdown; shared slot released.

## Verification

Fresh frontend/native build and receiver warnings-as-errors compilation passed.
Before-all failed while preparing the third preset: enable Unsafe was rolled back,
while the private DB remained Safe enabled/Unsafe disabled. Watcher identity
PermissionDenied was logged. No OS key was sent, no apply task existed, and overlay
was not rerun. Causality remains unproven; per-item setup responses need evidence.
Previous run F remains 4 passed/1 failed; this run adds no passing hotkey coverage.
