# Install hotkey feedback locally

## Context

The user authorized updating installed EMMM with the new notices, then deferred
direct game validation in favor of alternative checks.

## Changes

- Coordinate source freeze and compiler/native handoff with startup and cleanup.
- Back up and hash-verify the installation and production app data.
- Build normal production without bundles/audit features; atomically replace only
  installed `emmm.exe`, retaining its predecessor for rollback.
- Preserve all 169 other files, settings, DB and seven matching shipped resources.

## Impacted files

- `C:/Users/yusri/AppData/Local/EMMM/emmm.exe` (replaced).
- `docs/plans/hotkey-feedback/{implementation_plan.md,validation.md}`; this history.
- Ignored `.audit-output/hotkey-installed-update-20261011/` contains the scoped
  swap script, manifests, receipts, build log and source/binary verification.
- Backup: `C:/Users/yusri/AppData/Local/EMMM-backups/20261011-notice-a4665a37-0c64-4eaa-a6dd-756ff7b2cb87/`.

## Goal and impact

Fresh production build and installed SHA-256 match
`521a04c3f577efaa88bc351d79f0841344824a555f3251557acb547ce8a78077`.
1577 frozen inputs unchanged. Follow-up frontend 43 tests, Node 3 contracts and
actual bootstrap guard PASS. Coordinated private release native suite 15/15 PASS.
Production was not launched or reset; all owned native/compiler processes exited.
Version remains 0.1.0, NotSigned; this is not a public signed release.
Five existing library warnings remain. Direct game rendering, click-through,
multi-monitor/DPI visuals and exclusive fullscreen remain unverified by request.
No actual-library mutation, new dependency, commit or push.
