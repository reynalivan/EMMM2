# Correct native release UI profiling

## Context

Native CPU sampling exposed React development in the audit release: WDIO's
`NODE_ENV=test` leaked into Vite. Ordinary app builds were already production.

## Changes

- Force production frontend only for the native release build child.
- Capture CPU earlier, retain profiles after metrics failure, record buffered
  task/frame timestamps and exclude hidden intervals and validation IPC.
- Reject Home error states; correct the historical GIMI UI certification claim.

## Impacted files

- `wdio.conf.ts`
- `tests/e2e/specs/startup-real-library-readonly.e2e.ts`
- `tests/e2e/support/nativeCpuProfile.ts`
- `docs/plans/startup-performance/scan-reuse-census-ui-20261011.md`
- `docs/plans/startup-performance/release-readonly-results-20261011.md`
- This history file.

## Goal and impact

Six native specs passed, each with four actual process launches. Full fixture
checksums, isolated config restoration and selected source identity passed.
Typecheck, scoped lint/format, failure/state contracts and focused review passed.
No application UI refactor or dependency was added; real GIMI was not bound.

## Limits

Ordinary runs still showed frame gaps up to 150 ms. Covered gaps were mostly V8
idle; early 54–122 ms tasks ended before CPU attachment. WebView2 startup flags
exported no trace. Windows WPR attribution needs an Administrator operator; full
smoothness and a new production-frontend GIMI certification remain open.
