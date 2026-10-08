# Isolated client smoke audit

## Context

User requested client E2E positive, negative and edge cases, including actions, and approved disposable isolated QA fixtures. Product changes were not authorized for this audit.

## Changes

- Added four native UI smoke specs and explicit coverage/evidence reports.
- Verified rapid latest-wins and small bulk switching on disk; preserved failing regressions and blocked substeps.
- Confirmed stale Settings revisions, duplicate-path validation bypass, dialog Escape failure and address-entry focus loss; recorded unresolved preview/selection failures separately.

## Impacted Files

- Added: `tests/e2e/specs/client-settings-smoke.e2e.ts`, `client-workspace-smoke.e2e.ts`, `client-secondary-smoke.e2e.ts`, `client-browser-smoke.e2e.ts`.
- Added: `docs/plans/client-smoke-20261007/plan.md`, `report.md`, `demo-actions.md`, `execution-summary.json`, `settings-focused-evidence.json`.
- Added: this history entry. Ignored screenshot/DOM artifacts: `logs/client-smoke-20261007/`.

## Goal

Evidence-backed client audit without using the production library/database or claiming exhaustive green coverage.

## Impact

No application implementation, dependency or release change from this audit. Existing concurrent edits preserved. Typecheck and focused ESLint passed; native smoke failures remain. Driver disconnections prevented final coverage. Owned temp roots/tabs/server cleaned; private E2E cache and recoverable QA Recycle Bin residues explicitly retained.
