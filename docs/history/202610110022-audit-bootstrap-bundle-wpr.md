# Audit bootstrap bundle and prepare WPR

## Context

Investigate eager charts and native frame gaps without racing another chat's
physical checks or installed-app update.

## Changes

- Add a write-free bootstrap dependency guard; existing manual chunks fail with
  195 eager Recharts modules. Production Vite configuration is unchanged.
- Compare private bundle variants twice. Automatic splitting reduces initial JS
  1,838,606 → 1,423,433 bytes with charts retained lazily; priority-only grouping
  is neutral and discarded. Matched comparison has no selected-input drift.
- Prepare official WebView2 profiles and a temporary operator-gated WPR adapter.
  Keep launcher state together, use token-qualified acknowledgements and request
  stop before terminal restoration. Restoration still runs after stop failure.

## Impacted files

- `scripts/check-bootstrap-bundle.mjs` (added)
- `docs/plans/startup-performance/bootstrap-bundle-wpr-20261011.md` (added)
- This history; ignored graphs/profiles/adapter/contracts in
  `.audit-output/bootstrap-bundle-20261011/`.

## Goal and impact

Identify the smallest chart-splitting candidate and prepare native attribution.
WPR adapter regression contract RED → GREEN, TypeScript, scoped lint/format and
focused review pass. Shared dist/native binaries, dependencies, installed app
and real GIMI files were untouched.

## Limits

No startup speed gain or WPR/WPA attribution is claimed. Native comparison waits
for the other chat's source/native handoff; privileged recording awaits the
human Administrator operator. Bootstrap guard remains intentionally RED until
the candidate is applied to production configuration. GPU/DWM supplementary
profiles are available but their recording combination remains untested.
