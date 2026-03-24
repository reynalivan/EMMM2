# Fix Unused Variable in lib.rs

## Context

Fixed a compiler warning reported by the IDE regarding an unused variable in the `export_bindings` test.

## Changes

- **`src-tauri/src/lib.rs`**: Converted the `let builder =` assignment into a standalone expression. Since the `Builder` chain ends with `.expect(...)`, the code still executes the side effect (exporting bindings) without needing to store the result in a variable.

## Impacted Files

- `src-tauri/src/lib.rs` (modified)

## Goal

Achieve a warning-free compilation for the backend tests.

## Impact

- **Developer Experience**: No more unused variable warnings in `lib.rs`.
- **Code Quality**: Slightly cleaner test code.
- **Verification**: Verified with `cargo check --tests`.
