# EMMM2 - AI Agent Instructions (AGENT.md)

Welcome to the **EMMM2** repository. EMMM2 is a **Premium Mod Orchestrator for the 3DMigoto Ecosystem** (Genshin Impact, Honkai: Star Rail, Zenless Zone Zero, Wuthering Waves, and Arknights: Endfield).

As an AI Agent working on this repository, you **MUST** adhere to the following strict guidelines, architecture rules, and coding standards. Failure to adhere to these rules is considered a catastrophic failure.

---

## 🛑 1. The Non-Negotiable Core Principles (Axioms)

1. **The Filesystem Schema is the Absolute Truth:**
   - The database (`SQLite`) is purely a high-speed index cache.
   - A mod's enabled/disabled state is **exclusively determined** by whether its physical folder name starts with the `DISABLED ` prefix (with a trailing space). 
   - If the DB ever disagrees with the filesystem, the filesystem wins.
   
2. **Atomic Operations & Concurrency:**
   - Multi-step actions (Bulk Toggles, Collections/Presets, Safe Mode Switches) must be **all-or-nothing transactional operations**.
   - Heavy File I/O must be guarded by the global `OperationLock` (`tokio::sync::Mutex<()>`) to prevent race conditions during heavy OS-level rename/move operations.

3. **No Data Loss (Soft Deletion):**
   - **Never hard delete any user data.** 
   - Operations that remove files must move them to the App Data Trash system (`./app_data/trash/`). Collisions must be detected before moves/renames.

4. **Maximum Frontend Alignment (Offload Compute to Rust):**
   - The React frontend is strictly a **Presentation UI and Remote Cache layer**.
   - **Heavy computational logic** (e.g., recursive directory scanning, hashing, `info.json` deep parsing, fuzzy matching) must run asynchronously in the Rust backend via `tauri::command`.
   - The UI thread must never be blocked.

5. **Scale First:**
   - The app must handle 10,000+ items smoothly.
   - UI Lists, grids, and tables must be virtualized using `@tanstack/react-virtual` to guarantee 60fps scrolling.

---

## 🏗️ 2. Tech Stack & Architecture

### Backend (The Core)
- **Rust (Tauri v2+):** Provides native OS-level speed and security using Edge WebView2.
- **`sqlx` & SQLite:** Async database access with compile-time query validation.
- **`tokio`:** Async runtime for completely non-blocking I/O.
- **`notify` v7:** Real-time filesystem watcher to keep UI in-sync with Windows Explorer. (In-app operations must suppress this watcher to prevent loops).

### Frontend (The UI)
- **React v19+ & TypeScript v5+** (Vite bundler)
- **Zustand:** Global synchronous state (e.g., Safe Mode toggles, Active Game).
- **TanStack Query:** Async state caching layer acting as a bridge to Rust IPC.
- **daisyUI 5 & Tailwind CSS 4:** Styling. Prioritize beautiful, dark-mode-first premium aesthetics with micro-animations (`framer-motion`).
- **React Hook Form + Zod:** Form validations.

### Domain Models
- **Games (`games`):** Installed 3DMigoto game titles.
- **Objects (`objects`):** Virtual categories (Characters, Weapons, UI) driven by per-game `schema.json`.
- **Mods (`mods`):** Physical folders. Has stable Identifiers (SHA1 hash of the relative folder path). 
- **Collections (`collections`):** Virtual loadouts that are applied atomically.
- **Safe Mode:** Integrated into both Frontend (Zustand) and Backend (Query `is_safe = 1` filters) to protect NSFW content.

---

## 🤖 3. AI Agent Coding Standards

When editing code, follow the project's zero-tolerance context policies:

### 3.1 Context & No Assumptions
- **Context is required. No guessing.** Never invent APIs, types, schemas, file paths, behavior, or patterns. Always check `.docs/requirements/` (`req-01` to `req-43`) or the database schema files if unsure.
- **Read before edit.** Identify the exact file(s), call sites, and dependencies.
- **If unclear → Ask the User.** Do not build on assumptions.

### 3.2 🚫 Zero-Truncation Policy / Safe Output
- **NO PLACEHOLDERS:** Never replace existing code with `// ...`, `// rest unchanged`, or `/* omitted */`. 
- **Preserve 100% of logic.** Do not accidentally delete configuration, edge-case handlers, or UI elements.
- Ensure outputs provide full, valid unified diffs or complete file outputs.

### 3.3 Folder Structure
Follow the established feature-driven architecture:
- `src-tauri/src/commands/` - Tauri IPC Endpoints
- `src-tauri/src/database/` - Repositories / DAL (strict separation from services)
- `src-tauri/src/services/` - Business Logic (Scanner, Mods, INI Parser)
- `src/features/` - Domain-driven UI components (e.g., `foldergrid`, `objectlist`, `preview`)
- `src/components/` - Shared atomic UI elements

### 3.4 Verification
Always verify:
1. Did I introduce duplicate logic? Refactor to a single source of truth instead.
2. Are there unused variables, types, or unreachable branches? Clean them up.
3. Will this change block the `tokio` runtime or the React main thread?
4. If testing, run `cargo test` on backend changes or `vitest` for the frontend.

> *Your ultimate goal is to keep EMMM2 fast, reliable, memory-safe, and visually exceptional without ever compromising the user's mod files.*
