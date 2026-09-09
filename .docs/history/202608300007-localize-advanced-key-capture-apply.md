# Localize Advanced Key Capture Apply Action

## Context

Advanced Key Capture rendered `common:actions.apply` as a raw key because the common locale did not define the action.

## Changes

- Added the Apply translation to the EN, ID, and ZH common action dictionaries.
- Added modal regression coverage for the rendered action and all locale values.

## Impacted Files

- `src/shared/i18n/locales/en/common.json` (modified)
- `src/shared/i18n/locales/id/common.json` (modified)
- `src/shared/i18n/locales/zh/common.json` (modified)
- `src/widgets/mod-preview/components/AdvancedKeybindModal.test.tsx` (added)

## Goal

Advanced Key Capture shows the translated Apply action instead of the raw i18n key.

## Impact

No behavior or data-flow changes; only localized modal copy is added.
