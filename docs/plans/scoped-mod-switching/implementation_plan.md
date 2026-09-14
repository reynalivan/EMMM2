# Scoped mod switching improvements

User authorized implementation after the read-only audit.

1. Keep folder-conflict reports and manual repair entrypoints, but stop passive watcher reports from opening a modal over unrelated work.
2. Treat reported bulk path rewrites, including an empty list, as authoritative. Do not infer a rename for a no-op.
3. Separate Force Enable from Enable Only This and keep prepared target paths consistent with canonical rename paths.
4. Require an applied disk projection before closing toggle journals; preserve failed compensation for recovery and propagate pending runtime-sync warnings.
5. Run regression tests with temporary folders/in-memory databases, frontend checks, and focused read-only review. Preserve all pre-existing changes; do not operate on user mod storage.

No schema, dependency, new state machine, collection snapshot policy, or visual redesign changes.

Completed: the five steps above, including ignored-warning and Windows path-spelling regressions. Validation and remaining native E2E coverage gaps are recorded in `docs/history/202609149001-scoped-mod-switching.md`.

Follow-up completed after approval: bulk mutation inputs are strict descendants of the configured Mods root, missing folders are per-item failures, canonical aliases deduplicate, and ancestor/descendant selections fail without mutation. Terminal bulk results now report cancellation and processed/unprocessed counts. Workspace and randomizer paths use mod IDs for sibling exclusion; a disabled parent must be explicitly enabled before its child. Runtime counters/safety now use effective activity, and switch effects invalidate only affected mod-health reports. The legacy direct workspace switch executor was removed in favor of prepared switch + durable journal flow. Details and validation are recorded in `docs/history/202609149003-enable-disable-hardening.md`.
