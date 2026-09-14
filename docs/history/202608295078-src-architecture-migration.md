# Architecture Migration to Hybrid FSD

## Context
The `src/` directory suffered from "Domain Leakage" where domain-specific hooks and components were placed in global `/hooks` and `/components` directories. A structural reorganization was required to adopt a hybrid Feature-Sliced Design (FSD) architecture.

## Changes
- Moved application entry points to `src/app/`.
- Moved global libraries, utils, and tauri bindings to `src/core/`.
- Moved reusable UI components and shared hooks to `src/shared/`.
- Migrated domain-specific hooks and modals into their respective feature directories (`src/features/<feature-name>/`).
- Moved testing utilities and setup to `src/tests/`.
- Updated all internal absolute and relative imports across the codebase using a custom AST-like path resolution script.
- Regenerated Specta Tauri bindings and fixed type imports to point to `src/core/tauri/bindings.gen.ts`.
- Updated `vite.config.ts` and `eslint.config.js` to reflect new directory paths.

## Impacted Files
- `src/App.tsx`, `src/main.tsx` -> `src/app/`
- `src/lib/*` -> `src/core/lib/`, `src/core/tauri/`
- `src/components/layout/`, `src/components/ui/` -> `src/shared/components/`
- `src/utils/` -> `src/shared/utils/`
- `src/hooks/` (generic) -> `src/shared/hooks/`
- `src/hooks/` (domain-specific) -> `src/features/*/hooks/`
- `src/components/modals/` -> `src/features/*/modals/`
- `src/setupTests.ts`, `src/testing/` -> `src/tests/`
- `vite.config.ts` (modified)
- `eslint.config.js` (modified)
- `src-tauri/src/lib.rs` (modified)

## Goal
Establish a scalable, domain-driven structure where features are isolated and global folders strictly contain reusable, domain-agnostic code.

## Impact
- Clean architecture with no domain leakage.
- Type checking and tests all pass with 0 errors.
- No behavioral or runtime changes; purely structural.
- Specta generated path changed from `src/lib` to `src/core/tauri`.

## Notes
Used an automated Node script (`move-and-fix.mjs`) to synchronously move files and rewrite imports in bulk, avoiding partial states and TS-Morph Windows filesystem limitations. Type suppression (`@ts-expect-error missing command`) was added for `openModInboxFolder` as it was referenced in components but not yet implemented in Rust.


## Phase 2: Intra-Feature Structure Cleanup

### Context
After the high-level Feature-Sliced Design migration, the `src/features/` directory still contained root-level clutter and nested domains (e.g. `src/features/settings/tabs`).

### Changes
- Grouped scattered `*.tsx` into `components/` for features like `onboarding`, `settings`, `scanner`, etc.
- Moved loosely floating feature hooks and utils into `hooks/` and `utils/` within their respective domains.
- Fixed all broken relative imports and `vi.mock` references.
- Updated test setup to resolve SVG mock issues with `motion/react`.
- Moved remaining root files in `src/features/workspace-runtime/` into `hooks/` and `utils/` subfolders.

### Impacted Files
- `src/features/*/` (Reorganized internals)
- `src/tests/setupTests.ts` (Updated `motion/react` mock)
- `src/app/App.test.tsx` (Fixed `vi.mock` relative paths)
- All internal feature tests (Fixed relative import paths via regex)
- `src/features/workspace-runtime/` (Fixed final loose files root)

### Goal
Achieve 100% adherence to the component/hook/util internal segregation rule within features, passing all 743 tests successfully.
