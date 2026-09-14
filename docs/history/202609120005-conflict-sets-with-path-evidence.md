# Conflict sets with path evidence

## Context

The enabled-mod conflict dialog showed one card per runtime resource or shader
hash. Repeated mod names hid the fact that the participating folders could be
in different locations.

## Changes

- Grouped conflict cards by the participating mod paths, while retaining every
  resource or shader hash as expandable runtime evidence.
- Displayed the complete mod-folder path beside each action and the complete
  INI source path beneath each runtime hash.
- Updated resolution counts to describe conflict sets rather than individual
  runtime hashes; a keep decision applies to every hash in the same set.
- Added EN, ID, and ZH labels for runtime-key and mod-location counts.

## Validation

- `corepack pnpm exec vitest run src/features/conflict-report/conflictResolution.test.ts src/features/conflict-report/ConflictModal.test.tsx` — 15 passed
- `corepack pnpm exec prettier --check` on changed files
- `corepack pnpm build`
- `corepack pnpm tauri build --bundles nsis`
- `corepack pnpm i18n:lint` remains blocked by the three unrelated existing
  hardcoded strings in Match Wizard.
