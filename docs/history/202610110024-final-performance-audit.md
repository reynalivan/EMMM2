# Audit remaining performance and progress risks

## Context

Final review of startup, onboarding, normal sync and runtime efficiency, with
complementary skills and measured changes preferred over further architecture work.

## Changes

- Record two confirmed progress issues and their smallest proposed corrections.
- Reject terminal tombstones: authority retries can legitimately reuse a run ID.
- Define benchmark gates for per-mod harvest dispatch and onboarding foreground wait.
- Separate historical speedups/native evidence from current, still-changing source.
- No production code, configuration, dependencies or real-library files changed.

## Impacted files

- `docs/plans/startup-performance/final-performance-audit-20261011.md` (added).
- This history (added).
- Ignored evidence in `.audit-output/final-performance-audit-20261011/`: replay,
  isolated test config, source hashes and JSON results.

## Verification

Existing progress/onboarding controls: 22 PASS. Same-ID retry control: 1 PASS.
Four adversarial replays are RED; two findings are production-reachable, two remain
unproven. Corrected isolated-config aliases restore 20 control tests after an
initial harness-only failure. Five audited source hashes match. Independent
read-only review challenged the terminal-order assumptions.

## Goal and impact

Provide a small, reviewable follow-up plan. No new throughput gain is claimed.
New backend experiments and final native release validation await a stable combined
source/build slot; WPR still needs an Administrator operator. GIMI was not accessed.
