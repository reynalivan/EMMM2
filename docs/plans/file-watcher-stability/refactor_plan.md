# Audited watcher consolidation and Tauri separation plan

Status: execution approved on 10 October 2026. Consolidation and runtime relocation
are implemented; fresh validation and remaining acceptance gaps are tracked in
[execution.md](execution.md). The design audit below remains the approved contract.

WATCH-H1/H2/H4/H5 are already fixed. Retain their coverage gate, real-session
Notify, invalid-cache rejection and frontend recovery follow-up. Do not repeat
those fixes or treat their previous validation as certification of this refactor.

## Decision

Keep the modular monolith, notify debouncer, bounded receiver, existing
MutationCoordinator/journal and authoritative dirty/repair generations.
Consolidate existing decisions and relocate orchestration without changing
disk-first settlement, activation acknowledgements or frontend event fields.

No new event bus, daemon, full-scan polling, second lock/coordinator, persistent
suppression cache, dependency, public IPC, or generic dependency-injection
framework. Binding comparison is not ownership; a wake is not work demand.

The independent plan audit and corrected gaps are in
[plan_audit.md](plan_audit.md). Source evidence is bound to
[plan_audit_source_snapshot.json](plan_audit_source_snapshot.json); inspect fresh
source and approve an exact implementation scope before execution.

## Current contracts to preserve

| Boundary           | Current guarantee that must survive                                                                                                                                                                    |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| OS observation     | Coverage is evaluated before authority observation; trusted echo matching receives the original kind and ordered paths. Delivery may classify each endpoint differently.                               |
| Registration       | Native construction stays in the existing blocking worker; an unadmitted candidate cannot mutate active authority. Old coverage lives until a replacement passes admission.                            |
| Real session       | Resource clones share cache validity and Notify. Equivalent reconstructed bindings are comparison evidence only, never receivers, repair owners or health sources.                                     |
| Dirty demand       | Authority/repair generations own pending work; Notify carries no duplicate paths. Clean wake retains an existing retry deadline.                                                                       |
| Receiver lifetime  | One actual receiver/consumer. Normal close, early return and cancellation end only that session and recovery generation; no successor retirement.                                                      |
| Onboarding         | Watcher-before-scan, raw snapshot lease, activation claim guard, post-apply repair and buffered-event drain retain their ordering. Prepared discovery is not an accepted projection.                   |
| Ordinary batch     | Projection completes and reconciliation leases release before optional runtime enqueue.                                                                                                                |
| Activation Ready   | Keep the current acceptance operation guard and current-activation publication envelope, including its runtime enqueue, until publication finishes. Do not move it after release as a generic cleanup. |
| Mutation authority | Existing root/session/native identity, admission and journal barriers remain; acknowledged DiskCommitted never becomes a rollback candidate.                                                           |

Evidence: watcher `mod.rs` classification; `registration.rs` candidate admission;
`lifecycle.rs` receiver Drop, initial recovery/claim and publication paths;
reconciliation orchestrator state/leases. References are in the audit document.

## Target ownership and dependency direction

```text
cross-module integration callers -> workspace::api
watcher_activation adapter -> watcher_runtime adapter -> watcher application
                                                        -> reconciliation::api
watcher application resource/registration -> existing notify adapter

No watcher_runtime -> watcher_activation back-edge.
No application watcher -> Tauri adapter back-edge.
```

Existing reconciliation/workspace data dependencies are not eliminated merely
by using a facade. Pure planning consumes facts; it does not recurse into
another runtime scheduler.

| Location                                      | Responsibility after migration                                                                                                                               |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `application/scanner/watcher/event_filter.rs` | Lexical coverage, endpoint delivery policy and explicit content facts. No Tauri or hidden filesystem reads in pure decisions.                                |
| `watcher/suppressor.rs`, `watcher/mod.rs`     | Actual session/resource ownership, suppression/echo proof, binding comparison and shared wake; retain current native construction boundary.                  |
| `watcher/registration.rs`                     | Owned candidate/blocking construction and guarded admission; no Tauri.                                                                                       |
| `watcher/lifecycle.rs`                        | Tauri-free catch-up/acceptance helpers. Label stateful lease/identity checks as such; only fact-to-plan/retry decisions are pure.                            |
| `adapters/tauri/watcher_runtime.rs` (new)     | App-state reads, observation wiring, start/handoff/prewarm, receiver task, publication/events/runtime enqueue. One existing scheduler moved, not duplicated. |
| `adapters/tauri/watcher_activation.rs`        | Selected-game join/reuse orchestration; consumes runtime publication helpers.                                                                                |
| `workspace/api.rs`                            | Explicit selected integration exports plus existing core types. No new frontend commands.                                                                    |

Start with one new runtime adapter. Add one small core policy/binding file only
if an extracted responsibility has demonstrated reuse and cannot remain cohesive
in an existing file. Do not split by a line-count threshold or create a framework.

## 1. Consolidate event relevance

Define one internal policy result retaining the original event and ordered
per-endpoint facts. Do not use a single four-state enum that loses rename sides.

Separate:

- lexical authority coverage, including root/ancestor loss and exact runtime INI;
- content/structural delivery eligibility per endpoint;
- explicit coverage-loss/rescan evidence;
- suppression decisions, applied after observation and echo handling.

Order: policy -> original-event echo/authority observation -> endpoint suppression
-> typed delivery, or coalesced wake for retained demand without delivery.

A whole event with an eligible endpoint must retain its original paired evidence;
do not feed a filtered/reordered path list into the rename ledger. Preserve:
visible-to-hidden/outside becomes Removed; reverse becomes Created; two delivered
rename sides remain Renamed. One suppressed side follows current degradation.
Unknown/ambiguous event shapes keep current conservative behavior rather than
guessing a rename.

Current deep dotted content classification uses `path.is_dir()`. Keep that fact
acquisition explicit at the existing I/O boundary; pure policy accepts a known,
missing or unavailable file-kind fact. Removed/historical rename paths must not
be canonicalized through a live filesystem lookup. Retain current physical
namespace and hidden-directory rules, not logical DISABLED aliases as ownership.

Acceptance: ordered rename matrix, runtime sibling noise versus exact INI,
deep dotted directories versus assets, disappearance/stat failure, hidden runtime
staging, explicit dot-named roots, suppressed ancestor loss and pathless rescan.

## 2. Consolidate active/inactive eligibility

Use a common binding comparison with fresh root/runtime-parent identity evidence
and health from the actual resource. Native evidence acquisition is I/O, not a
pure helper. Keep binding values separate from session-owned validity/Notify.

Use thin admission-mode checks:

- new candidate: registration generation, current intent/config and unadmitted
  observation rules;
- active reuse: actual installed session, existing authority/liveness and
  acceptance barriers;
- inactive handoff: matching game binding, original buffered receiver and
  resource health; retain its existing generation-returning lookup API when
  callers need only a generation.

Do not apply the newest allocation-generation check to every reused resource:
prewarming another game can allocate a newer session without retiring valid
active/inactive coverage. A matching value snapshot is not permission to publish.

Revalidate at managed publication under the existing intent/config/session
envelope. Preserve native root checks, health invalidation and old live coverage
on failed registration. Do not add a new lock or claim atomicity against external
filesystem replacement; later loss must still dirty/invalidate coverage.

Acceptance: all three inactive checks, active/candidate symmetry, invalid real
cache versus same-valued reconstructed binding, older valid prewarm session,
replacement between capture/admission, open-channel invalid cache, and retained
receiver backlog.

## 3. Consolidate wait and catch-up consumption

Extract one cancellation-safe wait boundary around the existing typed receiver,
actual session Notify and optional retry deadline. Outcomes: event, work wake,
deadline or resource closure. It neither scans, publishes nor owns another queue.

The runtime loop remains the single consumer and rereads current authoritative
demand before nonblocking reconciliation. Preserve clean-wake deadlines,
foreground yield/backoff and bounded immediate catch-ups. Session replacement
and cancellation remain owned by the outer runtime, not the helper.

Move `WatcherReceiverLifetime` with the runtime task and keep its Drop alive
across every await/early return. Do not let helper cancellation retire a live
session, and do not let task cancellation skip retirement. Preserve raw onboarding
lease/claim ownership across initial recovery and its event drain.

Acceptance: idle/backoff cancellation, repeated wake/event races, last guard
release without another OS event/refocus, expiry/abort, A/B/A activation, receiver
closure during recovery and old Drop not finishing a successor.

## 4. Move Tauri orchestration out of lifecycle.rs

Perform a mechanical relocation only after contracts and policy consolidation
are green. Move app handles/state access, emission, telemetry, runtime publication,
watcher installation/prewarm and the receiver loop into the runtime adapter.
Keep selected-game join/reuse in the activation adapter, importing runtime helpers
in one direction. Keep pure types and stateful Tauri-free acceptance distinct.

Preserve every lease/guard acquisition, lifetime and recheck before simplifying
control flow. In particular, functions receiving an already-held activation/game/
operation guard must not acquire that same non-reentrant lock again. Do not await
new blocking native work under a synchronous mutex; keep existing spawn_blocking.

### Caller and test migration

Migrate these known entrypoint consumers and their facade in the same compiling
checkpoint as the relocation:

| Consumer                                               | Symbols/behavior to route through workspace::api                                                                   |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| `settings/adapters/tauri/settings_cmds.rs`             | Start/restore selected watcher, inactive handoff, readiness and WatcherActivation values.                          |
| `reconciliation/adapters/tauri/disk_reconcile_cmds.rs` | Active/inactive source repair and onboarding install scopes.                                                       |
| `system/application/app/bootstrap.rs`                  | Startup-selected/inactive coverage and selected watcher startup; bootstrap's broader architecture is out of scope. |
| `workspace/adapters/tauri/watcher_activation.rs`       | Reuse/join, current-authority publication and failure helpers.                                                     |
| `workspace/api.rs`, adapter `mod.rs`, watcher module   | Selected private integration exports, module registration and core type visibility.                                |

Do not preserve an old `application::...::lifecycle` Tauri entrypoint by forwarding
it back to an adapter: that recreates the forbidden dependency. Temporary
compatibility, if necessary, lives in `workspace::api`, never application code.
Prefer atomic caller migration over a long-lived compatibility path. Keep IPC
registration, payloads, generated bindings and frontend epoch logic unchanged.

Move policy tests with policy and runtime/lease/cancellation tests with the adapter.
Update private test imports/module paths without weakening assertions. Preserve
native watcher stress, registration, raw snapshot and authority tests; do not
expose a new production test endpoint to make them compile.

## Execution checkpoints

| Step | Deliverable                                                                      | Gate before proceeding                                                               |
| ---- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| P0   | Fresh source scope, current ownership/lock contracts and characterization tests. | Existing watcher fixes remain green; capture current signatures and guard lifetimes. |
| P1   | Shared endpoint-aware event policy.                                              | Parity/noise/root-loss and native observer regressions.                              |
| P2   | Binding evidence and mode-specific eligibility.                                  | Candidate/reuse/handoff/backlog and replacement race tests.                          |
| P3   | One wait boundary; existing task retains lifetime ownership.                     | Idle/backoff/cancellation and independent recovery tests.                            |
| P4   | Runtime adapter + same-checkpoint facade/caller/test migration.                  | Compile, structural dependency checks and lease/publication ordering tests.          |
| P5   | Remove compatibility residue and update backend guide.                           | Full focused verification + isolated native integration and independent review.      |

Each checkpoint is a small scoped diff. If parity fails, correct or undo only that
owned delta against its dirty baseline; preserve unrelated work. If characterization
reveals a new behavior defect, report it and obtain separate fix authority rather
than hiding a behavior change inside the refactor.

## Acceptance evidence and remaining gaps

Historical bug-fix evidence: 180 scoped Rust tests, 61 frontend tests and one native
startup case with three restarts. Those results are not rerun or newly certified
by this plan-only task. The test inventory is anchored by source in the audit.

Required additional coverage before accepting the refactor:

- full ready-idle receiver -> dirty demand -> DB/result convergence under a
  controlled suppression-release/expiry scenario, with no second event/refocus;
- mixed ordered rename endpoints and one-side suppression with unchanged echo
  proof input;
- binding/health invalidation between capture and publication, including a newer
  allocation that must not invalidate an older live resource;
- cancellation during registration, idle, backoff, onboarding claim and reconcile;
- activation publication retains its acceptance envelope, while ordinary runtime
  work starts after reconciliation leases release.

Use existing test seams/owned fixtures; no public IPC bypass. Extend native Rust
integration for deterministic internal barriers and freshly built isolated Windows
UI E2E for startup, switching and observable convergence. A signal-only test or
three startup restarts alone does not certify the full matrix.

Verification after implementation: watcher/cache/orchestrator and frontend watcher
suites; strict Clippy; scoped formatting/lint and build/typecheck; module/caller
checks proving no Tauri/app-handle/manager or Tauri-adapter import remains in the
application watcher subtree; fresh native harness with existing profile, ownership
and cleanup guards. Do not substitute a browser demo.

Native timing and external replacement remain residual risks. No snapshot proves
perpetual coverage; keep final revalidation, dirty-generation continuity, invalid
cache rebuild and fail-closed readiness.
