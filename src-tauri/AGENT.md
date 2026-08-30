# EMMM2 Backend Architecture & AI Index (AGENT.md)

> **[CRITICAL AI INSTRUCTION]**
> You are reading the Architectural Compass for the EMMM2 Backend. 
> **Rule of Maintenance:** If you (the AI Agent) implement a new module, remove an old one, or significantly alter a data flow, you **MUST** update this file to reflect the new reality. Do not let this map fall out of sync with the codebase.

## 1. System Paradigm
EMMM2's backend is a **Modular Monolith** using **Strict Vertical Slices**.
- **No Horizontal Layers:** We do not use global `services/`, `repo/`, or `domain/` folders. Everything is grouped by business capability (features).
- **Strict Encapsulation:** Internal module layers (`domain`, `application`, `adapters`) are explicitly marked as `pub(crate)` in `mod.rs`. 
- **The API Boundary:** Modules can **only** communicate with each other or expose functions to integration tests via their explicit `api.rs` facade. Never bypass `api.rs`.
- **Tauri Segregation:** Tauri IPC endpoints live exclusively inside `adapters/tauri/`. Core business logic (`application/`) must never import or depend on `tauri`.
- **Atomic Operations:** Direct filesystem manipulation by the frontend is forbidden. All disk writes and structural changes must go through the `MutationCoordinator` (which locks the watcher).

## 2. Core Topology
- `src/modules/` -> The 16 Vertical Slices (The heart of the application).
- `src/platform/` -> Cross-cutting infrastructure (Filesystem wrappers, OS-level recycle bin, OperationLocks).
- `src/pipeline/` -> Complex orchestrators that span across multiple modules (e.g., Collection Apply Pipeline).
- `src/shared/` -> Global primitives, error definitions, and synchronous types.

## 3. Module Index (The 16 Slices)
1. **`automation`**: Executes external scripts and user-defined tool chains.
2. **`browser`**: Internal web browser integration and scraping logic.
3. **`catalog`**: Master DB logic, object tracking, and metadata taxonomy.
4. **`collections`**: Loadout snapshots and preset management (diffing applied states).
5. **`dashboard`**: Analytics, application telemetry, and overview statistics.
6. **`duplicates`**: Deep-scanning engine for finding redundant files across mod folders (Epic 9).
7. **`games`**: Game configuration management, executable paths, and registry logic.
8. **`ingestion`**: Import queue, background ZIP/RAR extraction, and drop-zone handling (Epic 5).
9. **`library`**: Disk-direct views of mod folders and basic mod manipulation.
10. **`matching`**: Deep/Fuzzy matching engine bridging local folders to Master DB taxonomy (Epic 4).
11. **`mutation`**: The engine room. Houses the `MutationCoordinator`, operation journaling, and recovery systems.
12. **`reconciliation`**: Disk-to-DB syncing. Runs silently to fix discrepancies and handles 'Safe Mode' transitions (Epic 7).
13. **`settings`**: App-level configuration and global preferences.
14. **`system`**: Bootstrap routines, logging, themes, and application lifecycle.
15. **`updates`**: Upstream update checkers for both the app and installed mods.
16. **`workspace`**: The active mod routing, file watcher lifecycle, and live grid logic.

## 4. Key Data Flows
- **File System Watcher:** `notify-rs` detects changes -> Triggers `Disk Reconcile` (Reconciliation Module) -> Emits UI updates. (Suppressed during controlled mutations).
- **Mod Ingestion:** `Ingestion` receives ZIP -> Extracts to staging -> Hands off to `Matching` for metadata -> Hands off to `Mutation` to commit to disk -> `Workspace` refreshes.
- **Frontend Commands:** React calls IPC -> Routed to `modules/*/adapters/tauri/` -> Calls `application/` use cases -> Accesses DB via `adapters/sqlite/` -> Returns `Specta` generated types back to React.
