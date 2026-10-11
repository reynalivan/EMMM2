# Mods lifecycle remediation

User approved remediation of the lifecycle audit and requested rechecking current code.
Preserve the shared dirty checkout; use exact file snapshots, no stash, branch switch,
worktree, commit or production-library operations.

## Plan

1. Canary MM-L4: add empty-list navigation regressions, require an in-range item before focus changes. Verify existing keyboard behavior.
2. MM-L2: bind the existing destination ancestry before prepare/execute/retry; reject non-contained or rebound targets in both discovery and execution. Preserve no-overwrite and compensation.
3. MM-L3: capture the original collection root before preparation and journal that identity; recovery must preserve replacement roots. Retain legacy evidence for manual recovery.
4. MM-L5: reproduce SQL-finalized/journal-uncommitted recovery, then add the user-approved operation/game/root receipt in the collection finalization transaction. Journal format 4 distinguishes the new protocol; read formats 1–3, fail closed for legacy ambiguity, and retain evidence for compatible recovery.
5. Recheck current watcher/suppressor, projection, paging and structural recovery code; retain existing fixes, add only verified missing behavior.
6. Run scoped tests, type/lint/build checks and owned native Windows E2E. Record gaps explicitly; never certify a browser fixture as native behavior.

## Acceptance

- Empty/search-filtered grids accept navigation keys without exceptions.
- Organizer destination ownership is validated through the original Mods namespace.
- Collection recovery refuses to mutate a different physical root.
- Any implemented commit-window repair is operation-bound, durable and idempotent.
- Native fixtures preserve assets and isolated app state; no real game or library changes.
- Report every remaining manual decision and failed/unexecuted check.

## Independent-review corrections

The initial fixes passed their narrow regressions, but independent review found
two missing boundary cases. A separate immutable correction scope and baseline
were created; the earlier manifests and recovery evidence were preserved.

- MM-L2-R1: nested missing destinations and raced directories must be created
  through retained native parent handles, never adopted or followed through
  `create_dir_all`. Existing ancestry is captured before asynchronous preparation.
- MM-L3-R1: collection runtime/object retries and compensation must retain the
  original root namespace, not just source and immediate-parent identities.
  Source publication uses owned handles; trusted reconcile holds verified
  post-mutation ancestry and destination participants across its await. Pins
  for intentionally renamed objects must be rebased and released before rollback.
- MM-L3-R2: successful stale-spelling no-ops are checked runtime participants,
  even though they have no physical journal step. Pin their original-identity
  destination leaves through projection too. This follow-up has a separate
  immutable two-file baseline and bounded fix batch.

Verification uses owned temporary filesystem/SQLite/journal fixtures. Native
UI coverage uses the isolated E2E identifier and a freshly built binary. These
checks do not authorize mutations in a real user library or configuration.
