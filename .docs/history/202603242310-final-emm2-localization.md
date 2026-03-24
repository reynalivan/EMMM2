# Final EMM2 Localization

### Context
Complete multi-language localization (EN, ID, ZH) across the app to resolve the final remaining gaps in Onboarding and Launch components.

### Changes
- Replaced hardcoded strings with i18n hooks in all previously unlocalized components (`LaunchBar`, `WelcomeScreen`, `ManualSetupForm`).
- Enforced complete gap parity among `English`, `Indonesian`, and `Chinese` JSON maps for `objects`, `settings`, `layout`, and `scanner`.
- Handled UI translation interpolation for interactive toasts such as Shader Conflicts and Missing Pins.

### Impacted Files
- `src/locales/id/*.json` (modified)
- `src/locales/zh/*.json` (modified)
- `src/locales/en/*.json` (modified)
- `src/features/onboarding/WelcomeScreen.tsx` (modified)
- `src/features/onboarding/ManualSetupForm.tsx` (modified)
- `src/features/onboarding/AutoDetectResult.tsx` (modified)
- `src/features/launch-bar/LaunchBar.tsx` (modified)
- `src/features/object-list/useObjHandlersCrud.ts` (modified)

### Goal
Ensure a flawless, zero-hardcode multi-language experience without any translation gaps causing render failures or inconsistent UI.

### Impact
- The application is 100% internationalized.
- Zero remaining hardcoded UI texts.
- Edge-case interactive modals have robust handling for localized states.
