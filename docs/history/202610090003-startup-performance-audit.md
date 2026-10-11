# Audit configured startup performance

## Context

Continue validation after the earlier backfill/initialization fixes and identify
why the installed application's restart remains slow.

## Changes

- Add an ignored, read-only watcher profiling harness and audit evidence.
- Confirm watcher installation costs 26.09–28.90 seconds before timed indexing.
- Trace same-game activation discarding the verified startup watcher and scanning
  again. Confirm installed binary still contains legacy missing-table queries.
- Record current build/test results and unrelated validation failures.

## Impacted Files

- `src-tauri/tests/startup_watcher_audit.rs` (added)
- `docs/plans/startup-performance/audit-20261009.md` (added)
- `docs/plans/startup-performance/validation.md` (modified)
- This history file (added)

## Goal and Impact

Prioritize watcher reuse and identity-cache work using measurements. No application
behavior, installed binary, production database or mod files changed. Diagnostic
NoCache is not a production recommendation; rename proofs remain required.

## Validation

Build passed; startup tests 86 frontend/6 native passed; profiling harness passed
twice. Full snapshots found 4 frontend failures and 1 native failure in other
Storage Optimizer work. Details and Clippy/formatting boundaries are in the audit.
