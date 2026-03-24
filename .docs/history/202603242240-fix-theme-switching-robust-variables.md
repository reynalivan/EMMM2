## Fixing Theme Switching (Phase 3.2: Robust CSS Variable Pattern)

### Context

- Switched to Onyx (Dark) was reportedly "lost" or incorrectly showing as Light mode.
- Root cause: Tailwind 4's base `@theme` was likely getting superseded by DaisyUI's default light theme, and the attribute selector was missing.

### Changes

- Migrated `App.css` to a **Robust CSS Variable Pattern**.
- Defined design tokens in `@theme` using variables (e.g., `--color-base-100: var(--b1)`).
- Explicitly defined `:root, [data-theme="onyx"]` to set the dark palette primitives.
- Explicitly defined `[data-theme="light"]` to override with the light palette primitives.
- This ensures the `data-theme` attribute (DaisyUI's native mechanism) directly and reliably controls the entire color system.

### Impacted Files

- `src/App.css` (modified)

### Goal

- Eliminate any ambiguity between Tailwind and DaisyUI theming by using a shared primitive variable layer.

### Impact

- Guaranteed theme persistence and correct visual state for Onyx (Dark) and Light modes.
- Simplified CSS architecture that is easier to debug.
