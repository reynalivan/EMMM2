# Dashboard Statistics Visual Upgrade

## Context
The dashboard charts were appearing "blank/black" and lacked visual appeal. They also didn't take full advantage of the theme system or modern UI aesthetics.

## Changes
- **Vibrant Palette**: Switched from limited colors to a full range of DaisyUI semantic tokens (`--p`, `--s`, `--a`, etc.).
- **SVG Gradients**: Added linear gradients (`gradientPrimary`, etc.) to provide depth and a "glassmorphism" look to `BarChart` and `PieChart`.
- **StatTile Enhancement**: Added dynamic background gradients, icon scaling, and value shifting animations on hover.
- **Chart Polish**: Increased `paddingAngle` in Pie charts and added `strokeWidth` separation. Improved `Tooltip` styling with backdrop blurs and better shadows.
- **Animations**: Configured `animationDuration` for smoother initial data rendering.

## Impacted Files
- `src/features/dashboard/Dashboard.tsx` (modified)

## Goal
A premium, colorful, and theme-responsive dashboard that "wows" the user and provides clear statistical insights.

## Impact
- **UI/UX**: Significant improvement in first-glance aesthetics.
- **Themes**: Seamless integration with "Onyx" (dark) and "Light" modes.
- **Performance**: No measurable impact; uses lightweight CSS and SVG features.
