# Active Conflict Review Controls

## Context

The automatic active-conflict overlay interrupted normal work and offered no way to defer known conflict groups.

## Changes

- Replaced the overlay with a warning-only topbar indicator that opens the review dialog.
- Added persistent, game-scoped ignored conflict groups, canonicalized and validated in Rust.
- Added unresolved/ignored filtering, queue counts, per-group and per-mod ignore actions, selection checkboxes, and bulk ignore/restore controls.

## Impacted Files

- `src-tauri/migrations/20261010000100_ignored_active_mod_conflicts.sql` (added)
- `src-tauri/src/modules/workspace/adapters/sqlite/conflict/mod.rs` (modified)
- `src-tauri/src/modules/library/adapters/tauri/mod_meta_cmds.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src-tauri/AGENT.md` (modified)
- `src/shared/api/tauri/bindings.gen.ts` (modified)
- `src/features/conflict-report/*` (modified)
- `src/features/mod-runtime/hooks/useFolderMutations.ts` (modified)
- `src/features/mod-runtime/@x/conflict-report.ts` (modified)
- `src/widgets/launch-bar/*` (modified)
- `src/shared/i18n/locales/{en,id,zh}/{layout,scanner}.json` (modified)

## Goal

Known conflicts can be deferred safely while unresolved conflicts remain visible and actionable.

## Impact

No mod is enabled, disabled, or deleted by ignoring a conflict; the preference only changes review visibility for that game.

