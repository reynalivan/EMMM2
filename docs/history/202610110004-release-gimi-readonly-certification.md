# Certify release GIMI startup without library writes

## Context

The user authorized native release measurement on real GIMI data without any
library mutation, Windows restart or cache eviction.

## Changes

Add a default-off release audit feature and native release profile; retain
deny-by-default writes, installed-app interlock and private config restoration.
Measure the initial native page and three real process restarts. Correct stale
NTFS manifest metadata with existing-handle reads and verify full checksums.

## Impacted Files

Backend (modified):

- `src-tauri/Cargo.toml`
- `src-tauri/src/modules/system/application/app/readonly_audit.rs`
- `src-tauri/src/modules/collections/api.rs`, `duplicates/api.rs`, `games/api.rs`

Native verification:

- `wdio.conf.ts`, `wdio.readonly-audit.conf.ts` (modified)
- `wdio.readonly-release-audit.conf.ts` (added)
- `tests/e2e/support/readonlyAudit.ts` (modified)
- `tests/e2e/specs/startup-real-library-readonly.e2e.ts` (modified)
- `scripts/audit-library-manifest.ps1` (modified)

Documentation:

- `src-tauri/AGENT.md`, `.agent/skills/e2e-automation/SKILL.md` (modified)
- `docs/plans/startup-performance/release-readonly-certification-20261010.md` (added)
- `docs/plans/startup-performance/release-readonly-results-20261011.md` (added)
- This history (added)
- Ignored `.audit-output/gimi-release-certification-20261010/`: fixture, snapshot,
  workload and comparison helpers; logs, manifests, validated executable and reports.

## Goal and Impact

632 GB / 375,100 files: exact Before/After/Verify digest, private config restored,
1,526 build inputs unchanged. Core readiness: first index 9.103 s, restart median
2.677 s; restart backend throughput 2,832 mods/s. No production speed refactor,
schema/dependency change or protected-library repair. Census and initial
classification remain dominant; UI long tasks remain measurable.

## Verification and Limits

Final fixture and real native release runs passed; full integrity checks passed.
Guard cases passed 7/7 in release feature and ordinary release cfg; the latter
used command-only opt0 after opt3 compiler OOM. Typecheck, scoped lint/format and
strict all-target Clippy passed. Strict release-profile Clippy is not certified.
OS cache was uncontrolled, Genshin absent during measurement, runtime publication
excluded. See the execution report for timings, evidence and residual risks.
