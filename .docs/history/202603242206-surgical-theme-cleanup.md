# Surgical UI Theme Cleanup

## Context

Final audit revealed minor remaining gaps in modal backdrops and static color literals following the main theme migration. This change ensures 100% consistency and theme-awareness across all secondary UI components.

## Changes

- **Standardized Backdrops**: Unified all remaining modal overlays to use `bg-overlay-mask` and `backdrop-blur-sm`.
- **Semantic Rarity Color**: Migrated static `amber-400` stars to the theme-aware `text-warning` token.
- **Border Consolidation**: Standardized muted borders to `border-base-content/10` for consistent contrast in Light mode.

## Impacted Files

- `src/features/settings/tabs/MaintenanceTab.tsx` (modified)
- `src/features/folder-grid/MoveToObjectDialog.tsx` (modified)
- `src/features/folder-grid/ConflictResolveDialog.tsx` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/object-list/FilterPanel.tsx` (modified)

## Goal

Achieve a perfectly standardized, theme-compliant user interface across the entire application lifecycle, from main feature views to utility modals.

## Impact

- **Consistency**: High-depth surfaces (modals) now share a unified appearance across all themes.
- **Accessibility**: Improved contrast for rarity stars and chip borders in custom light themes.
- **Maintainability**: Centralized backdrop and rarity styling under semantic tokens.

## Notes

- `text-warning` was chosen for rarity stars as its palette matches the existing gold/amber expectation while allowing theme-specific adaptation.
