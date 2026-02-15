# EMMM2 Mod Manager

> **The Modern Mod Orchestrator for 3DMigoto Games**  
> _Zero-Compromise. Native Performance. Data Safety. Premium Aesthetics._

![Project Badge](https://img.shields.io/badge/Tauri-v2-orange?style=flat-square&logo=tauri)
![Project Badge](https://img.shields.io/badge/Rust-Backend-black?style=flat-square&logo=rust)
![Project Badge](https://img.shields.io/badge/React-v19-blue?style=flat-square&logo=react)
![Project Badge](https://img.shields.io/badge/SQLite-Database-003B57?style=flat-square&logo=sqlite)

**EMMM2** is a next-generation mod manager designed specifically for the 3DMigoto ecosystem (Genshin Impact, Honkai: Star Rail, ZZZ, Wuthering Waves). It bridges the gap between manual file management and the modern user's need for instant speed, absolute data safety, and a premium visual experience.

---

## ✨ Key Features

### 🧠 Deep Matcher (Intelligent Scanning)

No more "Unknown" folders. Our multi-layered pipeline (Name -> Content -> AI -> Fuzzy) automatically identifies and categorizes mods with precision.

### 🛡️ Privacy Mode (Safe Protocol)

A dedicated, encrypted "Safe Mode" that creates total isolation for NSFW content. Toggle visibility instantly with an Argon2-secured PIN.

### ⚡ Zero-Lag Virtual Grid

Built on `@tanstack/react-virtual`, the explorer renders 10,000+ mods effortlessly. No pagination, no loading screens—just pure speed.

### 📦 Collections & Snapshots

Create virtual loadouts. Snapshot your entire mod list state and restore it instantly. Undo highly destructive operations with a single click.

---

## 🏗️ Tech Stack

### Frontend (The Face)

- **Framework:** React v19 + TypeScript
- **Build Tool:** Vite
- **Styling:** Tailwind CSS v4 + DaisyUI 5
- **State:** Zustand (Global), TanStack Query (Server)
- **Performance:** TanStack Virtual (Scroll Virtualization)
- **Data Management:** TanStack Table (Headless UI Tables)
- **Forms:** React Hook Form + Zod
- **Charts:** Recharts (Analytics)
- **Icons:** Lucide React

### Backend (The Muscle)

- **Core:** Rust (Tauri v2)
- **Database:** SQLite (via `sqlx`)
- **Async Runtime:** Tokio
- **Logging:** `tauri-plugin-log`

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

We follow **Test-Driven Development (TDD)** and strict architectural rules.

### Commands

| Command          | Description                        |
| :--------------- | :--------------------------------- |
| `pnpm tauri dev` | Start the app in development mode. |
| `pnpm test`      | Run Frontend Unit Tests (Vitest).  |
| `pnpm test:ui`   | Run Frontend Tests with UI.        |
| `pnpm lint`      | Check for linting errors.          |
| `pnpm format`    | Auto-format code with Prettier.    |

### Backend Testing (Rust)

```bash
cd src-tauri
cargo test
```

---

## 🗺️ Roadmap (The 13 Pillars)

The project is divided into 13 modular Epics:

1.  **Onboarding & Config**: Heuristic game detection.
2.  **Intelligent Scanning**: The core identification brain.
3.  **Game & Object Manager**: Relational categorization.
4.  **Folder Grid Explorer**: Hybrid file navigation system.
5.  **Core Operations**: Atomic toggle/rename.
6.  **Preview & INI Editor**: Rich detail panels.
7.  **Privacy Mode**: Secure content isolation.
8.  **Virtual Collections**: Snapshot & Restore.
9.  **Duplicate Scanner**: Hash-based optimization.
10. **QoL Automation**: Launcher & Randomizer.
11. **Settings**: Global configuration.
12. **System Updates**: Auto-updating infrastructure.
13. **Dashboard**: Usage analytics.

---

## 📂 Project Structure

```
emmm2/
├── .agent/              # AI Agent Skills & Workflows
├── .docs/               # Epic Specifications & TRD
├── src/                 # React Frontend
│   ├── components/      # Reusable UI components
│   ├── features/        # Feature-based modules (Slices)
│   ├── hooks/           # Custom React Hooks
│   ├── stores/          # Global State (Zustand)
│   └── test-utils.tsx   # TDD Helpers
├── src-tauri/           # Rust Backend
│   ├── src/
│   │   ├── commands/    # Tauri Commands (Exposed to FE)
│   │   ├── services/    # Business Logic
│   │   └── lib.rs       # Entry Point
│   └── migrations/      # SQLx Migrations
└── e2e/                 # Playwright E2E Tests
```

---

> Built with ❤️ for the 3DMigoto Community.
