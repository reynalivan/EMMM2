# EMMM2 Mod Manager

> **Premium Mod Orchestrator for the 3DMigoto Ecosystem**

**EMMM2** is a high-performance, intelligent mod manager designed specifically for 3DMigoto-based games (Genshin Impact, Honkai: Star Rail, Zenless Zone Zero, Wuthering Waves, and Arknights: Endfield).

Built to replace tedious manual file management, EMMM2 bridges the gap between raw filesystem operations and the modern user's expectation for speed, safety, and visual excellence.

---

## 🎯 Why EMMM2 Exists

Managing 3DMigoto mods manually involves extracting archives, dealing with deeply nested folders, renaming files to toggle them on/off, and resolving hash conflicts by hand. For users with hundreds or thousands of mods, this becomes unmanageable.

EMMM2 was built with the following core principles to solve these pain points:

1. **Zero Data Loss & Safety First:** Atomic operations, soft-deletion (Trash system), collision detection, and Safe Mode for privacy.
2. **The Filesystem is the Source of Truth:** `DISABLED ` folder prefixes are the sole determinant of mod status. EMMM2 reads what's on disk, ensuring it never desyncs from reality.
3. **Instant Responsiveness:** Render 10,000+ items without lag. Optimistic UI updates provide immediate feedback before disk I/O completes.
4. **Intelligent Automation:** The Deep Matcher pipeline auto-categorizes unstructured mod folders into the correct Characters/Weapons without manual tagging.

---

## 🏗️ Technical Architecture Context

> **Core Stack:** Tauri v2 (Rust Backend), React v19, TypeScript, SQLite (SQLx), Tailwind CSS v4, DaisyUI 5, Zustand, TanStack Query.

EMMM2 is built on a robust hybrid architecture that separates physical reality from logical presentation:

### 1. The Hybrid State Model

- **Physical Truth (Disk):** Mod status is entirely driven by the `DISABLED ` prefix on folders. Fast, recursive filesystem scanning guarantees true representation.
- **Logical Truth (DB):** SQLite acts as a _High-Speed Index_. It stores metadata, object hierarchies, and custom tags for instant filtering and searching.
- **Portable Truth (JSON):** Each mod maintains an `info.json` inside its folder. Metadata (Author, Tags, Element) survives perfectly even if the mod is manually moved to another PC.
- **Decoupled Compute Layer:** The React Frontend is strictly a presentation layer. Heavy computations like DP-based fuzzy matching and database queries are completely offloaded to optimized Rust Services and a distinct Data Access Layer (Repositories).

### 2. The Deep Matcher Pipeline (The Brain)

To categorize raw, messy user folders (e.g., `[V1.2]_Cool_Hu_Tao_Mod_by_Author`), EMMM2 uses a staged, deterministic matching engine:

1. **Quick Hash & Alias:** Scans `.ini` file contents and exact folder names against a bundled `schema.json`.
2. **Deep Content Scan:** Tokenizes subfolders, file stems, and INI keys to build strong evidence.
3. **AI & Mechanical Reranks:** Leverages GameBanana API and optional AI trait-matching for ambiguous cases.
4. **Evidence Gating:** Guarantees zero false positives. Ambiguous matches are sent to a manual Review UI.

### 3. Bulletproof Operations

- **Transactional Toggles:** Changing the status of a 50-mod Collection (Preset) is atomic. If the filesystem blocks one rename, the entire operation is rolled back.
- **Background Watchdog:** The app monitors the filesystem for external changes (renames, deletions outside the app) and gracefully updates the UI with built-in loop prevention.
- **Smart Extraction:** Extracts messy `.rar`/`.zip` files natively via Rust, applying smart-flattening to prevent `Nested/Nested/Mod` folder structures.

### 4. Zero-Compromise UI/UX

- **Virtualized Grids:** `@tanstack/react-virtual` powers both the ObjectList and the main Folder Grid, allowing 60fps scrolling through massive libraries.
- **Immersive Metadata:** Built-in `.ini` editor with syntax highlighting, a lightbox image gallery, and a direct GameBanana metadata fetcher.
- **Premium Aesthetics:** Dark-mode optimized, glassmorphic interfaces, and satisfying micro-animations built strictly with Tailwind and DaisyUI.

---

## 🚀 Scope of Capabilities

EMMM2's capabilities are thoroughly documented across **42 dedicated requirement specifications**, grouped into major feature domains:

- **Bootstrap & Game Management (req-01 to req-05):** Single-instance guards, DB migrations, and auto-discovery of game installations.
- **Object Schema & ObjectList Navigation (req-06 to req-09):** Schema-driven categories, dynamic element/rarity filtering, and a virtualized object tree.
- **Folder Grid & Core Operations (req-10 to req-14):** Thumbnail virtualization, instant search via Web Workers, bulk toggling, and fast renaming.
- **Preview & Metadata Editing (req-16 to req-19):** INI inspection, image gallery auto-detection, and JSON metadata editing.
- **Scan Engine & Storage (req-22 to req-28):** Multi-threaded scanning, Trash safety, deep archive extraction, and deduplication (hashing).
- **Advanced Features (req-30 to req-43):** Privacy Safe Mode, Collections (Presets), Dashboard analytics, Smart Randomizers, and In-Game Hotkey integration.

---

> _This document serves as the high-level introduction to the EMMM2 ecosystem. For specific implementation details, refer to the individual `req-*.md` specification files in the `.docs/requirements` directory._
