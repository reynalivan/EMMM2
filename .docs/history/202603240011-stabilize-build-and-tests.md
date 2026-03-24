# Stabilize Build and Test Infrastructure

## Context
The EMM2 application had accumulated several TypeScript compilation errors, ESLint warnings, and Rust backend type mismatches that were blocking a clean production build and causing test regressions.

## Changes
- **Rust Backend:** Fixed `usize` to `u64` type mismatches in `settings_cmds.rs` and `maintenance_service.rs` to align with Tauri command serialization.
- **TypeScript Frontend:** Resolved all `tsc` errors by aligning domain models, fixing prop interfaces, and correcting Vite dependency resolution.
- **Test Suite:** Refactored all Vitest tests (`dedupService`, `scanService`, `ResolutionModal`) to use the type-safe `commands` registry from `bindings.ts` instead of raw `invoke` calls.
- **Folder Grid:** Restored missing properties and logic in `FolderGrid.tsx`, `useFolderGrid.ts`, and `useFolderGridActions.ts` (restored `toggleObjectMods` and `handleRevealInExplorer`).
- **Internationalization:** Fixed missing i18n keys and accessibility attributes in `ResolutionModal.tsx`.

## Impacted Files
- `src-tauri/src/commands/app/settings_cmds.rs` (modified)
- `src-tauri/src/services/app/maintenance_service.rs` (modified)
- `src/features/scanner/components/ResolutionModal.tsx` (modified)
- `src/features/scanner/components/ResolutionModal.test.tsx` (modified)
- `src/features/folder-grid/FolderGrid.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/scanner/services/dedupService.test.ts` (modified)
- `src/features/scanner/services/scanService.test.ts` (modified)
- `src/lib/bindings.ts` (verified/used)

## Goal
Achieve a zero-error build state and align the test suite with the current type-safe IPC architecture.

## Impact
- **Developer Experience:** Clean `tsc` and `cargo check` allow for faster iteration and CI/CD integration.
- **Reliability:** Type-safe IPC prevents runtime "Command not found" or "Deserialization error" failures.
- **Accessibility:** Improved ARIA support in scanner modals.
