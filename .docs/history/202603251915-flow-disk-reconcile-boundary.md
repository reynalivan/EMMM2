# Flow Clarification for Disk Reconcile vs Deep Match Scanner

## Context

The main flow document still lacked one explicit section that clearly separated runtime filesystem reconciliation from explicit canonical matching.

## Changes

- Added a dedicated domain-boundary section to `flow.md`.
- Defined the purpose, triggers, and hard rules for Disk Reconcile.
- Defined the purpose, triggers, and hard rules for Deep Match Scanner.
- Split runtime refresh/file watcher flow from explicit Deep Match Scanner flow.
- Added a practical decision rule to help future AI changes choose the correct domain.

## Impacted Files

- `.docs/flow.md` (modified)

## Goal

Make the main architecture flow document explicitly teach the difference between “sync what is true on disk” and “decide which canonical object a folder belongs to”.

## Impact

- Future implementation work has a clearer source-of-truth boundary.
- Watcher/runtime refresh logic is documented separately from explicit matching/import logic.
- No runtime behavior changed; this is architecture clarification only.
