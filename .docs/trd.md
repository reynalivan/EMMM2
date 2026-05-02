# Technical Requirements Document (TRD): EMMM Mod Manager

**Version:** 1.0.0-Stable **Target Platform:** Windows (Primary) **Architecture:** Rust (Backend) + React (Frontend)

---

## 1. System Architecture Overview

### 1.1 High-Level Tech Stack

This application uses a _hybrid_ architecture that combines _native_ execution speed with the flexibility of a web interface.

| Layer          | Technology     | Version | Justification                                               |
| -------------- | -------------- | ------- | ----------------------------------------------------------- |
| **Runtime**    | **Tauri**      | v2.0+   | Lightweight, secure, uses WebView2 (Edge Chromium).         |
| **Backend**    | **Rust**       | Stable  | Memory safety, high _concurrency_ for I/O, fastest hashing. |
| **Frontend**   | **React**      | v19+    | The most mature UI library with a broad ecosystem.          |
| **Language**   | **TypeScript** | v5+     | Type safety to prevent bugs on the frontend.                |
| **Database**   | **SQLite**     | v3+     | Structured relational local storage (SQLx).                 |
| **Build Tool** | **Vite**       | Latest  | Instant HMR and build optimization.                         |

### 1.2 Library & Dependencies (Production Grade)

#### **Frontend (User Interface)**

- **Styling:** **daisyUI 5** + **Tailwind CSS 4** (Theming & Layout).
- **State Management:** **Zustand** (Global App State: Safe Mode, Active Game).
- **Async State:** **TanStack Query** (Caching layer between Rust & React).
- **Localization:** **i18next** + **react-i18next** + **i18next-browser-languagedetector** (Namespace-based runtime localization for EN/ID/ZH).
- **Virtualization:** **@tanstack/react-virtual** (**CRITICAL**: Render 10k+ rows @ 60fps).
- **Data Grid:** **TanStack Table** (Headless UI for report tables).
- **Forms:** **React Hook Form** + **Zod** (Metadata & settings input validation).
- **Drag & Drop:** **dnd-kit** (For UI reordering and category changes).
- **Icons:** **Lucide React** (Vector icons).
- **Animations:** **Framer Motion** (Micro-animations and layout transitions).

#### **Backend (Core Logic - Rust Crates)**

- **Database:** **`sqlx`** (Async SQLite with compile-time query verification).
- **Async Runtime:** **`tokio`** (For non-blocking I/O operations).
- **File Watcher:** **`notify`** v7 (Real-time file monitoring with `RecommendedWatcher`).
- **Archive:** Pure Rust implementation using **`zip`** v2, **`sevenz-rust`** v0.6, and **`rar`** v0.4. Supports magic byte detection and password-protected archives. No C-dependencies.
- **Image Proc:** **`image`** (Resize & Convert to WebP thumbnails).
- **Hashing:** **`blake3`** (Super-fast content hashing for deduplication).
- **System Ops:** Custom soft-delete to `./app_data/trash/` (Safe Trash System).
- **Plugins:** `tauri-plugin-single-instance`, `tauri-plugin-dialog`, `tauri-plugin-fs`, `tauri-plugin-log`.
- **Security:** **`keyring`** (OS Keychain for API Keys).

---

## 2. Global Architectural Principles

EMMM is governed by **42 detailed Requirement Specifications** (`req-*.md`). All development must adhere to the following absolute truths:

### 2.1 The Filesystem is the Source of Truth

- The database is merely a high-speed **Index Cache**.
- A mod is `DISABLED` if and only if its physical folder name starts with the `DISABLED ` prefix (with a trailing space).
- If the database desyncs from the filesystem, the filesystem wins.

### 2.2 Atomic Operations & Concurrency Safety

- Destructive file operations (Toggle, Rename, Import, Delete, Safe Mode Switch) must be guarded.
- Operations on multiple items (Bulk Toggle, Collections Apply) must be transactional: all succeed or all rollback.
- Global `OperationLock` using `tokio::sync::Mutex<()>` prevents race conditions during heavy I/O.

### 2.3 The Hybrid State Model & Watchdog

- `notify` crate monitors `/Mods` for external user changes (Explorer).
- In-app operations suppress the File Watcher to avoid infinite feedback loops.
- Watcher, refocus, and Mods-entry refreshes must go through **Disk Reconcile** (`reconcile_disk_state_cmd`) to keep the DB projection aligned with the filesystem.
- Explicit scanning/import flows must go through **Deep Match Scanner** (`deepmatch_preview_cmd`, `deepmatch_scanner_cmd`).
- All operations update the UI optimistically before the Rust backend completes the I/O.

### 2.4 Maximum Frontend Alignment (Rust Offloading)

- **Decoupled Business Logic**: React runs exclusively as a Presentation and Remote Cache layer.
- **Heavy Compute**: Intensive Client-Side operations (e.g., Dynamic Programming-based fuzzy matching in `useMasterDbSync.ts`) MUST be offloaded to Rust `tauri::command`s. Block the UI thread as little as possible.
- **Data Access Layer**: Rust backend must strictly separate DB concerns (Repositories) from Business Logic (Services) to guarantee isolated Schema & Query Optimization.

### 2.5 Localization as a Build-Time Policy

- **Zero-Hardcode UI Rule**: All user-facing frontend copy (labels, titles, empty states, placeholders, tooltips, button text, dialog copy, aria-labels, and toast messages) MUST resolve from i18n namespaces, not inline literals or `defaultValue` production fallbacks.
- **Namespace Layout**: Locale resources are file-based and must follow `src/locales/<lang>/<namespace>.json` with nested keys and explicit namespace prefixes where required.
- **Primary Locale**: `en` is the canonical source locale. `id` and `zh` must mirror the same key structure.
- **Tooling Enforcement**: `i18next-cli` is the repository-level extractor/auditor used to lint hardcoded strings and sync secondary locale structure.

---

## 3. Core Domain Models (Database Schema Overview)

> Detailed schema specs reside in `req-09-object-schema.md` and `req-10-object-crud.md`.

### 3.1 `games` (Game Instances)

UUID-based tracking of installed games, their `game_type` (GIMI, SRMI, etc.), and paths to their `/Mods` directories.

### 3.2 `objects` (Categorical Grouping)

Virtual containers representing a Character, Weapon, UI, or Other entity derived from the Master Schema JSON. A single object contains many `mods`.

### 3.3 `mods` (The Mod Entries)

Stable identifiers using the **SHA1 hash** of the relative path. Stores the `actual_name`, `folder_path`, `thumbnail_path`, `is_safe` flag, and aggregated JSON `metadata`.

### 3.4 `collections` (Virtual Presets)

Atomic groupings of enabled mods (loadouts) that can be applied in a single click with snapshot/undo support.

---

## 4. Core Functional Pipelines

### 4.1 Deep Match Scanner Engine (`req-26`)

The explicit canonical matching pipeline of the application running asynchronously:

1. **Hash & Strict Alias:** Checks 3DMigoto `.ini` hashes and exact folder names against MasterDB.
2. **Deep Content Scan:** Tokenizes subfolders, file stems, and INI strings for substring matching.
3. **AI & Mechanical Reranks:** Uses GameBanana API and optional LLM context for ambiguous cases.
   | AC-26.4.4 | ⚠️ Edge | Strict thresholds prevent false-positive auto-matching. Ambiguous items require UI Review. |
   | AC-26.4.5 | ✅ Positive | **Storage Optimizer (`req-32`):** A parallel BLAKE3 scanner identified duplicate assets across mod folders, allowing resolution via NTFS Hardlinks or Trash with a persistent whitelist. |

### 4.2 Custom INI Parser (`req-18`)

Standard INI parser crates cannot be used because 3DMigoto syntax is unique (duplicate sections, naked global variables, inline comments). EMMM uses a custom Line-Based parser for **Lossless Editing**.

### 4.3 Safe Mode Filter (`req-30`)

- **Frontend Guard:** When `Zustand: safeMode = ON`, UI actively filters `is_safe: false` data.
- **Backend Guard:** Rust SQL queries dynamically append `AND is_safe = 1` preventing data leakage at the source. Requires OS Keychain verification to disable.

### 4.4 Modern Archive Extraction (`req-37`)

- **Pure Rust Architecture**: Completely eliminated C-dependent libraries (libarchive/compress-tools) to ensure lightning-fast, zero-bottleneck compilation.
- **Robust Format Detection**: Uses Magic Byte (file signature) detection (ZIP, 7z, RAR4, RAR5) with file extension fallback for 100% reliable identification.
- **Atomic Cleanup (RAII)**: Implements `TempDirGuard` to ensure `.temp_extract/` subfolders are automatically wiped on any error, cancellation, or panic.
- **IPC Performance**: Throttled progress streaming (250ms interval) prevents frontend IPC flooding while maintaining smooth progress bars.

---

## 5. Security & Deployment

- **Zero Cost Updater (`req-34`):** Uses Tauri Updater pointing to GitHub Releases. `schema.json` and MasterDB updates are fetched incrementally from raw GitHub URLs.
- **Log Rotation:** `tauri-plugin-log` configured for max 5 files, 2MB each, stored in `%AppData%`.
- **API Keys:** User AI API keys are stored in the secure OS keychain (`keyring` crate), never in plaintext databases.

---

## 6. Project Structure

**Root Project:** `EMMMNEW/`

```text
EMMMNEW/
├── .docs/                       # Documentation & Requirements
│   ├── requirements/            # req-01 to req-43 (The absolute source of truth)
│   ├── workflows/               # Automation standards
│   └── rules/                   # Coding standards
├── .github/                     # CI/CD Workflows
├── src-tauri/                   # [BACKEND - RUST CORE]
│   ├── migrations/              # SQLx Migrations
│   ├── src/
│   │   ├── commands/            # Tauri IPC Endpoints
│   │   ├── database/            # Data Access Layer (Repositories)
│   │   ├── services/            # Business Logic (Decoupled from Schema)
│   │   │   ├── scanner/         # req-25 (Scan Engine), req-26 (Deep Match Scanner)
│   │   │   ├── disk_reconcile/  # runtime filesystem reconciliation (watcher/focus/mods-entry)
│   │   │   ├── mod_files/       # req-13 (Core Ops), req-22 (Trash), req-37 (Archives)
│   │   │   ├── ini/             # req-18 (INI Parser)
│   │   │   ├── collections/     # req-31 (Collections & Presets)
│   │   │   ├── privacy/         # req-30 (Safe Mode Filter)
│   │   │   └── update/          # req-34 (Zero Cost Updater)
│   │   ├── types/               # Shared Rust types & models
│   │   ├── lib.rs               # App Builder & setup
│   │   └── main.rs              # App Entry Point
│   ├── tauri.conf.json          # Desktop permissions & setup
│   └── Cargo.toml
├── src/                         # [FRONTEND - REACT UI]
│   ├── components/              # UI Components (Atomic Design)
│   ├── features/                # Domain Driven Components
│   │   ├── collections/         # req-31 (Collections UI)
│   │   ├── dashboard/           # Home & Landing views
│   │   ├── foldergrid/          # req-11, req-12 (Main Mod Listing)
│   │   ├── objectlist/          # req-06, req-07 (Object Navigation)
│   │   ├── onboarding/          # First-time App Setup wizard
│   │   ├── preview/             # req-16, req-17, req-18 (Side Panel)
│   │   ├── scanner/             # Dedup/Scanner Dashboard
│   │   └── settings/            # App configuration & preferences
│   ├── hooks/                   # Custom business logic hooks
│   ├── locales/                 # i18n resources: <lang>/<namespace>.json
│   ├── stores/                  # Global State (Zustand: appStore, sessionStore)
│   ├── App.tsx                  # Main Layout & Routing
│   └── index.css                # Tailwind Directives
├── i18next.config.ts            # i18next-cli extractor/sync configuration
└── package.json
```

## 7. Quality Assurance & Automation

To ensure EMMM maintains stability across its hybrid architecture, the following testing layers are enforced:

### 7.1 Backend Unit & Integration Tests (Rust)

- Framework: `cargo test`
- Scope: Database operations (SQLx), file system watching, lock contention (`OperationLock`), JSON parsing, and archive extraction.
- Edge Cases: Focus on path traversals, disk space limits, and concurrent mutation rollbacks.

### 7.2 Frontend Component Tests (React)

- Framework: `vitest` + `@testing-library/react`
- Scope: React component rendering, Zustand store logic, TanStack Query caching, and UI focus management (e.g., Virtualizer arrow navigation).
- Mocks: `vi.mock('../../lib/bindings')` to mock the typed `commands` object (Specta-generated from `bindings.ts`). Raw `window.__TAURI_INTERNALS__` mocking is prohibited.

### 7.3 Localization Verification

- Commands:
  - `pnpm i18n:lint`
  - `pnpm i18n:sync`
  - `pnpm i18n:status`
- `i18n:lint` must pass before merge for touched frontend areas.
- `i18n:sync` is structural only; it propagates keys from `en` to secondary locales but does not author translations.
- `i18n:status` may fail on pre-existing untranslated namespaces, but any newly introduced or modified feature key must be present in `en`, `id`, and `zh`.

### 7.4 End-to-End (E2E) Testing

- Framework: **WebdriverIO** with **Tauri WebDriver** native integration.
- Scope: High-level user journeys crossing the typed IPC bridge (e.g., clicking 'Enable' in the React UI and verifying the actual folder rename on the host OS).
- CI/CD: Automated native build tests via GitHub Actions.

> **Note:** For granular implementation details on any specific feature, always consult the corresponding `req-*.md` file.
