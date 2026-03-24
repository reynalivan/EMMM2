## Title

Fix IDE Compilation Errors

## Context

The IDE flagged several typing errors (`Unexpected any`), React Compiler memoization warnings, unused imports, and Prettier formatting issues across various frontend feature hooks and core components.

## Changes

- Replaced `any` casting with correct explicit types (`PipelineTask[]`, `MatchedDbEntry`, `GameType`) in App configuration and object hooks.
- Corrected a broken module import path for `bindings.ts` in browser downloads hook.
- Switched nested dependencies (`activeGame?.id`) to base object dependencies (`activeGame`) in `useCallback` dependency arrays to satisfy the React Compiler's inferred dependency checks.
- Formatted multiline type declarations to adhere to Prettier rules.
- Removed unused `ApplyResult` import to clear lint warnings.

## Impacted Files

- `src/App.tsx` (modified)
- `src/features/browser/hooks/useDownloads.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/useObjectListDropZones.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelActions.ts` (modified)
- `src/hooks/useObjects.ts` (modified)

## Goal

To maintain a zero-error and zero-warning state in the frontend codebase, ensuring strong type safety and proper React hook memoization preserving.

## Impact

- Resolved 5 error traces and 2 warnings.
- No direct functional shifts; purely typing, import, and syntax stabilization.
