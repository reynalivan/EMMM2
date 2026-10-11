# Remove eager bootstrap charts

## Context

Apply the measured bundle simplification after the other chat's native/install
source freeze was released.

## Changes

- Remove only the 34-line `manualChunks` grouping; use default bundle splitting.
- Bootstrap guard RED → GREEN: charts leave the initial static closure and remain
  emitted lazily. No new lazy state, dependencies or application code.
- Record final release native validation and remaining profiling limits.

## Impacted files

- `vite.config.ts` (modified)
- `scripts/check-bootstrap-bundle.mjs` (added in preceding audit phase)
- `docs/plans/startup-performance/bootstrap-bundle-wpr-20261011.md` (updated)
- This history; ignored final artifacts in
  `.audit-output/bootstrap-bundle-20261011/native-final/`.

## Verification

App/config TypeScript, scoped lint/format and focused review pass. Fresh private
read-only release spec passes four process launches: 148 objects/4,243 mods,
one watcher/StartupBoot, no Home error. Full Before/After/Verify fixture integrity,
config restoration/zero bindings, 1,529 selected inputs/no drift and owned teardown
pass. Installed executable unchanged; real GIMI untouched. Build keeps five known
unused Rust warnings. Source/native slot returned to the hotkey chat.

## Goal and impact

Matched bundle experiments reduce initial JS 1,838,606 → 1,423,433 bytes (22.6%)
with charts retained lazily. No matched native speed gain is claimed. Frame gaps
reach 146.7 ms and initial buffered long task 236 ms; WPR attribution still awaits
an Administrator operator. This native run covers Home/initial isolated indexing,
not Finish Setup or chart interaction. No installation, commit or dependency change.
