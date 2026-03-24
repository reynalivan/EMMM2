## Fixing Theme Switching (Phase 3.1: DaisyUI Internal Variables)

### Context

- Theme switching was partially working (toast success) but DaisyUI components were not reflecting color changes.
- Root cause: Missing mapping between Tailwind 4 variables (`--color-primary`) and DaisyUI internal variables (`--p`, `--s`, `--b1`, etc.) in `App.css`.

### Changes

- Updated `App.css` to explicitly map Tailwind 4 design tokens to DaisyUI's internal variable system.
- Base `@theme` (Onyx) now overrides `--p`, `--s`, `--a`, `--n`, and `--b1-3` / `--bc` globally.
- Named `@theme light` now provides specific Slate-based overrides for the same DaisyUI variables.
- This ensures that components like `.btn-primary` or `.text-base-content` use the correct theme colors immediately upon switching.

### Impacted Files

- `src/App.css` (modified)

### Goal

- Ensure that every DaisyUI component and Tailwind utility correctly reflects the selected theme.

### Impact

- Robust theme switching for all UI elements.
- Consistent aesthetics between built-in and custom themes (which were already using this mapping in `DynamicThemeInjector`).
