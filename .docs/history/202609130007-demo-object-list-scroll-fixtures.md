# Demo object list scroll fixtures

## Context

Browser review needed enough object rows to exercise the existing virtualized object list and its scrolling behavior.

## Changes

- Added typed in-memory object fixtures across Character, Environment, and UI categories.
- Added long names plus enabled, disabled, safe, and unsafe object states for list review.
- Updated the demo workspace test to protect the fixture volume and category distribution.

## Impacted Files

- `src/demo/workspace.ts` (modified)
- `src/demo/workspace.test.ts` (modified)
- `.docs/history/202609130007-demo-object-list-scroll-fixtures.md` (added)

## Goal

The existing ObjectList can be reviewed with 80 objects and a real scroll range.

## Impact

The fixtures remain development-only and in memory. Production behavior is unchanged.
