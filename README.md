# EMMM2 Mod Manager

**_Premium Mod Orchestrator for the 3DMigoto Ecosystem_**

> _Zero-Compromise. Native Performance. Data Safety. Premium Aesthetics._

![Project Badge](https://img.shields.io/badge/Tauri-v2-orange?style=flat-square&logo=tauri)
![Project Badge](https://img.shields.io/badge/Rust-Backend-black?style=flat-square&logo=rust)
![Project Badge](https://img.shields.io/badge/React-v19-blue?style=flat-square&logo=react)
![Project Badge](https://img.shields.io/badge/SQLite-Database-003B57?style=flat-square&logo=sqlite)

**EMMM2** is a high-performance, intelligent mod manager designed specifically for 3DMigoto-based games (Genshin Impact, Honkai: Star Rail, Zenless Zone Zero, Wuthering Waves, and Arknights: Endfield).

Built to replace tedious manual file management, EMMM2 bridges the gap between raw filesystem operations and the modern user's expectation for speed, safety, and visual excellence.

---

## ✨ Key Principles & Features

### 🛡️ Zero Data Loss & Safety First

Atomic operations, soft-deletion (Trash system), collision detection, and a dedicated, encrypted "Safe Mode" that creates total isolation for sensitive content, toggleable instantly with an Argon2-secured PIN.

### 🧠 Deep Matcher (Intelligent Scanning)

No more "Unknown" folders. Our multi-layered pipeline (Name -> Content -> AI -> Fuzzy) automatically identifies and categorizes unorganized mods against a bundled `schema.json` with precision, ensuring zero false positives.

### ⚡ Zero-Compromise UI/UX (Zero-Lag Virtual Grid)

Built on `@tanstack/react-virtual`, the explorer renders 10,000+ items effortlessly at 60fps. No pagination, no loading screens—just pure speed.

### 🗄️ The Filesystem is the Source of Truth

The SQLite Database is a high-speed index. Mod status is entirely driven by the `DISABLED ` prefix on physical folders. EMMM2 reads what's on disk via transactional toggles, ensuring it never desyncs from reality.

### 📦 Collections & Snapshots

Create virtual loadouts. Snapshot your entire mod list state and restore it instantly. Undo highly destructive operations with a single click.

---

## 🏗️ Tech Stack & Architecture

### Frontend (The Face)

- **Framework:** React v19 + TypeScript
- **Build Tool:** Vite
- **Styling:** Tailwind CSS v4 + DaisyUI 5
- **State & Sync:** Zustand (Global State), TanStack Query (Async Server State)
- **Performance:** TanStack Virtual (Scroll Virtualization)
- **Data Management:** TanStack Table (Headless UI Tables)
- **Forms:** React Hook Form + Zod
- **Icons:** Lucide React

### Backend (The Muscle)

- **Core:** Rust (Tauri v2)
- **Database:** SQLite (via `sqlx`)
- **Async Runtime:** Tokio
- **File Watcher:** `notify` v7 (Real-time monitoring)
- **Security:** `keyring` (OS Keychain integration)

---

## 🚀 Getting Started

### Prerequisites

- **Node.js** (v20+) & **pnpm**
- **Rust** (Latest Stable)
- **VS Code** (Recommended)

### Installation

1.  **Clone the Repository**

    ```bash
    git clone https://github.com/your-username/emmm2.git
    cd emmm2
    ```

2.  **Install Dependencies**

    ```bash
    pnpm install
    ```

3.  **Run Development Server**
    ```bash
    pnpm tauri dev
    ```

---

## 🧪 Development Workflow

We follow **Test-Driven Development (TDD)** and strict architectural rules based on 42 dedicated requirement specifications.

### Commands

| Command             | Description                        |
| :------------------ | :--------------------------------- |
| `pnpm tauri dev`    | Start the app in development mode. |
| `pnpm test`         | Run Frontend Unit Tests (Vitest).  |
| `pnpm test:ui`      | Run Frontend Tests with UI.        |
| `pnpm lint`         | Check for linting errors.          |
| `pnpm format`       | Auto-format code with Prettier.    |
| `pnpm tsc --noEmit` | Run type checking.                 |

### Backend Testing & Linting (Rust)

```bash
cd src-tauri
cargo test
cargo clippy -- -D warnings
```

---

## 🗺️ Scope of Capabilities

EMMM2's capabilities are governed by **42 detailed requirement specifications (`req-01` to `req-43`)** across major domains:

1. **Bootstrap & Game Management (req-01 to req-05):** Single-instance guards, DB migrations, auto-discovery.
2. **Object Schema & ObjectList Navigation (req-06 to req-09):** Schema-driven categories, dynamic element/rarity filtering.
3. **Folder Grid & Core Operations (req-10 to req-14):** Thumbnail virtualization, instant search, bulk toggling, absolute Safe Mode.
4. **Preview & Metadata Editing (req-16 to req-19):** INI inspection, image gallery auto-detection, JSON metadata editing.
5. **Scan Engine & Storage (req-22 to req-28):** Multi-threaded scanning, Trash safety, deep archive extraction, hashing deduplication.
6. **Advanced Features (req-30 to req-43):** Privacy Safe Mode, Collections (Presets), Dashboard analytics, In-Game Hotkeys.

---

## 📂 Project Structure

```text
EMMM2NEW/
├── .agent/                      # AI Agent Skills & Workflows
├── .docs/                       # Epic Specifications & TRD (req-01 to req-43)
├── src/                         # [FRONTEND - REACT UI]
│   ├── components/              # UI Components (Atomic Design)
│   ├── features/                # Domain Driven Components
│   │   ├── collections/         # req-31 (Collections UI)
│   │   ├── dashboard/           # req-33 (Home & Landing views)
│   │   ├── foldergrid/          # req-11, req-12 (Main Mod Listing / Explorer)
│   │   ├── objectlist/          # req-06, req-07 (Object Navigation / Sidebar)
│   │   ├── onboarding/          # req-03 (First-time App Setup wizard)
│   │   ├── preview/             # req-16, req-17, req-18 (Right Side Panel)
│   │   ├── scanner/             # req-25, req-32 (Dedup/Scanner Dashboard)
│   │   └── settings/            # req-04 (App configuration & preferences)
│   ├── hooks/                   # Custom business logic hooks
│   ├── stores/                  # Global State (Zustand: appStore, sessionStore)
│   ├── App.tsx                  # Main Layout & Routing
│   └── index.css                # Tailwind Directives
├── src-tauri/                   # [BACKEND - RUST CORE]
│   ├── migrations/              # SQLx Migrations
│   ├── src/
│   │   ├── commands/            # Tauri IPC Endpoints
│   │   ├── database/            # Database Layer (SQLx Models & Queries)
│   │   ├── services/            # Business Logic
│   │   ├── types/               # Shared Rust types & models
│   │   └── main.rs              # App Entry Point
│   └── tauri.conf.json          # Desktop permissions & setup
└── e2e/                         # Playwright E2E Tests
```

---

> Built with ❤️ for the 3DMigoto Community.
