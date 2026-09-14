# GameBanana Discover provenance enrichment

## Context

Discover downloads reach the active game's Mod Inbox directly, but their signed asset URLs did not retain a trustworthy link to the GameBanana submission page.

## Changes

- Record only a canonical GameBanana page URL, item type, and item ID when the browser's current page is an HTTP(S) `gamebanana.com` submission URL; query strings and fragments are discarded.
- Preserve that provenance with the browser download and look it up only when the Mod Inbox source is a finished download with the same byte length and SHA-256 content hash. Files over the bounded hash size are treated as ordinary, non-enriched imports.
- Fetch and persist optional GameBanana source metadata as evidence, without contributing to matching scores or changing the selected destination.
- Bound each enrichment HTTP request to two seconds. Missing provenance, invalid pages, API failures, worker errors, and timeouts retain normal local import behavior; unavailable results are cached as non-blocking evidence.
- Expose persisted source metadata in the import-item binding and cover verified URL parsing, metadata persistence, and frontend test fixtures.

## Impacted Files

- `src-tauri/migrations/20260914000002_gamebanana_import_enrichment.sql` (added)
- `src-tauri/src/modules/browser/` download provenance and SQLite persistence (modified)
- `src-tauri/src/modules/ingestion/` analysis, persistence, and source-metadata enrichment (modified)
- `src-tauri/src/modules/matching/` verified URL parsing and optional API facade (modified)
- `src/shared/api/tauri/bindings.gen.ts` and import test fixtures (modified)

## Goal

Use a verified Discover source as optional provenance for Mod Inbox imports while keeping local analysis and user decisions authoritative.
