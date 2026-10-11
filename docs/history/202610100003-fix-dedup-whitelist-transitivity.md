# Fix duplicate whitelist transitivity

## Context

An ignored duplicate pair could return in an exact-match group when both folders
matched a third folder.

## Changes

- Keep exact-match edges from whitelist-containing components as pair-specific
  groups instead of unioning them transitively.
- Add a three-folder regression: ignored `A-B` yields separate `A-C` and `B-C`
  groups.

## Impacted Files

- `src-tauri/src/modules/duplicates/application/dedup/scanner.rs` (modified)
- `src-tauri/src/modules/duplicates/application/dedup/grouping.rs` (modified)
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_scanner_tests.rs` (modified)
- `src-tauri/src/modules/duplicates/application/dedup/tests/dedup_grouping_tests.rs` (modified)

## Goal

Whitelisted pairs remain excluded from later duplicate-resolution groups.

## Impact

Exact groups containing an ignored pair can now be represented as overlapping
two-member groups; unchanged exact clusters retain their compact spanning group.

## Validation

- Rustfmt and diff-whitespace checks passed.
- Targeted Rust test is blocked before compilation because `compress-tools`
  cannot find libarchive without Vcpkg/`VCPKG_ROOT`.
