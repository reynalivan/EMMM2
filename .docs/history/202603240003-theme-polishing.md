## Theme Polishing & Gap Closure

### Context

Address remaining UI "gaps" where hardcoded colors or lack of contrast prevent a premium experience across different themes (Onyx, Light, and Custom).

### Changes

- **AnimatedLogo**: Replaced hardcoded `white` with `currentColor`.
- **Dashboard**: Refactored `CHART_COLORS` to use theme-aware `oklch(var(--p))`, etc.
- **App.css**: Defined `.bg-github-gradient` using semantic variables.
- **WelcomeScreen**: Added high-contrast text color classes to the logo container.
- **DynamicThemeInjector**: Added fallback logic and style cleanup for theme switching.

### Impacted Files

- `src/features/welcome/AnimatedLogo.tsx` (modified)
- `src/features/dashboard/Dashboard.tsx` (modified)
- `src/features/settings/theme/DynamicThemeInjector.tsx` (modified)
- `src/features/onboarding/WelcomeScreen.tsx` (modified)
- `src/App.css` (modified)

### Goal

The system now provides 100% theme consistency across all core UI components, ensuring full accessibility and visibility in light mode and custom themes.

### Impact

- Improved visibility of branding on light backgrounds.
- Consistent, theme-aware data visualization in the Dashboard.
- Robust runtime theme switching with zero "style leaks" from previous custom themes.
