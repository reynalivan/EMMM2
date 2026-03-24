# Domain-Specific Type Import Refactor

## Context

Refactored the project's import structure to move away from a monolithic `bindings.ts` re-export pattern, improving type safety, reducing compilation overhead, and aligning with modern TypeScript best practices.

## Changes

- **Refactored `src/lib/bindings.ts`**: Removed all type re-exports. The file now only contains the `commands` registry and internal IPC types.
- **Domain-Specific Imports**: Updated all components, hooks, and services to import types directly from their respective `@/types/` domain files.
- **Fixed `tsc` Errors**: Resolved all compilation errors resulting from the refactor, including unused imports and missing properties in `useObjectListHandlers`.
- **Test Alignment**: Updated multiple test files and the global `setupTests.ts` to reflect the new import structure.

## Impacted Files

- `src/lib/bindings.ts` (modified)
- `src/features/object-list/useObjectListHandlers.ts` (modified)
- `src/features/object-list/useObjHandlersDrop.ts` (modified)
- `src/features/dashboard/hooks/useDashboardStats.test.ts` (modified)
- `src/features/object-list/useObjectListHandlers.test.ts` (modified)
- `src/features/settings/hooks/useMetadataSync.ts` (modified)
- `src/lib/services/dedupService.test.ts` (modified)
- `src/lib/services/scanService.test.ts` (modified)
- `src/setupTests.ts` (modified)

## Goal

Establish a clean, domain-driven import pattern that ensures the single source of truth for types resides within their respective modules, leading to a more maintainable and type-safe codebase.

## Impact

- **Performance**: Potentially faster compilation and IDE responsiveness due to reduced re-export chains.
- **Maintainability**: Clearer ownership of types and easier navigation for developers.
- **Reliability**: Verified clean build (`pnpm tsc` and `pnpm build`) ensures zero regressions in the build pipeline.
