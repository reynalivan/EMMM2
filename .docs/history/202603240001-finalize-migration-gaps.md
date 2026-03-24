# 202603240001-finalize-migration-gaps

## Finalization of Mod Management Architecture

### Context

This change resolves the remaining architectural gaps identified during the EMMM system migration, specifically focusing on data synchronization between the database and frontend, and ensuring robust mod restoration during corridor switches in Safe Mode.

### Changes

- **Object Detail Sync**: Added `active_mod_paths` to `ObjectSummary` and updated `get_filtered_objects` SQL to use `GROUP_CONCAT` for real-time mod status enrichment.
- **Switch Pipeline Resilience**: Implemented `restore_via_system_reason` in `switch_pipeline.rs` as a fallback when no collections are saved, ensuring mods disabled by the system are restored.
- **Documentation**: Updated `flow.md` to include `is_pinned` in the object schema and reflect the new restoration logic.
- **Pipeline Integrity**: Standardized the use of `PostApplyContext` across all mod-mutating operations to ensure side-effects (KeyViewer, Conflicts) are always processed.

### Impacted Files

- `src-tauri/src/repo/object_repo.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `.docs/flow.md` (modified)

### Goal

The system now maintains a single source of truth for mod states and ensures that corridor switches are atomic, resilient, and fully synchronized with UI previews.

### Impact

- Improved reliability of Safe Mode transitions.
- More accurate "active mods" display in the Preview Panel.
- End-to-end type safety for object summaries.
