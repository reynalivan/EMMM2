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
- **Atomic Operations:** Direct filesystem manipulation by the frontend is forbidden. All disk writes and structural changes must go through the single app-managed `MutationCoordinator`. It owns the shared `OperationLock`, persistent operation journal, and active task registry; production command state must never manage or inject a second `OperationLock`.

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
9. **`library`**: Disk-direct views of mod folders, basic mod manipulation, and read-only Mod Health analysis.
10. **`matching`**: Deep/Fuzzy matching engine bridging local folders to Master DB taxonomy (Epic 4).
11. **`mutation`**: The engine room. Houses the `MutationCoordinator`, operation journaling, and recovery systems.
12. **`reconciliation`**: Disk-to-DB syncing. Runs silently to fix discrepancies and handles 'Safe Mode' transitions (Epic 7).
13. **`settings`**: App-level configuration and global preferences.
14. **`system`**: Bootstrap routines, logging, themes, application lifecycle, and the shared game process/focus detector used by runtime hotkeys and KeyViewer maintenance.
15. **`updates`**: Upstream update checkers for both the app and installed mods.
16. **`workspace`**: The active mod routing, file watcher lifecycle, and live grid logic.

## 4. Key Data Flows

- **File System Watcher:** `notify-rs` detects changes -> Triggers `Disk Reconcile` (Reconciliation Module) -> Emits UI updates. (Suppressed during controlled mutations).
- **Mod Ingestion:** `Ingestion` receives an archive -> validates space and analyzes/extracts it through the `compress-tools` secure iterator into staging (including entry path/type validation, byte and ratio quotas, nested archives, password, progress, and cancellation) -> hands off to `Matching` for metadata -> hands off to `Mutation` to commit to disk -> `Workspace` refreshes. Per-batch extraction state serializes analysis and makes cancellation wait for extraction to stop before staging cleanup.
- **Frontend Filesystem Boundary:** React has no `plugin-fs` capability. User-selected theme import/export dialogs and all resulting reads/writes are owned by validated Rust commands.
- **Mutation Recovery:** Durable operations write every filesystem step before mutation, explicitly commit only after the database projection succeeds, and remain recoverable when their guard drops without a terminal action. `mutation-journal.json` retains the full immutable plan and terminal history; a compact, atomic, per-operation state file under its `.active` sibling directory records each active step transition. Terminal snapshots commit before best-effort active-state cleanup, so a leftover state file cannot override a terminal journal entry. Both records are format-versioned, revisioned, checksummed, and bind their operation IDs; startup treats a surviving canonical record as committed, recovers the newest valid artifact only while canonical is absent, and stops for manual repair on ambiguous artifacts. Startup validates journal paths against configured game/staging roots, rolls back only unambiguous disk states, and isolates ambiguous states as `FailedNeedsRepair` before configuring the app-managed `MutationCoordinator`.
- **Durable Slice Migration:** Folder-conflict group rename is the first durable slice: it plans old/stage/target paths while holding the game lease, journals them before the first rename, keeps watcher suppression around apply/compensation, and commits only after Disk Reconcile projects the final disk state. Other mutation slices remain on the lock-only compatibility path until migrated explicitly.
- **Frontend Commands:** React calls IPC -> Routed to `modules/*/adapters/tauri/` -> Calls `application/` use cases -> Accesses DB via `adapters/sqlite/` -> Returns `Specta` generated types back to React.
- **Catalog Pack Updates:** The Catalog adapter checks only the fixed public GitHub release channel without end-user credentials. It downloads `catalog-pack.zip` plus its Ed25519 signature, validates bounded catalog JSON and game-scoped avatar images, atomically swaps the app-data `asset-pack`, clears only legacy asset-pack-derived thumbnail fields, and invalidates the MasterDB cache. The private signing key remains outside EMMM and Git.
- **Discover Downloads:** Windows Discover tabs keep download requests inside their WebView2 profile and wait on a native download deferral until the user confirms a collision-safe destination. The app persists only file metadata and state for its UI; it never reads or copies cookies, OAuth tokens, or request credentials. Non-Windows and non-WebView download entry points retain the bounded `reqwest` fallback.
- **Randomizer Loadout:** Roll and preview are read-only Library use cases. Apply revalidates the preview fingerprint while holding the game lease, writes the durable workspace mutation, reconciles, then records the bounded per-Object anti-repeat history. Object shuffle mode is a nullable catalog field: category defaults resolve to exclusive for Character/Weapon and additive otherwise.
