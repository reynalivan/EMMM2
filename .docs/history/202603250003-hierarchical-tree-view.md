# Hierarchical Collection Tree View

## Context
The Collection Preview UI used a flat list that didn't show folder hierarchy, making it hard to see which mods belonged to which subfolders. Redundant "purple folder" levels for single-container objects also added unnecessary UI noise.

## Changes
- **Recursive Tree Builder**: Implemented `buildModTree` with recursive directory traversal and automatic flattening of single-root objects.
- **Recursive UI Component**: Updated `CollectionTreeView` to render folders and mods at arbitrary depths with proper indentation and icons.
- **L1 Mod Counts**: Moved mod counts to the Object header (L1) for better visibility.
- **Type Safety**: Updated `FolderGridResponse` to include `ancestor_disabled_by` for path-based status inheritance.
- **Test Alignment**: Updated `useCollections` test suite to match new hook signature.

## Impacted Files
- `src/features/collections/utils/buildModTree.ts` (modified)
- `src/features/collections/components/CollectionTreeView.tsx` (modified)
- `src/types/object.ts` (modified)
- `src/features/collections/hooks/useCollections.test.ts` (modified)

## Goal
A clean, filesystem-accurate tree view that reduces "purple icon" noise for simple objects while supporting deep nesting for complex mod packs.

## Impact
- Better scannability of mod structures.
- Reduced UI depth for 90% of objects (single root).
- Consistent hierarchy across Preview, Apply, and Mode Switch modals.
