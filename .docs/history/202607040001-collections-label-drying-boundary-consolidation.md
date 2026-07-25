# Collections System — Label DRYing & Boundary Consolidation

## Context

Finalization of the Collections System Audit Remediation Plan (Phase 4 and I6).
The goal was to eliminate inline duplication of the `unsavedLabels` object across five different UI components and clarify the architectural boundaries of the Collections feature, which handles Collections, Corridor State, Safe Mode, and PIN Security.

---

## Changes

### Phase 4 — Label DRYing

- Created `useUnsavedLabels()` and `useCorridorSwitchLabels()` hooks in `src/lib/corridorLabels.ts` to centralize i18n translation strings.
- Replaced the inline `{ safeLabel: t(...), unsafeLabel: t(...) }` objects across 5 consumers with these new hooks.
- Removed redundant `useTranslation('layout')` namespaces from consumers where no longer needed.

### I6 — Consolidate Feature Boundaries

- **Barrel Export**: Created `src/features/collections/hooks/index.ts` to cleanly export all Collection, Corridor, and PIN hooks.
- **Documentation**: Added `src/features/collections/README.md` explaining why these visually-cohesive concerns are grouped together despite backend separation.
- **Consumer Updates**: Updated `CollectionsPage.tsx` and `ContextControls.tsx` to import their collections hooks from the barrel file. Note: The Safe Mode and Corridor Switch hooks were previously removed from these files in a separate simplification pass.

---

## Impacted Files

**Added**
- `src/features/collections/README.md`
- `src/features/collections/hooks/index.ts`

**Modified**
- `src/lib/corridorLabels.ts` (added hooks)
- `src/features/collections/components/CollectionList.tsx`
- `src/features/collections/components/CollectionPreviewPanel.tsx`
- `src/features/collections/components/ApplyCollectionModal.tsx`
- `src/features/collections/CollectionsPage.tsx`
- `src/features/safe-mode/ModeSwitchConfirmModal.tsx`
- `src/components/layout/top-bar/ContextControls.tsx`

---

## Goal

The UI is now fully DRY regarding Safe Mode labels, ensuring consistent i18n translations. Feature boundaries are explicitly documented and the import graph is flattened via the new hooks barrel file.

---

## Impact

- **Maintenance**: Changing translation keys or label structure now only requires editing one place (`corridorLabels.ts`).
- **Organization**: External consumers no longer need to know internal file structure (`useCollections` vs `useCorridor`).
- **Safety**: Fully type-checked; zero TypeScript or ESLint regressions.
