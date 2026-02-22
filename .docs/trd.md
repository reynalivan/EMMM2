# Technical Requirements Document (TRD): EMMM2 Mod Manager

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
| **Database**   | **SQLite**     | v3+     | Structured relational local storage.                        |
| **Build Tool** | **Vite**       | Latest  | Instant HMR and build optimization.                         |

### 1.2 Library & Dependencies (Production Grade)

#### **Frontend (User Interface)**

- **Styling:** **daisyUI 5** + **Tailwind CSS 4** (Theming & Layout).
- **State Management:** **Zustand** (Global App State: Safe Mode, Active Game).
- **Async State:** **TanStack Query** (Caching layer between Rust & React).
- **Virtualization:** **TanStack Virtual** (**CRITICAL**: Render 10k+ rows @ 60fps).
- **Data Grid:** **TanStack Table** (Headless UI for report tables & settings).
- **Forms:** **React Hook Form** + **Zod** (Metadata & settings input validation).
- **Charts:** **Recharts** (Dashboard statistics visualization).
- **Icons:** **Lucide React** (Vector icons).

#### **Backend (Core Logic - Rust Crates)**

- **Database:** **`sqlx`** (Async SQLite with compile-time query verification).
- **Async Runtime:** **`tokio`** (For non-blocking I/O operations).
- **File Watcher:** **`notify`** v7 (Real-time file monitoring with `RecommendedWatcher`).
- **Serialization:** **`serde`** + **`serde_json`** (Parsing JSON config).
- **Archive:** **`zip`** v2 (ZIP), **`sevenz-rust`** v0.6 (7z), **`rar`** v0.4 (RAR). All pure Rust, no C deps. Password-protected archives supported via `aes-crypto` (zip) and `aes256` (sevenz-rust) features.
- **Image Proc:** **`image`** (Resize & Convert to WebP).
- **Hashing:** **`blake3`** (Super-fast content hashing for duplicate detection).
- **System Ops:** Custom soft-delete to `./app_data/trash/` with metadata-based restore (no `trash` crate).
- **Single Instance:** **`single_instance`** via Tauri plugin (prevent multiple app windows).
- **HTTP Client:** **`reqwest`** (For metadata updates & AI API).
- **Logging:** **`log`** + **`tauri-plugin-log`**.

---

### 1.3 Testing Strategy (Quality Assurance)

The testing standard follows the TDD (_Test Driven Development_) approach to ensure stability at every layer.

| Layer        | Type               | Tool                      | Focus                                                                   |
| :----------- | :----------------- | :------------------------ | :---------------------------------------------------------------------- |
| **Backend**  | Unit & Integration | **`cargo test`**          | Core Rust logic (Scanner, Parser, Hashing).                             |
| **Backend**  | Async Logic        | **`tokio::test`**         | Testing async I/O functions and Command handlers.                       |
| **Database** | Integration        | **`sqlx::test`**          | SQL query verification with a temporary in-memory database.             |
| **Frontend** | Unit & Component   | **Vitest**                | Fast testing framework replacing Jest (Native Vite support).            |
| **Frontend** | User Interaction   | **React Testing Library** | Simulating user clicks/inputs on UI components.                         |
| **System**   | End-to-End (E2E)   | **Playwright**            | Simulating full flow (Install -> Launch -> Modding) on the Desktop app. |

---

## 2. Database Schema Design (SQLite)

The database serves as an _Index Cache_. The UI does not read physical folders directly; instead, it reads this table.

### 2.1 Table: `games` (Configuration)

Stores game instance configurations.

```sql
CREATE TABLE games (
    id TEXT PRIMARY KEY,              -- UUID v4
    name TEXT NOT NULL,               -- e.g., "Genshin Impact"
    game_type TEXT NOT NULL,          -- Enum: GIMI, SRMI, WWMI
    path TEXT NOT NULL,               -- Absolute path to /Mods
    launcher_path TEXT,               -- Path to 3DMigoto Loader
    launch_args TEXT,                 -- Custom arguments
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 2.2 Table: `mods` (Main Index)

Stores the state and metadata of each mod folder.

```sql
CREATE TABLE mods (
    id TEXT PRIMARY KEY,              -- blake3 hash of the relative path (Stable ID)
    game_id TEXT NOT NULL,            -- FK -> games.id
    actual_name TEXT NOT NULL,        -- Clean name (without DISABLED prefix)
    folder_path TEXT NOT NULL,        -- Physical absolute path
    status TEXT DEFAULT 'DISABLED',   -- 'ENABLED' | 'DISABLED'
    is_pinned BOOLEAN DEFAULT 0,      -- Pin feature
    is_safe BOOLEAN DEFAULT 0,        -- Safe Mode feature
    last_status_active BOOLEAN,       -- State snapshot for Safe Mode toggle
    size_bytes INTEGER,               -- Folder size (for Duplicate Scan)
    object_type TEXT,                 -- 'Character', 'Weapon', 'UI'
    metadata_blob JSON,               -- Full info.json cache
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
);
```

### 2.3 Table: `collections` & `collection_items` (Presets)

Stores Virtual Collections data.

```sql
CREATE TABLE collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    game_id TEXT NOT NULL,
    is_safe_context BOOLEAN DEFAULT 0,  -- SFW/NSFW context filtering
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE collection_items (
    collection_id TEXT NOT NULL,
    mod_id TEXT NOT NULL,
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY(mod_id) REFERENCES mods(id) ON DELETE CASCADE
);
```

---

## 3. Core Functional Logic (Backend Requirements)

### 3.1 Advanced Logging System (Log Rotation)

The logging system must be _robust_ for debugging on the user side without filling up the disk.

- **Plugin:** `tauri-plugin-log`.
- **Config:**
  - **Rotation:** Keep the last 5 files.
  - **Max Size:** 2MB per file.
  - **Path:** `%AppData%/EMMM2/logs/`.
  - **Format:** `[TIMESTAMP] [LEVEL] [MODULE] Message`.

- **Levels:**
  - `INFO`: Normal operation (App Start, Game Switch).
  - `WARN`: Invalid mod, Failed to load thumbnail.
  - `ERROR`: Failed to rename file, Database locked.

### 3.2 Deep Matcher Engine (Porting Logic)

The "Brain" logic of the application running in a _background thread_.

1.  **Normalization:** Regex replace symbols -> Lowercase -> Unidecode (Rust crate `deunicode`).
2.  **Pipeline Check:**
    - _L1 Name Match:_ `str::contains` vs DB Aliases.
    - _L2 Token Match:_ Intersection of folder token `HashSet<String>` vs DB Tags.
    - _L3 Content Scan:_ `WalkDir` (crate) max depth 3 -> search for `.ini`/`.ib` files -> Match filename vs DB.
    - _L4 Fuzzy:_ Levenshtein distance (crate `strsim`).

### 3.3 Custom INI Parser (Rust Implementation)

A standard INI parser crate cannot be used because 3DMigoto syntax is unique.

- **Requirement:** Must support _Variables_ outside sections (`$var = 1`) and duplicate _Sections_ (`[TextureOverride...]`).
- **Method:** _Lossless Editing_.
  1.  Read the file as `Vec<String>` (Lines).
  2.  Identify the target line (e.g., line 50 `key = ...`).
  3.  Replace that line.
  4.  Rewrite `Vec<String>` back to the file.

- **Backup:** Always copy the original file to `.ini.bak` before writing.

### 3.4 Smart Duplicate Scanner

Rust optimization for speed.

1.  **Filtering:** SQL query `SELECT path, size_bytes FROM mods WHERE game_id = ? ORDER BY size_bytes`.
2.  **Grouping:** Group mods with a size difference < 1%.
3.  **Hashing:** Use `blake3` (Multi-threaded hashing).
    - Hash `.ini` files (excluding whitespace/comments).
    - Hash a 4KB header of the largest `.dds` file.

### 3.5 File Watcher Conflict Avoidance

The `notify` crate watches `/Mods` for external changes, but EMMM2's own operations (toggle rename, import, delete) also produce file events. Without suppression, these cause infinite feedback loops.

- **Suppression Strategy (In-App Action Guard):**
  1.  Before any in-app file operation, register the target path(s) in a shared `suppressions: Arc<Mutex<HashSet<PathBuf>>>`.
  2.  Execute the operation (`fs::rename`, `fs::create_dir`, etc.).
  3.  The watcher's event handler checks `suppressions`. If the path exists → **skip** the event (do NOT update DB or emit to frontend).
  4.  After a 500ms debounce, remove the path from `suppressions`.

- **Debounce Strategy (External Change Batching):**
  - All external events are batched in a 300ms window using `tokio::time::sleep`.
  - Batch is processed as one DB sync, not N individual updates.

- **Event Types & Behavior:**

  | Event    | Source                              | Action                                          |
  | -------- | ----------------------------------- | ----------------------------------------------- |
  | `Create` | External (user copies folder)       | Insert new mod into DB, emit `MOD_ADDED`        |
  | `Rename` | In-app (toggle/rename)              | **Suppressed** → no action                      |
  | `Rename` | External (user renames in Explorer) | Update `folder_path` and `actual_name` in DB    |
  | `Remove` | External (user deletes in Explorer) | Mark mod as `DELETED` in DB, emit `MOD_REMOVED` |
  | `Remove` | In-app (trash/delete)               | **Suppressed** → no action                      |
  | `Modify` | External (user edits ini/json)      | Re-parse affected file, update `metadata_blob`  |

### 3.6 Operation Queue Lock (Concurrency Safety)

Destructive file operations must not run concurrently.

- **Implementation:** A global `OperationLock` using `tokio::sync::Mutex<()>`.
- **Protected Operations:** Toggle, Rename, Import, Delete, Safe Mode Switch, Collection Apply.
- **Behavior:**
  - If a user triggers a protected operation while another is running → return `Err("Operation in progress. Please wait.")` and show a toast.
  - The lock is acquired at the **Command** layer, not the **Service** layer, to keep services reusable.
  - Lock has a 30s timeout via `tokio::time::timeout` to prevent deadlocks.

---

## 4. Frontend Implementation Specifications

### 4.1 Grid Virtualization Logic

Implementation of `TanStack Virtual` on the `FolderGrid` component.

- **Measure:** Use a fixed `estimateSize` (e.g., 280px for mod cards) for the best scroll performance.
- **Overscan:** Set `overscan: 5` (render 5 rows off-screen) so images are ready before the user scrolls.

### 4.2 Dashboard Charts

Implementation of `Recharts` on `HomeDashboard`.

- Data is retrieved via `TanStack Query` which calls the Tauri command `get_dashboard_stats`.
- The Rust backend performs `COUNT()` and `SUM()` aggregations via SQL to minimize data transfer to the frontend.

### 4.3 Drag & Drop Handler

- Listen for global `tauri://file-drop` events.
- If the file extension is `.zip/.rar/.7z` -> Trigger "Smart Import" modal.
- The frontend displays a visual _Overlay Zone_ when a file is dragged over the window.

---

## 5. Security & Deployment

### 5.1 Zero Cost Updater Workflow

- **Repo:** GitHub Public Repository.
- **Artifacts:** `EMMM2-setup.exe` in GitHub Releases.
- **Metadata:** `db_char.json`, etc., in the `main` branch (access via `raw.githubusercontent.com`).
- **Tauri Updater:** `tauri.conf.json` configuration points to a static JSON endpoint on GitHub containing the update signature.

### 5.2 Safe Mode Implementation

- **Data Guard:** When `Safe Mode = ON` (in Zustand Store), the Frontend actively filters `is_safe: false` data so it is never rendered to the DOM.
- **Backend Guard:** The Tauri Command `get_mods` receives a `safe_mode_active` parameter. If true, the SQL Query automatically adds `AND is_safe = 1`.

---

## 6. Project Structure

### Foundation

- Setup Tauri v2 + React + daisyUI template.
- Database configuration (SQLite) & initial migrations.
- Logger implementation (`tauri-plugin-log`).

### Structure

**Root Project:** `EMMM2NEW/`

```text
EMMM2NEW/
├── .github/                     # CI/CD Workflow (Auto-build Release)
├── .vscode/                     # Debug configuration for Rust + React
├── src-tauri/                   # [BACKEND - RUST CORE]
│   ├── migrations/              # SQLx Migrations (001_init.sql, etc.)
│   ├── src/
│   │   ├── commands/            # Bridge: Functions called by the Frontend
│   │   │   ├── mod_cmds.rs      # invoke('scan_mods'), invoke('toggle_mod')
│   │   │   ├── game_cmds.rs     # invoke('add_game'), invoke('launch_game')
│   │   │   ├── app_cmds.rs      # invoke('get_system_stats')
│   │   │   └── mod.rs           # Module export
│   │   ├── database/            # Database Layer (SQLx)
│   │   │   ├── connection.rs    # SQLite Connection Pool
│   │   │   ├── repository.rs    # Query logic (SELECT/INSERT)
│   │   │   └── models.rs        # Rust Structs <-> SQL Mapping
│   │   ├── services/            # Business Logic ("The Brain")
│   │   │   ├── scanner/
│   │   │   │   ├── deep_matcher.rs  # Epic 2: Regex & Scoring Logic
│   │   │   │   ├── duplicate.rs     # Epic 9: Hash & Size comparison
│   │   │   │   └── walker.rs        # Efficient File Walking
│   │   │   ├── parser/
│   │   │   │   └── ini_parser.rs    # Epic 5: Custom 3DMigoto INI Parser
│   │   │   ├── file_ops/
│   │   │   │   ├── archive.rs       # Unzip/Unrar logic
│   │   │   │   ├── io.rs            # Atomic Rename/Move/Delete
│   │   │   │   └── trash.rs         # Epic 4: Soft Delete
│   │   │   ├── images/
│   │   │   │   └── thumbnail.rs     # Epic 4: Resize & WebP Cache
│   │   │   └── sync/
│   │   │       └── github.rs        # Epic 9: Update checker
│   │   ├── utils/               # Shared helpers (Hashing, String ops)
│   │   ├── lib.rs               # Library Entry
│   │   └── main.rs              # App Entry Point
│   ├── tauri.conf.json          # Config: Permissions, Icons, Window
│   ├── Cargo.toml               # Rust Dependencies (sqlx, tokio, image, etc)
│   └── build.rs
├── src/                         # [FRONTEND - REACT UI]
│   ├── assets/                  # Static images
│   ├── components/              # UI Components (Atomic)
│   │   ├── ui/                  # Generic UI (Button, Input, Modal - daisyUI)
│   │   └── virtual/             # TanStack Virtual wrappers
│   ├── features/                # Domain Driven Design (By Epic)
│   │   ├── onboarding/          # Epic 1: Welcome & Setup
│   │   ├── dashboard/           # Epic 10: Charts & Summary
│   │   ├── explorer/            # Epic 4: Grid, Filter, Breadcrumbs
│   │   ├── sidebar/             # Epic 3: Object List & Game Switcher
│   │   ├── details/             # Epic 5: Preview, INI Editor
│   │   ├── scanner/             # Epic 2 & 9: Scan Results & Duplicate Report
│   │   └── settings/            # Epic 11: Game Paths, Safe Mode PIN
│   ├── hooks/                   # Custom Hooks
│   │   ├── useTauri.ts          # Type-safe wrapper for invoke()
│   │   ├── useSafeMode.ts       # Safe Mode Auth Logic
│   │   └── useMods.ts           # TanStack Query hooks
│   ├── stores/                  # Global State (Zustand)
│   │   ├── appStore.ts          # UI State (Theme, Sidebar Open)
│   │   └── sessionStore.ts      # Active Game, Filters
│   ├── lib/                     # Utilities
│   │   ├── db-types.ts          # TS Interface matching Rust Models
│   │   └── utils.ts
│   ├── App.tsx                  # Main Layout & Router
│   ├── main.tsx                 # Entry
│   └── index.css                # Tailwind Directives
├── public/                      # Public assets
├── package.json                 # React Dependencies
├── tsconfig.json
├── vite.config.ts
└── tailwind.config.js           # daisyUI Config
```

### Main Structure

#### 1. Backend (`src-tauri/src`)

This is the main "engine" that runs heavy logic (Rust).

- **`services/`**: This is where EMMM2's "brain" logic resides.
  - **`scanner/`**: Contains the _Deep Matcher_ (Epic 2) and _Duplicate Scanner_ (Epic 9) logic. These files handle _multithreading_ and hashing.
  - **`parser/`**: Custom `.ini` parser implementation (Epic 5) that preserves comments and original file structures.
  - **`file_ops/`**: Handles physical file manipulation (Rename, Delete to Trash, Extract Archive) securely.

- **`commands/`**: Bridges the Frontend and Backend. The Frontend calls functions here (e.g., `invoke('scan_mods')`), and this file calls the relevant `services`.
- **`database/`**: Manages the SQLite connection using `sqlx`. Migration files (`.sql`) are stored separately in the `migrations` folder for database versioning.

#### 2. Frontend (`src/`)

This is the user interface built with React.

- **`features/`**: Groups components based on feature (Domain Driven Design). Example: Everything related to the "Preview Panel" is in the `details/` folder. This makes maintenance easier than piling all components in a single folder.
- **`stores/`**: Uses **Zustand** to store lightweight global states, such as "Is Safe Mode active?" or "Which game is currently selected?".
- **`components/common/`**: Basic UI components (Button, Input) wrapped from **daisyUI**. This ensures design consistency throughout the application.

#### 3. Configuration (`Root`)

- **`tailwind.config.js`**: daisyUI theme configuration (Light/Dark/Dracula) and the application's color palette.
- **`tauri.conf.json`**: Manages file system access permissions, window configurations, and desktop application security rules.
