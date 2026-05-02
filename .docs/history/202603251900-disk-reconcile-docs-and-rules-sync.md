# Disk Reconcile Docs and Rules Sync

## Context

Public API and backend folders were already split between Disk Reconcile and Deep Match Scanner, but the active requirements, TRD, intro, flow notes, and agent rules still mixed the two domains and still referenced legacy indexing guidance.

## Changes

- Updated core requirements to distinguish Disk Reconcile from Deep Match Scanner.
- Renamed active requirement terminology from Deep Matcher to Deep Match Scanner where the docs describe explicit matching/import behavior.
- Added domain-boundary notes so ObjectList and runtime freshness are documented as Disk Reconcile responsibilities.
- Updated TRD, intro, and flow docs to reflect watcher/focus/mods-entry using Disk Reconcile and explicit scan/import using Deep Match Scanner.
- Replaced stale `quickImport` / `scanPreview` / `syncDatabase` guidance in agent rules with `reconcileDiskState`, `runDeepmatchPreview`, and `runDeepmatchScanner`.

## Impacted Files

- `.docs/requirements/req-07-object-list.md` (modified)
- `.docs/requirements/req-09-object-schema.md` (modified)
- `.docs/requirements/req-10-object-crud.md` (modified)
- `.docs/requirements/req-23-mod-import.md` (modified)
- `.docs/requirements/req-25-scan-engine.md` (modified)
- `.docs/requirements/req-26-deep-matcher.md` (modified)
- `.docs/requirements/req-27-sync-database.md` (modified)
- `.docs/requirements/req-38-auto-organizer.md` (modified)
- `.docs/requirements/req-44-discover-hub-smart-import.md` (modified)
- `.docs/trd.md` (modified)
- `.docs/intro.md` (modified)
- `.docs/flow.md` (modified)
- `.agent/rules/project_core.md` (modified)
- `.agent/rules/data_logic.md` (modified)

## Goal

The active documentation and agent guidance now describe Disk Reconcile as the runtime filesystem-truth domain and Deep Match Scanner as the explicit canonical matching/import domain.

## Impact

- Future AI/code changes now have a clearer documented boundary between runtime sync and explicit matching.
- Agent rules no longer recommend removed flows like `quickImport`.
- No product behavior changed; this is documentation and rule alignment only.

## Notes

- `.agent/workflow` does not exist in this repo, so no workflow file was updated there.
- `cargo check` and `pnpm exec tsc --noEmit` still pass after the documentation/rule sync.
