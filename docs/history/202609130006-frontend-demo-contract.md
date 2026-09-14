# Frontend demo contract

## Context

The project needed durable instructions for reviewing the real frontend with browser-safe fixture data.

## Changes

- Added the demo boundary, native safety rules, and browser verification requirements to the agent guide and agent rules.
- Added the fixture locations, update flow, production isolation, and UI review checklist to `docs/frontend-demo.md`.

## Impacted Files

- `AGENT.md` (modified)
- `.agent/rules/ui_ux.md` (modified)
- `.agent/rules/dev_ops.md` (modified)
- `docs/frontend-demo.md` (added)
- `docs/history/202609130006-frontend-demo-contract.md` (added)

## Goal

Agents can update demo content and verify the existing UI without creating duplicate frontend surfaces or touching real data.

## Impact

The changes add documentation and operating rules only. Runtime behavior is unchanged.
