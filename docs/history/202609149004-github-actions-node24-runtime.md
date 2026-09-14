# GitHub Actions Node 24 Runtime

## Context

GitHub Actions deprecated the Node 20 runtime used by the workflow actions.

## Changes

- Upgraded checkout, Node setup, and pnpm setup actions from v4 to v5 in CI and release workflows.
- Updated the RustSec audit's pinned checkout revision from v4.2.2 to v5.0.1.

## Impacted Files

- `.github/workflows/ci.yml` (modified)
- `.github/workflows/release.yml` (modified)
- `.github/workflows/security-audit.yml` (modified)
- `docs/history/202609149004-github-actions-node24-runtime.md` (added)

## Goal

The repository workflows now use action versions that run on Node 24.

## Impact

Workflow steps and the project's Node 22 runtime remain unchanged. GitHub-hosted runners meet the v5 actions' Node 24 runner requirement.
