# Greenfield Phase 2 (Steps 2.7–2.8): SafeMode Toggle + Cleanup Catalog

## Context

Final steps of Phase 2: v2 SafeMode toggle hook and legacy cleanup documentation.

## Changes

- **useV2SafeModeToggle** (`hooks/v2/useV2SafeModeToggle.ts`): ~90 lines replacing 209-line `useSafeModeToggle`. Composes `useCorridorSwitch` + `useV2HasPin`. Eliminates manual invoke/cache, `normalizeSwitchWarnings`, `prepareSwitchPreview`
- **V2 barrel export** updated with `useV2SafeModeToggle`
- **Cleanup catalog** (`.docs/v2-migration-cleanup.md`): Documents 15+ legacy files, their v2 replacements, and active callers blocking deletion

## Impacted Files

- `src/features/collections/hooks/v2/useV2SafeModeToggle.ts` (added)
- `src/features/collections/hooks/v2/index.ts` (modified — added export)
- `.docs/v2-migration-cleanup.md` (added)

## Goal

Complete Phase 2 frontend layer — all v2 hooks and components in place.

## Impact

- No breaking changes — new hook coexists with legacy `useSafeModeToggle`
- `npx tsc --noEmit` passes with zero errors
- Cleanup catalog provides clear migration path for Phase 3
