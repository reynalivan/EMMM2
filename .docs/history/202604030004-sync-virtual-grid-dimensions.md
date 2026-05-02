### Title

Sync Virtual Grid Dimensions and Aspect Ratio

### Context

After refactoring the `FolderGrid` rendering onto a dynamic CSS Grid, the gap horizontally was flawless, but there remained a significant discrepancy in the calculated vertical gaps (`rowHeight`). This discrepancy surfaced because `useFolderGridLayout` math miscalculated row heights by utilizing a portrait `4:3` size multiplier, whereas the actual visual components use `aspect-square`. Additionally, `cardWidth` fallback estimations suffered slight offsets against the DOM CSS `1fr` rule due to an outdated `max_width` hard cap.

### Changes

- Removed `CARD_MAX_W` constant dependencies out of `useFolderGridLayout`.
- Stripped `Math.min(CARD_MAX_W, ...)` from `cardWidth` pipeline, allowing perfect mirror matching to actual CSS `auto-fit` stretches.
- Modified `cardHeight` mathematical formula from `Math.round(cardWidth * (4/3))` down directly to `Math.round(cardWidth)` aligning seamlessly with 1:1 `aspect-square` layout mechanics.

### Impacted Files

- `src/features/folder-grid/hooks/useFolderGridLayout.ts` (modified)

### Goal

Eliminate fictitious ghost vertical gaps scaling over dimensions and secure complete stability over width computations matching CSS behavior natively.

### Impact

- High virtualized row densities. Cards layout is noticeably tighter and fluid dynamically without overlap errors.
