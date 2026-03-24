# Final i18n Audit & Localization Completion

## Context

Standardize all remaining hardcoded strings identified during the final audit to ensure 100% internationalization coverage for EMM2.

## Changes

- **Localize Settings**: Refactored `GameFormModal.tsx` (labels, placeholders, validation).
- **Localize Scanner**: Refactored `ArchiveModal.tsx` (overwrite confirmations, progress/error messages).
- **Localize Preview**: Refactored `MetadataSection.tsx`/`IniEditorSection.tsx` (status labels, tooltips).
- **Unified Locales**: Updated `settings.json`, `scanner.json`, and `preview.json` across `en`, `id`, and `zh` folders.
- **Bugfixes**: Restored missing imports and fixed type safety in refactored components.

## Impacted Files

- `src/features/settings/modals/GameFormModal.tsx` (modified)
- `src/features/scanner/components/ArchiveModal.tsx` (modified)
- `src/features/preview/components/MetadataSection.tsx` (modified)
- `src/features/preview/components/IniEditorSection.tsx` (modified)
- `src/locales/{en,id,zh}/{settings,scanner,preview}.json` (modified)

## Goal

Achieve absolute 100% i18n coverage across all user-facing UI elements.

## Impact

No functional side effects; improved accessibility and consistency for multi-language users.
