# Fix project errors and warnings

## Context

Various TypeScript and Rust errors were reported across the project, including `any` type violations, missing dependencies in hooks, invalid expressions, and missing type definitions for the `@tauri-apps/plugin-fs` plugin.

## Changes

- Fixed `any` types in `Dashboard.tsx`, `GeneralTab.tsx`, `themeOptions.ts`, and `AuroraBackground.tsx` using proper casting or specific types.
- Corrected `useReviewTable.tsx` by removing an invalid syntax-error-inducing expression and adding a missing `useMemo` dependency (`t`).
- Updated `@tauri-apps/plugin-fs` type shim to include missing exports (`writeFile`, `writeTextFile`, `readTextFile`).
- Removed redundant `DuplicateModInfo` import in `src-tauri/src/services/scanner/conflict/mod.rs`.

## Impacted Files

- `src/features/dashboard/Dashboard.tsx` (modified)
- `src/features/scanner/components/useReviewTable.tsx` (modified)
- `src/features/settings/tabs/GeneralTab.tsx` (modified)
- `src/features/settings/theme/themeOptions.ts` (modified)
- `src/features/welcome/AuroraBackground.tsx` (modified)
- `src/types/tauri-plugin-fs.d.ts` (modified)
- `src-tauri/src/services/scanner/conflict/mod.rs` (modified)

## Goal

The system now passes TypeScript linting and Rust compilation with zero errors in the modified files.

## Impact

- Improved type safety across Dashboard, Settings, and Scanner features.
- Resolved build-breaking syntax errors in the Scanner UI.
- No breaking changes; all functionality remains intact.
