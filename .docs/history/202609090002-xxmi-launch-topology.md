# XXMI launch topology

## Context

Importer folders managed by a full XXMI installation can contain 3DMigoto files without a launcher executable. Treating those folders as standalone caused a false setup warning and persisted a directory as an executable path.

## Changes

- Added explicit `standalone` and `xxmi_managed` launch modes, an instance path, and a shared XXMI launcher path to game configuration.
- Added an additive SQLite migration. Existing configurations retain standalone behavior and must provide an actual game executable before Play can run.
- Recognize a managed root only when `Resources/Bin/XXMI Launcher.exe` is an executable file. Managed launch uses `--nogui --xxmi <importer>` and never starts a second game executable.
- Kept standalone launcher discovery to executable files directly under the importer root; nested files such as entries under `Mods` are ignored.
- Updated onboarding, Settings, generated bindings, localized labels, and focused regression tests for both topologies.

## Impacted areas

- `src-tauri/migrations/20260909000200_game_launch_modes.sql` and `src-tauri/.sqlx/`
- `src-tauri/src/modules/games/`, `src-tauri/src/modules/settings/application/config/`, and game configuration fixtures
- `src/pages/onboarding/`, `src/pages/settings/`, `src/shared/api/tauri/bindings.gen.ts`, and localized game settings/onboarding strings

## Goal

Ensure full XXMI installations launch through their shared launcher while standalone importer packages retain their own loader-and-game-executable workflow.

## Notes

- The supported managed importer set remains GIMI, SRMI, WWMI, ZZMI, and EFMI; HIMI is intentionally out of scope.
