# EMMM2 Mod Manager: Technical Roadmap & Architecture

> **Philosophy:** Zero-Compromise. Native Performance. Data Safety. Premium Aesthetics.
> **Core Stack:** Tauri v2 (Rust), React v19, SQLite, Tailwind v4, DaisyUI 5.

**EMMM2** is an intelligent **Mod Orchestrator** for the 3DMigoto ecosystem (Genshin Impact, HSR, ZZZ, WW). This application bridges the gap between tedious manual file management and the needs of modern users who demand instant speed and absolute data safety.

---

## 🗺️ Epic Roadmap: The 13 Pillars

Here is the mapping of features (Epics) that build the foundation of EMMM2. Each Epic is designed to be independent yet modularly integrated.

| No     | Epic Module                                                    | Core Responsibility (High Level)                                                                     | Technology Key                          |
| :----- | :------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------- | :-------------------------------------- |
| **01** | [Onboarding & Config](@/.docs/epic1-onboarding-config.md)      | **First Impression.** Heuristic game installation detection & folder integrity validation.           | `Rust WalkDir`, `tauri-plugin-store`    |
| **02** | [Intelligent Scanning](@/.docs/epic2-mod-scan-organization.md) | **The Brain.** Mod identification pipeline (Name -> Content -> AI -> Fuzzy) for auto-categorization. | `Regex`, `Levenshtein`, `Gemini API`    |
| **03** | [Game & Object Mgr](@/.docs/epic3-game-object-manage.md)       | **Structure.** Multi-game management & dynamic category schemas (Character, Weapon, UI).             | `SQLite Relational`, `Web Worker`       |
| **04** | [Folder Grid Explorer](@/.docs/epic4-foldergrid-manage.md)     | **Interface.** Hybrid file navigation (Explorer-style) with virtualization & thumbnail caching.      | `Virtual Grid`, `L1/L2 Cache`           |
| **05** | [Core Operations](@/.docs/epic5-core-mod-manage.md)            | **Action.** Atomic execution of toggle (Enable/Disable), Import, and Rename operations.              | `std::fs::rename`, `Atomic Transaction` |
| **06** | [Preview & INI Editor](@/.docs/epic6-previewpanel.md)          | **Control.** Detail panel, image gallery, and `.ini` configuration editor (Keybinding/Var).          | `Rust INI Parser`, `Image Slider`       |
| **07** | [Privacy Mode (Safe)](@/.docs/epic7-privacy-mode.md)           | **Protection.** Exclusive SFW/NSFW switch with PIN encryption & total isolation.                     | `Argon2`, `Bulk SQL Update`             |
| **08** | [Virtual Collections](@/.docs/epic8-collection.md)             | **Loadouts.** Mass mod presets with an instant Snapshot & Undo system.                               | `JSON Snapshot`, `State Restore`        |
| **09** | [Duplicate Scanner](@/.docs/epic9-duplicate-scan.md)           | **Optimization.** Hash & structural analysis to remove file duplicates.                              | `Rayon Parallel Hash`, `BLAKE3`         |
| **10** | [QoL Automation](@/.docs/epic10-qol.md)                        | **Convenience.** Integrated game launcher (Admin), Randomizer (Gacha Mod), & Pinning.                | `std::process::Command`, `Rand`         |
| **11** | [Settings Infrastructure](@/.docs/epic11-settings.md)          | **Foundation.** Global configuration, game path management, & maintenance tools.                     | `Serde JSON`, `Atomic Write`            |
| **12** | [System Updates](@/.docs/epic12-system-update.md)              | **Evolution.** Application self-updater & database metadata synchronization (Characters/Weapons).    | `Tauri Updater`, `GitHub Raw`           |
| **13** | [Dashboard & Analytics](@/.docs/epic13-dashboard.md)           | **Insight.** Main page with usage statistics & quick access.                                         | `SQL Aggregation`, `Recharts`           |

---

## 🏗️ Technical Architecture Context

### 1. The Hybrid State Model (Performance vs Truth)

We do not trust a single source of truth.

- **Physical Truth (Disk):** Folders with the `DISABLED ` prefix are the sole determinant of mod status in the eyes of the game.
- **Logical Truth (DB):** SQLite acts as a _High-Speed Index_ for the UI. All physical operations (Rename) must be synchronized to the DB via `Watchdog` or `Scan`.
- **Portable Truth (JSON):** Each mod has its own `info.json` so that metadata (Author, Tags) survives when files are moved between PCs.

### 2. The Deep Matcher Pipeline (Intelligent Ingestion)

No more "Unknown" folders. Our system dissects every new mod folder:

1.  **Level 1 (Name):** Does the folder name match the Character Database?
2.  **Level 2 (Content):** Are there unique `.ini` or `.dds` files inside?
3.  **Level 3 (AI):** (Optional) LLM-based structure analysis for complex patterns.
4.  **Level 4 (Fuzzy):** "Raiden" ≈ "Raiden Shogun".

### 3. Atomic Safety Protocols

- **Crash-Proof Writes:** No configuration files are partially written. We write to `.tmp` then rename.
- **Transactional Toggles:** Changing the status of 100 mods (Preset) is done in a single database transaction. If one fails, all are cancelled (Rollback).
- **Non-Destructive Deletion:** "Delete" means moving to the application's Trash folder, not permanent deletion.

### 4. Zero-Compromise UI/UX

- **Virtualization:** Render 10,000 mods without any lag using `@tanstack/react-virtual`.
- **Optimistic UI:** The UI changes color/status _before_ the disk operation completes (with a rollback mechanism if it fails), creating the illusion of instant speed.
- **Native Dark Mode:** Color themes are calibrated for eye comfort (Dracula-based), not just simple color inversion.

---

> _This document governs all development. Every PR (Pull Request) or new feature must refer to one of the Epics above._
