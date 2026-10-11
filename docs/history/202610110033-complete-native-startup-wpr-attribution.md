# Complete native startup and WPR attribution

## Context

Lossy traces prevented frame analysis; restart readiness also accepted Tauri's
bridge before the local document loaded.

## Changes

- Require the local document origin and IPC bridge; save each completed launch.
- Bound capture merge acknowledgements to ten minutes with config restoration.
- Filter browser ETW providers to WebView2, use light GPU/composition profiles,
  and retain failed captures separately.
- Export WPA tables and integrate scheduling/V8 events for one 133.2 ms frame gap.

## Impacted files

- `tests/e2e/specs/startup-real-library-readonly.e2e.ts`.
- `docs/plans/startup-performance/final-performance-audit-20261011.md`; this history.
- Ignored `.audit-output/final-performance-audit-20261011/`:
  `run-wpr-operator.ps1`, `WebView2_Startup.wprp`, `snapshot-inputs.mjs`,
  `test-startup-readiness.mjs`, `prepare-wpa-export.ps1`,
  `Native-CPU-GPU.wpaprofile`, `GPU-DesktopComposition-Light.wprp`,
  `analyze-frame-thread-states.py`, `analyze-webview-spans.py`, and capture evidence.
- Ignored `.audit-output/bootstrap-bundle-20261011/`:
  `wdio.wpr-readonly.conf.ts`, `test-wpr-gates.mjs`.

## Verification

Readiness regression RED→GREEN; timeout/restore contracts, E2E/harness types,
scoped lint/format and profile validation PASS. Four native launches passed both
without WPR and in the final capture: zero lost events/buffers, 1,578 unchanged
inputs. Fixture manifests matched; config restored; installed executable/DB
unchanged; owned processes stopped. Tested binary preserved in capture artifacts.

## Goal and impact

Indexed fixture core-ready observations: 1.14–1.17 seconds without WPR. During the
sampled 133.2 ms gap the renderer ran 119.167 ms and was Ready 6.394 ms; V8 callbacks
overlap it. Sparse CPU samples prevent function-level or React attribution.
No production optimization, new dependency, install or real-GIMI mutation here.
Shared source/build/native ownership returned to the cleanup task.
