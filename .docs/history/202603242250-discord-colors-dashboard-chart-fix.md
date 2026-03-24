## Fixing Dashboard Charts and Restoring Discord Colors (Phase 3.4)

### Context
- User requested original Discord-style hex colors.
- Dashboard charts were rendering black because of an invalid CSS wrapper (`oklch(hex)`).

### Changes
- Restored exact Discord hex colors in `App.css` for the **Onyx** theme:
  - Base colors: `#313338`, `#2b2d31`, `#1e1f22`.
  - Brand colors: `#5865f2` (Blurple), `#23a559` (Green).
- Fixed `Dashboard.tsx` chart rendering:
  - Removed redundant `oklch()` wrappers from `CHART_COLORS`.
  - Corrected `Tooltip` and `XAxis`/`YAxis` stroke handling to be compatible with hex variables.
  - Used `strokeOpacity` for axis lines instead of inline OKLCH opacity to ensure cross-theme stability.

### Impacted Files
- `src/App.css` (modified)
- `src/features/dashboard/Dashboard.tsx` (modified)

### Goal
- Restore the beloved Discord dark aesthetic and fix data visualization on the dashboard.

### Impact
- Dashboard charts are now fully visible and theme-aware.
- The "Onyx" theme once again feels authentic to the project's "Discord-inspired" roots.
