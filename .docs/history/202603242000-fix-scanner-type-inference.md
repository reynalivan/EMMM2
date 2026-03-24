# Fix Type Inference in Duplicate Scanner

## Context

The Rust compiler was unable to infer the type of the `members` collection in `scanner.rs` because of variable shadowing and complex iterator mapping during the duplicate grouping phase.

## Changes

- `src-tauri/src/services/scanner/dedup/scanner.rs` (modified): Added explicit type annotation `: Vec<DupScanMember>` to the `members` variable declaration inside `build_groups`.

## Impacted Files

- `src-tauri/src/services/scanner/dedup/scanner.rs` (modified)

## Goal

Restore compilation of the duplicate scanner service.

## Impact

- Fixes the "type annotations needed" compilation error.
- No runtime behavior changes; logic remains identical.
