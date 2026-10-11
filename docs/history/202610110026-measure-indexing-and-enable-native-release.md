# Measure indexing and enable native release validation

## Context

Harvest dispatch and onboarding root skew needed release measurements before
changing production scheduling. Final native validation also needed a release
fixture profile and real Windows folder selection.

## Changes

- Run four optimized fixture benchmarks with source hashes and paired summaries.
- Allow the isolated fixture harness to build release; keep `readonly-audit`
  exclusive to the readonly profile and retain identifier/build/reset guards.
- Resolve the native picker executable from the validated build mode.
- Use absolute spec paths in nested audit configs so WDIO selects the intended tests.
- Prepare a bounded command runner and the combined 15-case native release run.

## Impacted files

- `wdio.conf.ts`; `tests/e2e/specs/onboarding-indexing-native.e2e.ts`.
- `docs/plans/startup-performance/final-performance-audit-20261011.md`; this history.
- Ignored `.audit-output/final-performance-audit-20261011/`:
  `run-bounded-command.mjs`, `summarize-benchmarks.mjs`, `snapshot-inputs.mjs`,
  `test-native-release-config.mjs`, `wdio.final-fixture-release.conf.ts`,
  `tsconfig.native-harness.json`, and release/debug evidence directories.
- Ignored `.audit-output/bootstrap-bundle-20261011/`:
  `wdio.wpr-readonly.conf.ts`, `test-wpr-gates.mjs`.

## Verification

Four release benchmarks PASS; 695 Rust inputs unchanged. E2E/harness typechecks,
scoped ESLint, formatting, config/handshake contracts and diff checks PASS.
Fresh native release: 15/15 cases PASS, zero skipped, 1,577 inputs unchanged.
Real Windows pickers selected owned fixtures; 6,000+80-mod onboarding reached
Home in 2.48 seconds after Confirm. Sync/retry/supersession, restart, progress and
preview/Dashboard checks passed. Owned shutdown/cleanup and installed DB hash
verification passed. WPR remains pending the production-update slot and operator.

## Goal and impact

Batch 4 reduced full-scope fixture harvest medians by 11.9–14.8%; it remains a
test prototype. Skewed census spent about 97% of its time in the first batch,
so removing the next-batch boundary alone cannot solve the dominant cost.
No production scheduler, batching, dependency or whole-app speedup is claimed.
Real GIMI and installed configuration were not changed.
