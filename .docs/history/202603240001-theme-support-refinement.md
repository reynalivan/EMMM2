# Theme Support Refinement

## Context

The application supported multi-theme but lacked high-contrast consistency in light mode and had hardcoded colors in key components like the Top Bar and Onboarding pages.

## Changes

- **Refined Theme Definitions:** Updated `App.css` with matured `onyx` (dark) and new `light` themes using semantic CSS variables.
- **Dynamic Styling:** Refactored `TopBar.tsx`, `WelcomeScreen.tsx`, and `AuroraBackground.tsx` to use theme-aware variables instead of literals.
- **Simplified Registry:** Streamlined `themeOptions.ts` to focus on Onyx and Light schemas, setting Onyx as the default.
- **Scrollbar & Glass Effects:** Globally standardized these effects to adapt based on the active theme.

## Impacted Files

- `src/App.css` (modified)
- `src/components/layout/top-bar/index.tsx` (modified)
- `src/features/onboarding/WelcomeScreen.tsx` (modified)
- `src/features/welcome/AuroraBackground.tsx` (modified)
- `src/features/settings/theme/themeOptions.ts` (modified)

## Goal

Establish a professional, high-contrast, and aesthetic multi-theme system that maintains a "premium" feel in both dark and light modes.

## Impact

- Improved readability and accessibility in light mode.
- Cleaner codebase with reduced hardcoded color dependencies.
- Enhanced visual consistency across onboarding and main navigation.
