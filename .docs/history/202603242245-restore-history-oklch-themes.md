## Fixing Theme Switching (Phase 3.3: History-Aligned OKLCH Themes)

### Context

- User reported the dark theme was "lost" and requested using colors from the git history.
- Switched to the more robust DaisyUI 5 theme plugin syntax.

### Changes

- Reverted Onyx (Dark) colors to the original **OKLCH** values found in the git history (`oklch(21% 0.034 264.665)` for base-100, etc.).
- Migrated `App.css` to use the official DaisyUI 5 `@plugin "daisyui/theme"` block syntax.
- Designated Onyx as the `default: true` theme in CSS.
- Maintained the improved Slate-based Light mode but also migrated it to the `@plugin "daisyui/theme"` structure for consistency.
- Unified the Tailwind utility mapping in a final `@theme` block that consumes variables from whichever theme is active.

### Impacted Files

- `src/App.css` (modified)

### Goal

- Restore the beloved original dark aesthetic while maintaining the modern theme engine improvements.

### Impact

- Perfect theme switching driven by DaisyUI's native plugin architecture.
- Authentic "Onyx" look aligned with project origins.
- Higher performance and cleaner CSS structure.
