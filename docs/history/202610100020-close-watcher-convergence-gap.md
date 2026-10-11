# Close native watcher convergence verification gap

## Context

User requested closing the remaining full-Tauri-loop suppression verification
gate after the watcher refactor.

## Changes

- Add a feature-only native fixture, private-profile/temp-ownership gated and
  armed after UI setup; normal builds contain no probe or new IPC.
- Observe real wait/work/deadline and publication boundaries. Bind each accepted
  revision to its exact runtime enqueue generation and terminal artifact status.
- Exercise last-owner release, real echo expiry and live abort without a second
  rename/refocus. Retain errors/report before owned fixture cleanup.
- Review corrected early-abort, coalesced-wake and generation-correlation proof
  weaknesses; no production bug fix or clock/TTL alteration was needed.

## Impacted Files

- `src-tauri/Cargo.toml` (feature added).
- `src-tauri/src/modules/workspace/adapters/tauri/{mod.rs,watcher_runtime.rs}` (feature wiring/passive hooks).
- `src-tauri/src/modules/workspace/adapters/tauri/watcher_verification.rs` (added).
- `src-tauri/src/modules/workspace/application/scanner/watcher/suppressor.rs` (feature-only introspection).
- `wdio.conf.ts` (optional feature build; normal behavior retained).
- `wdio.watcher-verification.conf.ts` (added).
- `tests/e2e/specs/watcher-suppression-native.e2e.ts` (added).
- `src-tauri/AGENT.md`, `docs/plans/file-watcher-stability/execution.md` (updated).
- `.audit-output/watcher-gap-verification/native-report.json` (generated evidence).
- `docs/plans/file-watcher-stability/verification_source_snapshot.json` (added provenance).
- This history entry (added).

## Goal and Impact

Close the documented suppression-to-accepted-result/runtime-publication gap
using the actual native loop. No dependency, schema, journal, public command,
generic runtime framework or normal-build scheduler change. All fixture I/O is
owned/private; actual game reload remains outside this fixture's claims.

## Validation

80 core watcher tests, 19 runtime tests and 2 safety tests pass; strict feature Clippy, app/E2E
typechecks, scoped lint/format/architecture checks pass. Fresh native case passes
all 3 scenarios: revisions 6/7/8 correlate to runtime generations 3/4/5 and
Published artifacts. Owned teardown removes 1 private fixture. A prior failed
diagnostic is excluded from coverage. Full evidence and residual platform risks
are in [execution.md](../plans/file-watcher-stability/execution.md).
