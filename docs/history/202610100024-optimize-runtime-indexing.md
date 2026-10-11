# 2026-10-10 — Runtime indexing efficiency and settlement

Implemented the approved runtime indexing audit in the existing architecture:
latest-wins hotkey admission with typed awaited results, explicit supersession,
guarded settlement/cleanup, one-pass scoped cache invalidation, single-read bounded
metadata observations and verified catalog/INI digest reuse. No dependency, schema,
worker pool or persistent cache was added by this change.

Review also closed older recovery replacing a newer waiter, old failed callers
restaging already-settled intent, and lost activation authority leaving Running.
Progress remains based on successful SQL and authoritative runtime completion.
The composed file-lock integration suite now lives at app level and uses public
feature APIs; a shared conflict test's raw JSON expectation was corrected.

Paired debug fixture evidence: 2,048-mod root invalidation + harvest median
1,194.578 → 804.123 ms (~32.7%); single-mod 1.154 → 1.096 ms with overlapping
ranges; one-root metadata 69.899 → 56.654 ms (~18.9%). Catalog 548.566 → 539.845 ms
did not establish a large gain. Full avatar integrity remains enforced.

Validation: 1,785 Rust tests passed / 32 ignored, four performance fixtures
explicitly passed, strict all-target Clippy and formatting passed, app/native
typechecks and architecture/scoped lint passed, 15 frontend integration tests
passed. Fresh native Windows E2E passed four cases in two specs, including three
real process restarts; 1,446 build inputs were stable through the run.

Native SHA256: `52f24a67230be45d95bd578014df1cf39ddfce5f2c282d5265f68902a3978298`.
Only owned temporary fixtures/private E2E profile were used; the installed app and
real GIMI library were not operated or modified. No commit or installation.

[Execution report and residual limits](../plans/startup-performance/runtime-indexing-execution-20261010.md)
and `.audit-output/runtime-indexing-implementation-20261010/` contain evidence.
Metadata still holds the existing writer transaction and is bounded by item count,
not bytes. Real large-library/cold-disk performance and in-game reload remain
outside this run's certification; no claim of zero remaining bottlenecks is made.
