# Global onboarding indexing progress

## Context

The onboarding progress bar showed the active game's root count, which made a local percentage appear to represent the full multi-game indexing run. The raw current-root value also looked like terminal output rather than a user-facing status.

## Changes

- Added a pure global-progress calculation that combines completed games with the active game's discovery, scan, projection, and finalization stages.
- Replaced the dense phase/stat/terminal card with a single global progress bar, ETA, game position, and human-readable active-folder status.
- Added localized copy for the new global status in English, Indonesian, and Chinese.
- Added regression coverage for global percentage calculation and the onboarding display.

## Impacted files

- `src/pages/onboarding/WelcomeScreen.tsx`
- `src/pages/onboarding/WelcomeScreen.test.tsx`
- `src/pages/onboarding/utils/indexingProgress.ts`
- `src/pages/onboarding/utils/indexingProgress.test.ts`
- `src/shared/i18n/locales/{en,id,zh}/onboarding.json`

## Goal

Make indexing progress truthful at the onboarding scope while keeping the active scan context concise and readable.
