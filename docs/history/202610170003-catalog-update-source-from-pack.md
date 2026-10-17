# Read catalog update source from installed pack

## Context

The catalog updater embedded a repository endpoint even though catalog packs already declare their source.

## Changes

- Backend catalog pack/import/update code: derive the GitHub update repository from the installed `manifest.json`, validate its URL, and require signed replacements to keep the same pack ID and repository.
- Settings UI, tests, and translations: allow update checks after a pack is installed and show invalid-source errors.
- `docs/catalog-asset-pack.md` and `src-tauri/AGENT.md`: document the new update flow.

## Goal

Catalog update URLs follow the installed ZIP rather than a repository hardcoded in EMMM.

## Impact

Manual update checks need an installed pack with a valid public GitHub source. The existing pinned signature and atomic replacement remain in force.
