# File watcher and suppressor: audit and adjustment plan

Status: four approved bug fixes implemented and scoped verification passed.
Audit source snapshot: 10 October 2026. Broader architecture remains proposed.
See `refactor_plan.md` for the four separately planned architecture follow-ups.

## Decision

Retain the modular monolith, notify debouncer, bounded receiver, operation
coordinator and journal-backed disk projection. Fix convergence and reuse
eligibility before extracting responsibilities. A new event bus, daemon,
permanent polling scan, second mutation lock or persistent suppression store
is not justified by these findings.

Bug Hunter bounded the behavioral findings and adversarial review. Karpathy
guidance kept the proposal incremental. The architecture skill informed explicit
ownership/interfaces; its generic reference templates were not used as evidence
for Rust behavior. Tokio wake semantics were checked through Context7 and the
[primary Notify documentation](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html).

## Confirmed bugs

All four are Medium: reachable convergence/availability defects, not demonstrated
data loss or unsafe disk overwrite. IDs retain audit hypothesis numbering.

| ID       | Trigger and failure                                                                                                                                                                               | Evidence                                                                                                          | Smallest correction                                                                                                    |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| WATCH-H1 | With runtime INI outside Mods, a sibling log/temp write reaches authority but not the receiver. It sets lost-event/full-repair state despite no relevant change.                                  | `watcher/mod.rs:563-577`, `lifecycle.rs:741-778`, `orchestrator/state.rs:1198-1207`; native observer probe.       | Apply one coverage relevance policy before authority observation, independent of suppression.                          |
| WATCH-H2 | Relevant external changes during a path/blanket guard, or unresolved echo expiry, can leave a ready-idle receiver asleep. Thumbnail success has no terminal reconcile to cover the external edit. | `suppressor.rs:364-379,600-601,940-954`, `lifecycle.rs:2100`, thumbnail command guard; native in-place INI probe. | Coalesced session-local dirty-work wake plus generation recheck under existing lease/backoff.                          |
| WATCH-H4 | An inactive watcher with an open channel but invalid identity cache is still eligible and handed off. First activation fails instead of rebuilding known-broken coverage.                         | Inactive checks `watcher/mod.rs:213-219,257-264,282-288`; isolated native-resource probe.                         | Shared inactive eligibility checks cache validity before reuse/discard/handoff; normal registration rebuilds.          |
| WATCH-H5 | Frontend retry IPC fails; backend subsequently full-repairs and emits an accepted revision. Event handling reasserts pending because full-required remains set.                                   | `useFileWatcher.ts:371-393,594-610`; real store clearing at `gameSlice.ts:531-540`; actual-hook RED probe.        | Schedule one coalesced current-epoch authority refresh; preserve stale-result guard and duplicate-revision settlement. |

WATCH-H3 was dismissed: production expected-echo registration and commit share
the same operation lease. A successor cannot enter the hypothetical interleaving.
Concurrent caller edits were reread; serialization remains. Keep this invariant.

## Proposed ownership

```text
OS event / backend error
  -> coverage relevance (not suppression)
  -> session-bound dirty/echo evidence
  -> typed event or coalesced work wake
  -> existing catch-up + operation lease
  -> committed disk projection
  -> runtime enqueue + frontend settlement
```

The channel carries event detail; dirty generations remain the source of truth.
A wake carries no second copy of pending roots. Suppression may delay optional
work but must not prevent eventual processing of relevant external evidence.

## Stage 1: focused correctness patches

1. WATCH-H4: one inactive-resource eligibility predicate reused by lookup,
   discard and handoff. Check root identity, runtime-parent binding, open receiver
   and shared cache validity. A failure after valid handoff still fails closed.
   Do not construct a new session merely to make its default validity true.
2. WATCH-H1: define coverage relevance separately from presentation relevance.
   Preserve visible Mods structural evidence, exact runtime INI, watched-root/
   runtime-parent destruction or rebinding, pathless error and rescan. Reject
   unrelated runtime siblings. Do not simply filter all observations through
   `keep_side`, because it includes suppression and can discard ownership loss.
3. WATCH-H2: attach one wake primitive to the actual session/resource lifetime.
   Notify after relevant dirty evidence, suppression release, unresolved expiry
   and abort/overflow repair. Inactive resources retain pending demand for takeover;
   an old session's wake cannot trigger the successor. Register wake before
   checking work and recheck generations after waking. Coalesce bursts; defer
   while guarded or the foreground mutation lease is occupied. Use the existing
   bounded backoff, not a permanent full-scan timer.
4. WATCH-H5: on an accepted recovery event with full-required still outstanding,
   coalesce a current-context check through the existing refresh scheduler. If a
   recovery is in flight, do not duplicate it; failure must retain demand for the
   next bounded opportunity. Capture failure epoch and allow the existing
   same-revision response path to settle UI. Do not unconditionally trust a Full
   result or remove epoch protection. Initially tolerate at most one redundant
   full authority check; receipt-based no-rescan optimization is a later decision.

Implement reproducible regressions first, then these narrow changes. No IPC,
schema, journal format or dependency change is required for Stage 1.

## Stage 2: simplify responsibilities after behavior is green

- `lifecycle.rs` combines registration, prewarm, activation publication, receiver
  loop, retry planning and Tauri integration. Move desktop orchestration into the
  existing workspace adapters, leaving pure plan/acceptance rules in application
  code. Extract by ownership and control flow, not an arbitrary file-length limit.
- Replace coupled `activation`, `activation_recovery_pending`, pending-plan and
  timer combinations with explicit publication/catch-up states only where a
  transition becomes easier to enforce. Reuse existing request/scope types;
  avoid another state machine mirroring the reconciliation coordinator.
- Couple installed watcher and its real session proof so active/inactive paths
  cannot independently implement health rules. Keep receiver ownership explicit
  and retain old coverage until a validated replacement is published.
- Keep blanket, path-scoped and identity-proven echo mechanisms: they serve
  different guarantees. Do not replace them with a timer-only suppression flag.
  Make lifecycle notifications an explicit contract rather than implicit caller
  expectations about terminal reconciliation.
- Frontend should consume revisions and own UI presentation, not become a second
  filesystem scheduler. Keep one in-flight/coalesced refresh path; centralize
  subscription cleanup/error handling if a second proven use requires it.

Each extraction should be a separate behavior-preserving step with the same
tests and independent review. No large rewrite prerequisite for the bug fixes.

## Stage 3: evidence-driven improvements, not confirmed bugs

- Measure ledger lock time and event lag for 100/1,000/8,192 expected edges.
  `lineage_proof` performs native filesystem reads under a synchronous ledger
  mutex and is called per edge. This is a performance risk, not a measured
  production regression in this audit. If material, validate a request-local
  proof/index once per dependency group; never reuse stale ownership proofs.
- Decide bounded rebuild policy for cache failure after admission: explicit
  recovery action now, automatic owned replacement only if justified by native
  tests. Distinguish temporary backoff from resource replacement needed.
- Consider a backend authority receipt for unsolicited frontend settlement only
  if the extra Stage 1 check is measurably costly. Bind root/session and covered
  observation generation; a naked revision or timestamp cannot prove coverage.
- Add diagnostic correlation for existing session/generation, pending-demand
  reason and retry/rebuild outcome using existing logging. No separate queue or
  observability subsystem.

## Race and regression acceptance matrix

| Group          | Cases that must pass                                                                                                                                                                                                                 |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Relevance      | Runtime sibling log/temp is ignored; exact INI changes wake; Mods edits remain dirty; root/runtime-parent rename/delete and pathless rescan retain full repair.                                                                      |
| Suppression    | External in-place INI, info.json and folder changes during thumbnail work; guarded write failure/cancellation; overlapping guards; last release without any second OS event/focus; no spin while guard/operation lease remains held. |
| Echo lifecycle | Early/split/folded/reversed echoes; missing side expires while idle; discard/abort; overflow; parent/child lineage; replacement identity never suppressed; expiry after session switch cannot dirty successor.                       |
| Handoff        | Healthy inactive backlog preserved; invalid-cache/open-channel resource rebuilt on first activation; root/runtime-parent replaced; failure between eligibility and publication; rapid A/B/A activation keeps newest readiness.       |
| Projection     | Evidence arriving during scan remains dirty; operation lock contention backs off; foreground toggles are not starved; DiskCommitted is never rolled back on projection failure.                                                      |
| Frontend       | IPC failure then backend Full event; new watcher failure after scan start; event-before-response and duplicate revision; game/root switch; unmount; stale repair never clears newer pending work.                                    |

Validation gates: scoped regressions and existing watcher/native-cache/authority
suites; frontend watcher suites; typecheck/build; strict Clippy and formatting;
fresh isolated native Windows E2E for switch, recovery and relevant-event flow.
Do not substitute browser-demo results for native IPC/disk behavior.

## Current audit evidence and limits

- Existing watcher/suppressor suite: 79 passed; frontend watcher: 55 passed.
- Three isolated Windows notify/resource probes reproduce WATCH-H1/H2/H4.
  The watcher-only fixture omits Tauri activation orchestration and imports the
  current suppressor/filter/cache; it does not certify a full application E2E.
- One actual-hook regression is intentionally RED for WATCH-H5: revision2 is
  accepted, but pending remains true. It is an audit reproducer, not a fixed test.
- Initial native fixture produced a late parent event; setup events were drained
  and an existing INI overwritten to isolate the no-wake case. No production
  events/filter or check were bypassed. Initial probe compile missed a session
  argument; corrected only the probe. Its unused-source warnings are recorded;
  this does not substitute for production strict Clippy.
- One-pass focused audit, not complete source/release/security coverage. Core
  production reads and caller slices are recorded in `.bug-hunter/watcher-audit/`.
  Five out-of-focus sources changed concurrently and remain failed coverage;
  original hashes were preserved. Current confirmed-source hashes remained stable.
- No production source fix, dependency update, game launch, real mod mutation,
  full native UI E2E, restart crash-recovery certification or Linux check occurred.
- Final frontend `tsc --noEmit` passed; no full production/native app rebuild
  was performed in this audit.

The subsequent user request approved WATCH-H1/H2/H4/H5 bug fixes. Current scoped
patches add coverage gating, session work notification/loop consumption, invalid
inactive-cache rejection and a current-epoch frontend recovery follow-up. Existing
identity/journal/lease safeguards remain. Verification is recorded separately in
`.bug-hunter/watcher-fix/validation.md`; the audit evidence above is historical.

Approval is still required before executing the broader `refactor_plan.md`.
Recheck the shared checkout and freeze that scope then; this report is not
blanket write authority.
