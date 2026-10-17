# EMMM — 3DMigoto Mod Manager

EMMM is an open-source Windows desktop manager for GIMI, SRMI, ZZMI, WWMI, and EFMI mod libraries. Point it at the folders you already use, including libraries spread across multiple locations. EMMM follows your layout instead of requiring you to rebuild it around the manager.

EMMM is an independent third-party project. It is not affiliated with, endorsed by, or sponsored by any game, game developer, publisher, or platform mentioned in this repository.

## What you can do

### Import and classify mods

Drop a mod folder or a `.zip`, `.7z`, or `.rar` archive into Mod Inbox. EMMM detects mod roots inside nested downloads, then suggests an identity and destination using available INI hashes, [catalog](https://github.com/reynalivan/3dm-catalog-asset) records, aliases, and folder names. You review ambiguous matches and placement before applying them. The catalog can also add available metadata and thumbnails to matched objects.

### Work with your own folders

Browse by game and object in grid or list view, open nested folders, and use breadcrumbs to go back. Search and filter by available names, categories, and metadata; pin the mods you return to often. Enable or disable a mod, use **Enable Only This** to compare alternatives in a group, or apply bulk actions. A `DISABLED ` folder prefix records the enabled state on disk, while SQLite indexes the library for browsing.

EMMM watches relevant folder, INI, metadata, and image changes, so edits made in File Explorer can be reconciled with the app's view.

### See your mods and controls

EMMM detects images in mod folders. Choose a preview or paste a thumbnail from the clipboard; edit metadata and inspect INI files in the app. Detected keybinds from active mods appear in a searchable view. For supported runtime matches, the optional in-game KeyViewer shows bindings and status; **F7** toggles the overlay.

### Save setups and maintain the library

Save collections and switch between them while the game is in the foreground with **Shift+F5** and **Ctrl+F5**. **F5** toggles Safe Mode, which limits the active setup to mods you have classified as Safe. Storage Optimizer helps you inspect duplicate files before deciding what to clean up.

The normal **Remove Mod** action sends the folder to the OS Recycle Bin. Keep a separate backup before large imports or reorganizations.

## Catalog data

The separate [3DM Catalog Asset](https://github.com/reynalivan/3dm-catalog-asset) supplies known identities, aliases, hashes, available metadata and images, and reviewed KeyViewer runtime targets. It contains no game mods or manager executable. In EMMM, open **Settings > Catalog Assets** to check the official catalog release channel, or use **Install from ZIP** to review and install a downloaded `catalog-pack.zip`. Catalog coverage differs by game and entry; matching suggestions still need your review.

## Getting started

EMMM is currently developed for Windows. This repository contains the application source and development instructions below. The [catalog release](https://github.com/reynalivan/3dm-catalog-asset/releases/latest) is data for EMMM, not an application installer.

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
- Corepack, which provides the pinned pnpm 11.19.0 release
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
