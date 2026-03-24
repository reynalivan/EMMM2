# Resolve IDE-Reported Problems

## Context

Addressed specific IDE (Rust-analyzer and ESLint) warnings and errors that were causing visual noise and linting failures, despite the code being structurally sound and passing full compilation.

## Changes

- **`src-tauri/src/lib.rs`**: Removed unnecessary `mut` on the `tauri_specta::Builder` in the `export_bindings` test. This resolved a specific IDE error regarding `NoRuntime` type resolution which was being triggered by incorrect mutability inference.
- **`src/features/object-list/useObjectListLogic.ts`**: Removed trailing whitespace on line 97 to satisfy Trunk/Linter rules.
- **`src/stores/useAppStore.ts`**: Adjusted the formatting of the `queryKey` array in `initStore` to include a trailing comma and correct indentation, following the IDE's formatting suggestion.

## Impacted Files

- `src-tauri/src/lib.rs` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/stores/useAppStore.ts` (modified)

## Goal

Ensure the codebase is not only compilable but also clean according to IDE and linting rules, providing a better developer experience and preventing CI/CD linting failures.

## Impact

- **Developer Experience**: Reduced IDE noise and clear "green" status for affected files.
- **Code Quality**: Adherence to project-wide formatting and linting standards.
- **Verification**: Confirmed with `pnpm exec tsc` and `cargo check --tests`.
