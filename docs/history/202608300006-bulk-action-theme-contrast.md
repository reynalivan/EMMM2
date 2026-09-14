# Improve Bulk Action Theme Contrast and Localization

## Context

Bulk status actions used success/warning text directly on the primary action-bar surface, making them hard to read across themes. The grid variant also did not provide a translated label for its More actions button.

## Changes

- Switched Safe/Unsafe and Enable/Disable controls to semantic status button surfaces with matching content tokens.
- Made the shared More actions label required and supplied its EN/ID/ZH translations for the grid bulk bar.
- Added a regression test covering status contrast classes and the More actions label.

## Impacted Files

- `src/shared/ui/components/ui/BulkActionBar.tsx` (modified)
- `src/shared/ui/components/ui/BulkActionBar.test.tsx` (added)
- `src/widgets/mod-explorer/components/BulkActionBar.tsx` (modified)
- `src/shared/i18n/locales/en/grid.json` (modified)
- `src/shared/i18n/locales/id/grid.json` (modified)
- `src/shared/i18n/locales/zh/grid.json` (modified)

## Goal

Bulk action status controls remain readable in Onyx and Light themes, and all visible/accessibility labels are provided through the localization layer.

## Impact

No behavior or data-flow changes. Bulk actions remain available with clearer status surfaces and complete grid-bar localization.
