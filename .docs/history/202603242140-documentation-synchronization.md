# Documentation Synchronization: Storage Optimizer v2

## Context

Functional implementation of the Storage Optimizer (Duplicate Scanner) evolved during development to include a dedicated page architecture, a persistent ignore/whitelist system, and variant-aware intelligence. The technical documentation required synchronization to maintain its role as the source of truth.

## Changes

### Requirement Specifications

- **req-32-dedup-scanner.md**:
  - Updated Success Criteria to include Variant-Awareness and Persistent Whitelisting.
  - Refined AC-32.2.x table to reflect the new `/storage-optimizer` dedicated page and ignore recovery modal.
  - Updated technical schema for the `duplicate_whitelist` database table.

### Implementation History

- Created detailed logs for:
  - Storage Optimizer Dedicated Page & Whitelist Repository.
  - Core System Stabilization (Hotkey & Watcher regressions).
  - Frontend Dependency & Build Fixes (Vite import analysis).

## Impacted Files

- `.docs/requirements/req-32-dedup-scanner.md` (modified)
- `.docs/history/202603242120-storage-optimizer-ignore-and-build-fix.md` (added)
- `.docs/history/202603242130-fix-hotkey-watcher-regressions.md` (added)

## Goal

Ensure all architectural and functional changes are accurately reflected in the long-term project documentation.

## Impact

- **Maintenance**: Future developers have an accurate map of the new ignore system and dedicated page structure.
- **Traceability**: Implementation history clearly links build stabilization efforts to the relevant structural changes.
