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

- **File System Watcher and Authority:** The activation coordinator exclusively owns the active `notify-rs` watcher; React has no start/stop command. A watcher feeds scoped Disk Reconcile batches, then queues runtime publication only after commit and release of the game/operation leases. Inactive games keep lightweight watchers that only accumulate dirty top-level roots or overflow/gap evidence. In-memory authority tokens bind root identity, watcher session, reconcile revision, and dirty evidence; a clean revisit reuses the last projection, dirty roots receive scoped catch-up, and identity/gap/settings ambiguity forces one full scan. Tokens are deliberately not persisted across restart.
- **Game Activation:** `settings::set_active_game` is the single activation owner. It increments a monotonic generation, persists the selection, hands watcher coverage from the previous game to the target without an uncovered interval, publishes `game_activation:status`, and returns while baseline/catch-up work continues. Stale generations cannot publish readiness or runtime work. The active game hydrates first; inactive games are prewarmed sequentially in the background.
- **Regional Workspace Toggle:** Identity-preserving mod, parent-activation, bulk-mod, and object-batch toggles hold the per-game lease, derive and identity-validate their complete mutation scope, run regional preflight/collision checks, journal every rename, query/project only affected root keys and filesystem identities, and commit the journal after DB success. Object batches are all-or-nothing, parent confirmation is bound to a filesystem evidence token, and ambiguous scopes retain the full-scan path. Rollback uses the same scope and escalates to full only when scoped authority cannot be proven.
- **Collection Apply:** Preset application diffs only active and target roots, loads rename targets and object rows by affected keys/IDs, journals expected filesystem identities, and runs trusted root-scoped projection after the rename batch. The task/DB finalization and journal commit complete before a scoped KeyViewer generation is queued; restore, recovery, Safe Mode, and preset hotkeys use the same non-blocking runtime boundary.
- **Runtime Synchronization:** Filesystem and DB commit are synchronous authority boundaries; KeyViewer publication/reload is reconstructible and runs through a per-game, latest-wins async queue. Leaf mutations enqueue stable mod IDs, parent mutations enqueue compact root scopes, and superseded generations stop at bounded checkpoints while leaving a coherent warm base for the merged successor request. Warm composition caches retain all enabled contributions for a bounded number of games, and unchanged per-mod harvests are shared through `Arc` values instead of deep-cloned. `runtime_sync:status` reports queued/running/succeeded/manual-reload/failed generations. Runtime preflight and per-mod INI harvests are fingerprint-cached in memory, while the generated artifact and manifest remain atomically published on disk.
- **Paged Explorer:** `get_workspace_structure` returns bounded navigation context and registered object metadata only. Folder search/sort/filter runs through cursor-paged `get_workspace_explorer_page` responses (maximum 200 items); one bounded-LRU server snapshot owns the sorted candidate set and opaque listing revision for a cursor session so later pages and bulk selection do not rescan or silently absorb newly appeared folders. Expired revisions return `ExplorerSnapshotExpired`, allowing the frontend to restart the exact query and discard stale selection. Bulk “all matching” selection stays symbolic in React and is resolved against its immutable snapshot with explicit exclusions and the existing mutation safety limit.
- **Mod Ingestion:** `Ingestion` receives an archive -> validates space and analyzes/extracts it through the `compress-tools` secure iterator into staging (including entry path/type validation, byte and ratio quotas, nested archives, password, progress, and cancellation) -> hands off to `Matching` for metadata -> hands off to `Mutation` to commit to disk -> `Workspace` refreshes. Per-batch extraction state serializes analysis and makes cancellation wait for extraction to stop before staging cleanup.
- **Frontend Filesystem Boundary:** React has no `plugin-fs` capability. User-selected theme import/export dialogs and all resulting reads/writes are owned by validated Rust commands.
- **Mutation Recovery:** Durable operations write every filesystem step before mutation, explicitly commit only after the database projection succeeds, and remain recoverable when their guard drops without a terminal action. `mutation-journal.json` retains the full immutable plan and terminal history; a compact, atomic, per-operation state file under its `.active` sibling directory records each active step transition. Terminal snapshots commit before best-effort active-state cleanup, so a leftover state file cannot override a terminal journal entry. Both records are format-versioned, revisioned, checksummed, and bind their operation IDs; startup treats a surviving canonical record as committed, recovers the newest valid artifact only while canonical is absent, and stops for manual repair on ambiguous artifacts. Startup validates journal paths against configured game/staging roots, rolls back only unambiguous disk states, and isolates ambiguous states as `FailedNeedsRepair` before configuring the app-managed `MutationCoordinator`.
- **Durable Slice Migration:** Folder-conflict group rename is the first durable slice: it plans old/stage/target paths while holding the game lease, journals them before the first rename, keeps watcher suppression around apply/compensation, and commits only after Disk Reconcile projects the final disk state. Other mutation slices remain on the lock-only compatibility path until migrated explicitly.
- **Frontend Commands:** React calls IPC -> Routed to `modules/*/adapters/tauri/` -> Calls `application/` use cases -> Accesses DB via `adapters/sqlite/` -> Returns `Specta` generated types back to React.
- **Catalog Pack Updates:** The Catalog adapter checks only the fixed public GitHub release channel without end-user credentials. It downloads `catalog-pack.zip` plus its Ed25519 signature, validates bounded catalog JSON and game-scoped avatar images, atomically swaps the app-data `asset-pack`, clears only legacy asset-pack-derived thumbnail fields, and invalidates the MasterDB cache. The private signing key remains outside EMMM and Git.
- **Discover Downloads:** Windows Discover tabs keep download requests inside their WebView2 profile and wait on a native download deferral until the user confirms a collision-safe destination. The app persists only file metadata and state for its UI; it never reads or copies cookies, OAuth tokens, or request credentials. Non-Windows and non-WebView download entry points retain the bounded `reqwest` fallback.
- **Randomizer Loadout:** Roll and preview are read-only Library use cases. Apply revalidates the preview fingerprint while holding the game lease, writes the durable workspace mutation, reconciles, then records the bounded per-Object anti-repeat history. Object shuffle mode is a nullable catalog field: category defaults resolve to exclusive for Character/Weapon and additive otherwise.
