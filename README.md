# EMMM2 Mod Manager

**_Premium Mod Orchestrator for the 3DMigoto Ecosystem_**

> Zero-Compromise. Native Performance. Data Safety. Premium Aesthetics.

![Tauri](https://img.shields.io/badge/Tauri-v2-orange?style=flat-square&logo=tauri)
![Rust](https://img.shields.io/badge/Rust-Backend-black?style=flat-square&logo=rust)
![React](https://img.shields.io/badge/React-v19-blue?style=flat-square&logo=react)
![SQLite](https://img.shields.io/badge/SQLite-Database-003B57?style=flat-square&logo=sqlite)

**EMMM2** is a high-performance, intelligent mod manager designed for 3DMigoto-based games (Genshin Impact, Honkai: Star Rail, Zenless Zone Zero, Wuthering Waves, and Arknights: Endfield). EMMM2 bridges the gap between raw filesystem operations and the modern user's expectation for speed, safety, and visual excellence.

---

## 🛑 1. Core Axioms (Absolute Truths)

EMMM2 is built on five non-negotiable architectural principles documented in `AGENT.md`:

1.  **Filesystem is Truth:** Folder prefix `DISABLED ` is the ONLY source of truth. SQLite is a high-speed index cache.
2.  **Atomic Operations:** Bulk actions (Toggles, Collections) are transactional. Heavy I/O is guarded by a global `OperationLock`.
3.  **Soft Deletion:** Never hard delete user data. Removals move to `./app_data/trash/` with collision detection.
4.  **Scale First:** Virtualization is MANDATORY for all grids/lists > 50 items to maintain 60fps.
5.  **Rust Compute:** Heavy logic (Scanning, Hashing, INI Parsing) is offloaded to Rust `tauri::command` to keep the UI thread free.

---

## ✨ 2. Comprehensive Capabilities

### 🔍 Intelligence & Management
- **Deep Matcher Engine**: A multi-layered pipeline (Name -> Content -> AI -> Fuzzy) that automatically identifies unorganized mods against a global `schema.json`.
- **Byte-Level Deduplication**: Uses BLAKE3 hashing to identify bit-identical mods across your entire library, reclaiming disk space instantly.
- **Archive Native Support**: Deep-scan and import directly from `.zip`, `.7z`, and `.rar` (including password-protected) without manual extraction.
- **Conflict Reporting**: Intelligent detection of overlapping mod files with a guided resolution modal.

### 🌐 Discovery & Integration
- **GameBanana Discover Hub**: Integrated semantic search for the GameBanana ecosystem with smart-import capabilities and version tracking.
- **Integrated Download Manager**: Handles concurrent mod downloads with pause/resume support and auto-categorization upon completion.
- **Automated Update Engine**: "Zero Cost" updates via GitHub Releases for both the core application and the MasterDB schema.

### ⚡ Performance & Discovery
- **60fps Virtual Explorer**: Effortlessly browse 10,000+ mods using `@tanstack/react-virtual` with zero input lag or UI stutters.
- **Real-Time Watcher**: Integrated `notify` v7 service detects external Explorer changes and updates the UI instantly via optimistic sync.
- **Advanced Navigation**: Schema-driven Sidebar for filtering by Game, Category (Character, Weapon, UI), Element, and Rarity.
- **One-Click Play**: Game-specific launch bar with admin-elevation support for 3DMigoto loaders.

### 🎨 Visuals & Editing
- **Lossless INI Parser**: A custom line-based parser that handles 3DMigoto’s non-standard syntax (duplicate sections, naked globals) for safe editing.
- **Rich Preview Gallery**: Auto-detects screenshots, previews, and GIF thumbnails within mod folders for a premium visual experience.
- **Metadata Enrichment**: Enhance mod records with JSON tags, author info, and dynamic keybinding extracted from `d3dx.ini`.

### 🛡️ Privacy & Reliability
- **Safe Mode PIN Gate**: Total isolation for sensitive content (NSFW/Privacy). Frontend/Backend filters enforced via Argon2-secured PIN verification.
- **Collections & Snapshots**: Create virtual loadouts. Snapshot your entire mod list state and restore it instantly with transactional safety.
- **Loadout Randomizer**: Experiment with your collection by generating random mod combinations within specific categories.
- **In-Game Key Viewer**: Quick reference for active mod hotkeys and 3DMigoto mappings without leaving the game.

### 📊 Dashboard & Analytics
- **Global Overview**: Real-time stats on total mod counts, disk usage, and duplicate waste.
- **Visual Analytics**: Interactive `Recharts` distribution charts (Pie: Categories, Bar: Per-Game distribution).
- **Activity Hub**: "Recently Added" feed and "Quick Play" resume shortcuts.

---

## 🎨 3. Design Philosophy

EMMM2 is a **Premium Orchestrator**. We prioritize visual excellence and tactile feedback:
- **Glassmorphism UI**: Modern, translucent interfaces with smooth layout transitions (Framer Motion).
- **Responsive & Alive**: Hover-aware interactive elements and micro-animations for enhanced engagement.
- **Zero Placeholder Policy**: Demonstrations use high-fidelity generated assets or real mod previews.
- **Micro-Interactions**: Subtle, non-intrusive feedback for all file-system and state mutations.

---

## 🏗️ 3. Tech Stack & Architecture

### Backend (Core Logic - Rust)
- **Runtime:** [Tauri v2](https://v2.tauri.app/) for native OS integration and security.
- **Async Runtime:** [Tokio](https://tokio.rs/) for high-concurrency I/O.
- **Database:** SQLite via [SQLx](https://github.com/launchbadge/sqlx) with compile-time query verification.
- **File Monitoring:** [Notify v7](https://github.com/notify-rs/notify) for real-time filesystem synchronization.
- **Security:** Integrity-checked execution and OS Keychain integration via `keyring`.

### Frontend (UI/UX - React)
- **Framework:** [React v19](https://react.dev/) + [TypeScript v5](https://www.typescriptlang.org/).
- **Styling:** [Tailwind CSS v4](https://tailwindcss.com/) + [daisyUI 5](https://daisyui.com/) (Design System).
- **Virtualization:** [@tanstack/react-virtual](https://tanstack.com/virtual) for 10k+ row rendering.
- **State Management:** [Zustand](https://github.com/pmndrs/zustand) (Global) & [TanStack Query](https://tanstack.com/query) (Server Cache).
- **Motion:** [Framer Motion](https://www.framer.com/motion/) for premium micro-animations.

### Orchestration & Tooling
- **Agentic Dev:** Unified `.agent`, `.opencode`, and `.github` layers for advanced AI-pair programming.
- **Quality:** Strict TDD workflow with [Vitest](https://vitest.dev/) and Rust unit/integration tests.

---

## 🛡️ 5. Security & Safety

- **Safe Mode isolation**: NSF-aware partitioning of files and database records.
- **OS Keychain Integration**: Encrypted storage for API keys (e.g., GameBanana, OpenAI) via native system `keyring`.
- **Argon2 Hashing**: High-entropy PIN security for administrative gates.
- **SHA1 Mod Identification**: Robust, collision-resistant mod tracking based on deep folder hashing.

---

## 🚀 4. Development Workflow

### Prerequisites
- **Node.js** (v20+) & **pnpm** (v9+)
- **Rust** (Stable) & **Cargo**
- **Administrator Privileges**: Required for PowerShell run-as-admin game launching and symbolic link operations.

### Installation
1. `pnpm install` — Install frontend and tooling dependencies.
2. `pnpm tauri dev` — Start the application in development mode with HMR.

### Essential Commands
| Command | Layer | Description |
| :--- | :--- | :--- |
| `pnpm tauri dev` | Full | Start dev server with native bridge |
| `pnpm tauri build` | Full | Build production-ready binaries |
| `pnpm test` | FE | Run unit tests via Vitest |
| `pnpm test:ui` | FE | Interactive test runner with UI |
| `pnpm test:coverage`| FE | Generate test coverage report |
| `pnpm lint` | FE | Check code style via ESLint |
| `pnpm format` | FE/Tool | Auto-format with Prettier |
| `cargo test` | BE | Run Rust unit and integration tests |
| `cargo clippy` | BE | Run Rust static analysis (Linter) |
| `cargo fmt` | BE | Format Rust source code |

---

> Built with ❤️ for the 3DMigoto Community. Sync standards maintained by Antigravity Agent.
