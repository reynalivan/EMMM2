# EMMM Mod Manager

EMMM is a desktop mod manager for 3DMigoto-based games. It keeps the filesystem as the source of truth while providing a fast workspace for organizing, previewing, importing, and launching game-specific mod libraries.

The project currently targets Genshin Impact, Honkai: Star Rail, Zenless Zone Zero, Wuthering Waves, and Arknights: Endfield through their XXMI-compatible workflows. EMMM is an independent third-party tool and is not affiliated with any game developer, publisher, or platform.

## What it does

- Organizes mods by game, object, and folder, with grid and list views that remain usable for large libraries.
- Reads the enabled state from the folder name on disk. A `DISABLED ` prefix disables a mod; SQLite indexes metadata and hierarchy for search and filtering.
- Imports archives and folders, detects common nested layouts, and keeps Mod Inbox, Downloads, Collections, and the browser scoped to the selected game.
- Provides previews, metadata editing, INI inspection, bulk actions, safe-mode controls, and a trash-based removal flow.
- Saves and applies collections, scans for duplicate files, and offers a guided matching and classification workflow for unstructured folders.
- Watches the library for external changes and reconciles the local index with the filesystem.

## Architecture

EMMM is a Tauri v2 application with a React and TypeScript frontend and a Rust backend. The frontend uses TanStack Query for server state, Zustand for local application state, and TanStack Virtual for large collections. The Rust side owns filesystem operations, archive processing, hashing, database access, and Tauri commands.

The core safety model is intentionally simple:

1. The filesystem determines whether a mod is enabled.
2. SQLite is an index and metadata store, not a competing source of truth.
3. File mutations use recoverable workflows and reconciliation when an operation fails or the library changes outside EMMM.
4. Removed content goes through the trash flow rather than an immediate hard delete.

## Repository layout

```text
src/             React application
src-tauri/       Rust services, database, and Tauri shell
tests/           Unit and browser end-to-end tests
docs/            Requirements, test cases, architecture notes, and history
scripts/         Development and release helpers
```

## Prerequisites

- Node.js 22 through 24
- Corepack, which provides the pinned pnpm 10.24.0 release
- Rust stable and Cargo
- Windows is required for the native desktop workflow

## Setup

Run the setup script from PowerShell to validate the required runtimes, install locked frontend dependencies, and fetch Cargo crates.

```powershell
.\setup.ps1
```

Start the desktop application:

```powershell
pnpm tauri dev
```

`pnpm tauri dev` prepares an isolated development catalog pack. By default it expects a sibling checkout at `../3dm-catalog-asset`; set `EMMM_DEV_CATALOG_SOURCE` to use another local catalog source. The command stops if the pack or its manifest is invalid so the development profile cannot silently run with incomplete catalog data.

## Common commands

| Command                                             | Purpose                                               |
| --------------------------------------------------- | ----------------------------------------------------- |
| `pnpm dev`                                          | Start the frontend-only Vite server.                  |
| `pnpm tauri dev`                                    | Start the desktop application with the native bridge. |
| `pnpm build`                                        | Type-check and build the frontend.                    |
| `pnpm tauri build`                                  | Create the desktop application bundle.                |
| `pnpm test --run`                                   | Run the Vitest suite once.                            |
| `pnpm test:e2e`                                     | Run WebdriverIO browser end-to-end tests.             |
| `pnpm lint`                                         | Run ESLint.                                           |
| `pnpm lint:arch`                                    | Check frontend architecture boundaries.               |
| `pnpm format`                                       | Format supported files with Prettier.                 |
| `cargo test --manifest-path src-tauri/Cargo.toml`   | Run Rust tests.                                       |
| `cargo clippy --manifest-path src-tauri/Cargo.toml` | Run Rust static analysis.                             |

## Configuration

Copy `.env.example` to `.env` only when you need local GameBanana or observability integration. Keep `.env` local. It can contain credentials and is not part of the repository.

## Documentation

- [Project introduction](docs/intro.md)
- [Requirements](docs/requirements)
- [Test cases](docs/test-cases)
- [Architecture and migration notes](docs/relocation.md)
- [Contribution and agent guidance](AGENT.md)

## Status

This repository is under active development. Check the requirements and test cases before relying on a workflow that is important to your mod library.
