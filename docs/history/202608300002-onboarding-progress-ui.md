# Onboarding Indexing UI Redesign

## Context
The initial indexing progress UI during onboarding was using a standard HTML `<progress>` bar which felt a bit generic and lacked detailed feedback on what the backend scanner was actually doing. The user requested a cleaner, more informative layout.

## Changes
- Replaced the native `<progress>` element with a custom Tailwind CSS animated bar with a pulse effect.
- Added a `PhaseLabel` (e.g., "1/4 Discovering Folders") derived directly from the backend's `diskProgress.phase`.
- Added a granular unit counter (e.g., `1,500 / 10,000`) instead of just a raw percentage.
- Added a "Mini Terminal" log box that scrolls the `current_root` path to visually indicate that files are rapidly being processed.
- Improved the loading spinner layout with a pulsing background ring.
- Refined ETA display for the final steps ("Finishing up...").

## Impacted Files
- `src/pages/onboarding/WelcomeScreen.tsx` (modified)

## Goal
To make the initial disk scanning and indexing phase feel significantly faster, more transparent, and visually premium for first-time users.

## Impact
- Better perceived performance due to rapid visual feedback in the terminal box.
- No breaking changes; purely presentational improvements using existing `diskProgress` state.
