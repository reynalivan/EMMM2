# Simplify Match Wizard UI

## Context

The Match Wizard presented one item in a wide seven-column table, kept the modal at a fixed viewport height, and exposed several secondary actions beside the primary decision controls.

## Changes

- Replaced the dense table rows with responsive item cards grouped into match, destination, and outcome panels.
- Reduced the workflow indicator to review, resolve, and commit.
- Limited bulk shortcuts to multi-item batches and moved retry/open actions into a More menu.
- Added translated labels for the new UI in EN, ID, and ZH.

## Impacted Files

- `src/features/match-wizard/ImportBatchWizard.tsx` (modified)
- `src/features/match-wizard/components/ImportBatchWizardItemRow.tsx` (modified)
- `src/features/match-wizard/components/ImportBatchWizardCategoryPanel.tsx` (added)
- `src/features/match-wizard/components/ImportBatchWizardDestinationPanel.tsx` (added)
- `src/features/match-wizard/components/ImportBatchWizardOutcomePanel.tsx` (added)
- `src/features/match-wizard/ImportBatchWizard.test.tsx` (modified)
- `src/shared/i18n/locales/{en,id,zh}/match_wizard.json` (modified)

## Goal

Make the Match Wizard easier to scan and operate while preserving all existing classification, destination, recovery, and commit actions.

## Impact

Visual and interaction-layer change only; no backend contract or import decision behavior changed.
