# Simplify Match Wizard Actions

## Context

The redesigned Match Wizard still repeated item information, added workflow chrome without navigation, and used inconsistent confidence sources for bulk actions.

## Changes

- Replaced the non-interactive workflow stepper with compact batch status badges.
- Made planned-name editing inline and removed the duplicate name form.
- Removed duplicate confidence/outcome content and only render details when useful.
- Unified bulk accept/skip eligibility around the primary destination suggestion, added counts, and disabled bulk actions while an item update is active.
- Limited retry suggestions to actionable retry states and improved import-oriented labels across EN, ID, and ZH.
- Added regression coverage for inline rename and confidence-tier edge cases.

## Impacted Files

- `src/features/match-wizard/ImportBatchWizard.tsx` (modified)
- `src/features/match-wizard/components/ImportBatchWizardItemRow.tsx` (modified)
- `src/features/match-wizard/components/ImportBatchWizardOutcomePanel.tsx` (modified)
- `src/features/match-wizard/ImportBatchWizard.test.tsx` (modified)
- `src/shared/i18n/locales/en/match_wizard.json` (modified)
- `src/shared/i18n/locales/id/match_wizard.json` (modified)
- `src/shared/i18n/locales/zh/match_wizard.json` (modified)

## Goal

Make the Match Wizard easier to scan, less repetitive, and safer to operate for both single-item and bulk imports.

## Impact

No backend contract changes. Bulk shortcuts now classify items consistently from their primary destination suggestion; items without a suggestion remain in the review group. Existing unrelated worktree changes were preserved.

## Notes

Focused tests, ESLint, TypeScript, Prettier, and production build pass. Repository-wide i18n lint still reports pre-existing hardcoded strings outside Match Wizard.
