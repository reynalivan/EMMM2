# Storage Optimizer safety fixes

## Changes

- Bind duplicate scans to the configured, canonical Mods root for the requested game.
- Fail a scan when its folder walk, metadata reads, INI reads, or file hashing cannot complete.
- Store the root used by each completed report and hide reports when the game's configured root changes.
- Keep the global Stop action available after switching games and stop the scan's owning game.

## Data and compatibility

The additive `dedup_jobs.root_path` migration preserves existing rows. Reports created before the migration have no root proof and remain hidden until a new scan completes.

## Validation

- Targeted Storage Optimizer UI tests passed (2/2).
- Rust formatting check passed.
- Frontend production build passed after the targeted fix.
- Rust targeted tests passed in SQLx offline mode: scan-root validation, unreadable candidate handling, vanished-file hashing, and stale report rejection.
