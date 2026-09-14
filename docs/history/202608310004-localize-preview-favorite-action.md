# Localize Preview Favorite Action

## Context

The Preview Panel context menu displayed the raw translation key `context.favorite` instead of the localized Favorite action label.

## Changes

- Added the missing `context.favorite` and `context.unfavorite` translations to the shared grid namespace used by the Preview Panel context-menu policy.
- Added a focused regression assertion so favorite labels cannot silently fall back to a raw key.

## Impacted Files

- `src/features/mod-runtime/actions/modContextMenuPolicy.test.ts` (added)
- `src/shared/i18n/locales/{en,id,zh}/grid.json` (modified)

## Goal

Ensure the Preview Panel context menu displays a translated Favorite action in every supported language through the shared policy namespace.

## Impact

No behavior or backend changes. Only the user-facing label and locale coverage were updated.

## Notes

Validation should include the focused context-menu test and the repository i18n lint.
