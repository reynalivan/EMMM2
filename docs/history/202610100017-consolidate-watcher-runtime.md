# Consolidate watcher policy and separate native orchestration

## Context

Execute the approved, audited watcher consolidation/Tauri separation plan.
Preserve the shared dirty checkout and existing activation/onboarding/authority
contracts. No commit, dependency, IPC, journal or schema change in this scope.

## Changes

- Share ordered lexical endpoint policy before authority observation and event
  delivery; capture filesystem directory facts explicitly for content only.
- Separate binding evidence from real session cache/Notify ownership; unify
  inactive lookup/discard/handoff while preserving mode-specific admission.
- Consolidate receiver/work/backoff waiting without moving lifetime retirement
  into the wait helper or dropping clean retry deadlines.
- Move Tauri registration, onboarding, event processing, publication and runtime
  enqueue from core lifecycle to one runtime adapter. Migrate activation,
  settings, reconciliation and bootstrap callers through `workspace::api`.
- Preserve 20 baseline lifecycle tests; extend endpoint, binding, buffered
  handoff, registration/receiver/onboarding/reconcile cancellation and lease
  tests. Add an isolated native external-change/switch/reuse spec.

## Impacted files

Watcher core (`mod`, `event_filter`, `suppressor`, `registration`, `lifecycle`),
watcher tests, workspace Tauri adapters/API, settings and reconciliation Tauri
caller imports, bootstrap imports, backend guide, plan execution evidence and
`tests/e2e/specs/watcher-consolidation.e2e.ts`. Exact source hashes and dirty
baseline provenance are retained in the linked execution report.

## Validation

360 Rust tests passed, 4 manual benchmarks ignored; 67 frontend tests passed.
Strict Clippy, TypeScript typecheck, scoped formatting/lint/whitespace and
architecture scans passed. Independent read-only review found two binding
parity regressions; both received observed RED then integrated GREEN.
Fresh private Windows E2E passed 3 cases across 2 specs, including external
watcher convergence, switch/reuse and 3 real process restarts. Owned teardown
completed and 3 private game fixtures were removed. Full results and source
provenance are in [execution.md](../plans/file-watcher-stability/execution.md).

## Remaining acceptance gap

Controlled suppression release verifies wait/reconcile/SQL/result boundaries,
not complete Tauri-loop publication. Deterministic full-loop last-owner release,
expiry/abort and event/runtime publication remain unverified; no public bypass
or speculative generic runtime layer was added to hide that limitation.
