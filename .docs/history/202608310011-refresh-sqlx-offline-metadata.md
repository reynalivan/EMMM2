# Refresh SQLx Offline Metadata

## Context

The reported Tauri startup log reached Rust compilation but failed after native dependency resolution. Reproducing the command from the repository root could also expose stale SQLx metadata because Cargo then read the old local `app.db` instead of the package's offline configuration.

## Changes

- Regenerated `src-tauri/.sqlx` query metadata against a temporary SQLite database.
- Applied all repository migrations before preparing the metadata, including the `games.mods_path`, `games.ready_to_move_path`, and download queue schema.
- Removed the temporary database after generation; the application database was not modified.

## Impacted Files

- `src-tauri/.sqlx/` (refreshed generated query metadata)
- `src-tauri/migrations/` (used as the schema source; no migration logic changed)

## Goal

Keep SQLx compile-time query validation aligned with the committed migrations so Tauri development builds do not depend on a stale local database.

## Impact

Cargo can compile in offline mode from the Tauri working directory. The exact `pnpm tauri dev` flow now reaches and runs `emmm.exe`; the earlier `zstd.lib` linker error and the original port error did not reproduce after the metadata refresh.

## Validation

- Applied all three migrations to a temporary SQLite database.
- Ran `cargo sqlx prepare -- --all-targets --all-features`.
- Ran `cargo run --no-default-features --color always --` from `src-tauri` successfully.
- Ran `pnpm tauri dev` end-to-end successfully; Vite, Cargo, linker, and EMMM startup all completed.
