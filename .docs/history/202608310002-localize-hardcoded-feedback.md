# Localize Remaining Hardcoded Feedback

## Context

The repository i18n lint found 14 hardcoded user-facing strings in onboarding, archive extraction feedback, and folder conflict resolution.

## Changes

- Localized onboarding language selector labels through the onboarding namespace.
- Localized extraction progress and archive password dialog copy through match_wizard.
- Localized folder conflict instructions through folder_grid.
- Updated the import wizard host test mock and assertions for the new translation keys.

## Impacted Files

- `src/pages/onboarding/WelcomeScreen.tsx` (modified)
- `src/features/import-batches/ImportBatchAnalysisFeedback.tsx` (modified)
- `src/features/import-batches/ImportBatchWizardHost.test.tsx` (modified)
- `src/widgets/mod-explorer/modals/FolderConflictManager.tsx` (modified)
- `src/shared/i18n/locales/{en,id,zh}/onboarding.json` (modified)
- `src/shared/i18n/locales/{en,id,zh}/match_wizard.json` (modified)
- `src/shared/i18n/locales/{en,id,zh}/folder_grid.json` (modified)

## Goal

Remove all repository-reported hardcoded strings while keeping user-facing copy translated in EN, ID, and ZH.

## Impact

No backend or data-flow changes. The repository i18n lint now passes. Existing unrelated worktree changes were preserved.

## Notes

Targeted tests may continue to emit pre-existing React act warnings in `ImportBatchWizardHost.test.tsx`; they do not fail the test run.
