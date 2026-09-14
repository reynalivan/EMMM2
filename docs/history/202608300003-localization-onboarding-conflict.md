# Localize Onboarding and Conflict Manager

## Context
The onboarding screen's new indexing UI and the previously redesigned Folder Conflict Manager UI contained several hardcoded English strings. The user requested to use the `i18next` localization system instead and also asked for a language selector directly on the initial Welcome Screen.

## Changes
- Extracted all hardcoded indexing phases, ETA texts, and status strings into `onboarding.json` (EN, ID, ZH).
- Extracted all hardcoded Folder Conflict candidate card details (e.g., "Folder Info", "Created", "Contents", "Move to Trash") into `folder_grid.json` (EN, ID, ZH).
- Updated `WelcomeScreen.tsx` to use `t()` for all texts.
- Updated `FolderConflictCandidateCard.tsx` to use `t()` for all tooltip and button texts.
- Added a floating Globe icon at the top-right corner of the Welcome Screen to allow language switching (`en`, `id`, `zh`) before doing anything else.

## Impacted Files
- `src/shared/i18n/locales/*/onboarding.json` (modified)
- `src/shared/i18n/locales/*/folder_grid.json` (modified)
- `src/pages/onboarding/WelcomeScreen.tsx` (modified)
- `src/widgets/mod-explorer/modals/FolderConflictCandidateCard.tsx` (modified)

## Goal
Ensure the entire application remains fully translatable and give new users the ability to switch languages before navigating deeper into the app.

## Impact
No breaking changes. Enhanced accessibility for non-English speakers.
