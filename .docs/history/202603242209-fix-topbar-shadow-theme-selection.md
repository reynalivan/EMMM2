## Fix TopBar Shadow and Theme Selection

### Context

- The EMMM logo shadow in the TopBar was reported to have a "bad color" in Light mode.
- Theme switching (Onyx/Dark) was failing because Tailwind 4 `@theme` blocks were not being activated (missing CSS class on `documentElement`).

### Changes

- Updated `useThemeRuntime.ts` to sync the theme as both a `data-theme` attribute (DaisyUI) and a CSS class (Tailwind 4).
- Refined `.glow-text` in `App.css` to use theme-aware variables (`--glow-color`, `--glow-blur`).
- Substantially reduced glow opacity and blur for Light mode to provide a more premium, subtle lift.
- Added a `gap-0.5` to the TopBar logo container for improved brand readability.

### Impacted Files

- `src/features/settings/theme/useThemeRuntime.ts` (modified)
- `src/App.css` (modified)
- `src/components/layout/top-bar/index.tsx` (modified)

### Goal

- Restore theme switching functionality and provide a premium, theme-aware branding experience.

### Impact

- Theme selection now works correctly across all components.
- TopBar branding looks more sophisticated in both Light and Dark modes.
