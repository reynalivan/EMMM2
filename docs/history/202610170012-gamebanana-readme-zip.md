# Restore GameBanana README ZIP

## Context

The previous README-only GameBanana ZIP was missing from ignored `dist/` output.

## Changes

- `docs/gamebanana-upload-readme.txt` (added): durable English source with project and release links, install steps, features, file caution, and independence statement.
- `dist/gamebanana/README.txt` and `dist/gamebanana/EMMM2-GameBanana-README-only.zip` (generated, ignored): one-file upload artifact.

## Goal

The GameBanana upload can direct readers to EMMM without bundling an installer or mods.

## Impact

Documentation and local packaging only. No application behavior changes.
