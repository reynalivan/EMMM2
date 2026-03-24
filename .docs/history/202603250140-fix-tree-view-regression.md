# Fix Tree View Regression & Type Safety

## Context
The hierarchical tree view in the collection preview was failing to render mod items (showing "Status: None") due to inconsistent path normalization and case-sensitive comparisons in the tree builder. Additionally, a TypeScript error was present in the tree view component.

## Changes
- **buildModTree.ts**: 
    - Forced all path comparisons and `startsWith` checks to use lowercase normalized paths (`/` and `toLowerCase()`).
    - Implemented robust relative path calculation by subtracting normalized base path length from the full normalized mod path.
- **CollectionTreeView.tsx**: 
    - Removed invalid `is_safe` property access on `ModTreeNode`.
    - Cleaned up unused `EyeOff` import.
    - Updated empty children fallback message and added i18n keys.
- **Localization**:
    - Added `status.none` and `status.no_subfolders` to `en/common.json` and `id/common.json`.

## Impacted Files
- `src/features/collections/utils/buildModTree.ts` (modified)
- `src/features/collections/components/CollectionTreeView.tsx` (modified)
- `src/locales/en/common.json` (modified)
- `src/locales/id/common.json` (modified)

## Goal
Restore the hierarchical mod tree in the collection preview panel and ensure stable, case-insensitive path grouping.

## Impact
- Fixes the blank/empty mod list in collection previews.
- Improves compatibility with Windows-style backslashes and mixed-case directory names.
- Resolves TypeScript compilation errors.
