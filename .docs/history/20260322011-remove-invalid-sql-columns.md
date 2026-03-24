# Phase 12: Audit & Fix is_nested_mod DB Crash

## Context

During user testing of the Collection Apply Preview and the Topbar Dirty State indicator, the backend returned a fatal `(code: 1) no such column: is_nested_mod` SQLite exception. This occurred because previous refactoring sessions erroneously assumed that the `mods` table housed an `is_nested_mod` column (which never actually existed in the schema).

## Changes

- **SQL Query Normalization (Corridor Service)**: Removed `AND is_nested_mod = 0` clauses from the `get_corridor_state` and `switch_corridor` database queries, as the `mods` table intrinsically only contains root-level entries (`MemberKind::Mod`).
- **SQL Query Normalization (Collection Service)**: Removed `is_nested_mod` references from the `preview_apply_collection` projection list, resolving the crash triggered when opening the "Apply Collection" preview modal.
- **Data Shape Alignment**: Removed the conditional `MemberKind::Mod` vs `MemberKind::Nested` mapping for active queries, standardizing all parsed records strictly as `MemberKind::Mod`.

## Impacted Files

- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)

## Goal

Restore application stability by scrubbing all hallucinated database columns from the backend Rust queries, thereby allowing Dirty State tracking and UI Previews to function correctly.

## Impact

- **Database Resilience:** Eliminates the runtime SQLite panic triggered by `sqlx::query` strings executing dynamically.
- **TopBar Functionality:** The Dirty State tracker is now completely unblocked, successfully identifying when the active selection deviates from saved states.
- **Modal Functionality:** The Apply Collection modal successfully evaluates "Before" vs "After" diffs without immediate closure.
