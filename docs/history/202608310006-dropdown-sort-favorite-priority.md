# Dropdown Sort and Favorite Priority

## Context

The mod manager used a cycling sort button, requiring repeated clicks to reach a field and direction. Descending sort also reversed the favorite priority, so favorite mods could appear below regular mods.

## Changes

- Replaced the cycling sort control with a localized dropdown covering Name, modified date, and size in both directions.
- Kept container-first grouping within each priority tier.
- Made favorite mods a global first-priority tier that is preserved for ascending and descending sorts.
- Added focused coverage for all sort combinations and the dropdown interaction.

## Impacted Files

- `src/widgets/mod-explorer/components/FolderGridToolbar.tsx` (modified)
- `src/widgets/mod-explorer/FolderGrid.tsx` (modified)
- `src/widgets/mod-explorer/hooks/folderCache.ts` (modified)
- `src/widgets/mod-explorer/hooks/useFolderGrid.ts` (modified)
- `src/widgets/mod-explorer/hooks/useFolderGridNav.ts` (modified)
- `src/widgets/mod-explorer/components/FolderGridToolbar.test.tsx` (added)
- `src/widgets/mod-explorer/hooks/folderCache.test.ts` (added)
- `src/shared/i18n/locales/{en,id,zh}/grid.json` (modified)

## Goal

Make sorting discoverable and predictable while ensuring favorite mods remain visible at the top regardless of the selected sort option.

## Impact

Sorting remains client-side and uses the existing persisted sort field and direction. No backend or data model changes were made.

## Notes

Validation completed with 19 focused tests, TypeScript, ESLint, Prettier, i18n lint, and production build.
