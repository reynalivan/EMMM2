## Fixing Theme Switching (Phase 3: DaisyUI Mechanism)

### Context

- Theme switching in Light mode was reported as "not functional" when trying to return to Onyx (Dark) mode.
- Previous implementation used named @theme blocks for both, which caused variable mapping issues in Tailwind 4 + DaisyUI 5.

### Changes

- Reorganized `App.css` to use a **base (unnamed) @theme block** for the default Onyx/Dark state.
- Used a **named @theme light block** for Light mode overrides, which DaisyUI 5 automatically maps to `[data-theme="light"]`.
- Simplified `useThemeRuntime.ts` to sync both `data-theme` attribute and CSS class for reliable theme activation across both Tailwind 4 and DaisyUI 5 internal components.
- Improved cleanup in `useThemeRuntime.ts` to prevent variable pollution during theme transitions.

### Impacted Files

- `src/App.css` (modified)
- `src/features/settings/theme/useThemeRuntime.ts` (modified)

### Goal

- Restore full functionality to the theme switching system and ensure long-term stability using DaisyUI 5's official mechanism.

### Impact

- Seamless transition between Light and Onyx themes.
- Proper variable resolution for both Tailwind utility classes and DaisyUI components.
- Cleaner CSS-first architecture aligned with modern standards.
