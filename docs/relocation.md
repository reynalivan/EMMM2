# Architecture target

EMMM2 should use Feature-Sliced Design (FSD) in the frontend and a domain-oriented modular monolith in the Rust backend. Tauri should remain a thin inbound adapter and composition root. Complex backend modules can use application, domain, and ports/adapters boundaries where they provide clear value.

The best structure for EMMM2 is:

| Area                    | Target architecture                                                                 |
| ----------------------- | ----------------------------------------------------------------------------------- |
| Frontend                | **Enforced Feature-Sliced Design**                                                  |
| Backend                 | **Domain/capability-based modular monolith**                                         |
| Internal backend module | **Application + Domain + Ports/Adapters**, only when complexity warrants it          |
| Tauri                   | **Thin inbound adapter and composition root**                                       |
| Data consistency        | **Recoverable mutation workflow + operation journal + reconciliation**              |
| Async frontend data     | **TanStack Query**                                                                  |
| Client-only state       | **Small Zustand stores owned by the relevant domain/widget**                        |

---

# 1. Target repository structure

```text
EMMM2/
├── .agent/                         # Canonical agent rules, skills, workflows
├── .claude/                        # Tool-specific adapter/generated mirror
├── .codex/
├── .cursor/
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
│
├── docs/
│   ├── architecture/
│   │   ├── overview.md
│   │   ├── frontend-fsd.md
│   │   ├── backend-modules.md
│   │   ├── dependency-rules.md
│   │   ├── data-consistency.md
│   │   └── adr/
│   │       ├── 0001-frontend-fsd.md
│   │       ├── 0002-backend-modular-monolith.md
│   │       ├── 0003-disk-db-consistency.md
│   │       └── 0004-tauri-contract-boundary.md
│   ├── requirements/
│   ├── test-cases/
│   ├── knowledge/
│   ├── tasks/
│   └── history/
│
├── src/                            # React frontend
├── src-tauri/                      # Rust backend + Tauri shell
│
├── tests/
│   └── e2e/
│       ├── specs/
│       ├── fixtures/
│       └── support/
│
├── public/
├── scripts/
│   ├── check-architecture.mjs
│   ├── generate-bindings.mjs
│   └── sync-agent-configs.mjs
│
├── .env.example
├── .gitignore
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
└── wdio.conf.ts
```

Project documentation lives in `docs/`, making it easier for developers and standard tooling to find.

---

# 2. Frontend target: properly applied FSD

Official FSD defines this layer order:

```text
app
pages
widgets
features
entities
shared
```

Imports are allowed only toward lower layers. Slices on the same layer must be independent, and access from outside a slice must go through its public API. FSD also recommends grouping segments by purpose, such as `ui`, `api`, `model`, `lib`, and `config`, rather than using generic groupings such as `components`, `hooks`, or `types`.

## Recommended frontend structure

```text
src/
├── app/
│   ├── entrypoint/
│   │   ├── App.tsx
│   │   └── main.tsx
│   │
│   ├── providers/
│   │   ├── QueryProvider.tsx
│   │   ├── I18nProvider.tsx
│   │   ├── ThemeProvider.tsx
│   │   └── ErrorBoundaryProvider.tsx
│   │
│   ├── routes/
│   │   ├── router.tsx
│   │   └── routePaths.ts
│   │
│   ├── runtime/
│   │   ├── fileWatcherBridge.ts
│   │   ├── queryEventBridge.ts
│   │   ├── startupRecovery.ts
│   │   └── runtimeEvents.ts
│   │
│   ├── store/
│   │   └── resetClientState.ts
│   │
│   └── styles/
│       ├── index.css
│       └── themes.css
│
├── pages/
│   ├── onboarding/
│   │   ├── ui/
│   │   │   └── OnboardingPage.tsx
│   │   └── index.ts
│   │
│   ├── dashboard/
│   │   ├── ui/
│   │   │   ├── DashboardPage.tsx
│   │   │   ├── DashboardCharts.tsx
│   │   │   └── DashboardStats.tsx
│   │   ├── api/
│   │   └── index.ts
│   │
│   ├── workspace/
│   │   ├── ui/
│   │   │   └── WorkspacePage.tsx
│   │   ├── model/
│   │   └── index.ts
│   │
│   ├── collections/
│   ├── storage-optimizer/
│   ├── browser/
│   ├── downloads/
│   ├── mod-inbox/
│   └── settings/
│
├── widgets/
│   ├── app-shell/
│   │   ├── ui/
│   │   │   ├── AppShell.tsx
│   │   │   └── ResizableWorkspace.tsx
│   │   └── index.ts
│   │
│   ├── top-bar/
│   │   ├── ui/
│   │   ├── model/
│   │   └── index.ts
│   │
│   ├── object-sidebar/
│   │   ├── ui/
│   │   ├── model/
│   │   ├── lib/
│   │   └── index.ts
│   │
│   ├── mod-explorer/
│   │   ├── ui/
│   │   │   ├── ModExplorer.tsx
│   │   │   ├── ModGrid.tsx
│   │   │   ├── ModList.tsx
│   │   │   ├── ModCard.tsx
│   │   │   ├── ExplorerToolbar.tsx
│   │   │   └── ExplorerBreadcrumbs.tsx
│   │   ├── model/
│   │   │   ├── selectionStore.ts
│   │   │   ├── useModExplorer.ts
│   │   │   └── useExplorerNavigation.ts
│   │   ├── lib/
│   │   └── index.ts
│   │
│   ├── mod-preview/
│   │   ├── ui/
│   │   ├── model/
│   │   └── index.ts
│   │
│   ├── launch-bar/
│   ├── import-queue/
│   └── conflict-center/
│
├── features/
│   ├── game/                       # Slice group only; no shared code here
│   │   ├── switch/
│   │   │   ├── ui/
│   │   │   ├── model/
│   │   │   └── index.ts
│   │   ├── manage/
│   │   └── launch/
│   │
│   ├── mod/
│   │   ├── toggle/
│   │   ├── rename/
│   │   ├── trash/
│   │   ├── bulk-manage/
│   │   ├── edit-metadata/
│   │   ├── edit-ini/
│   │   ├── set-thumbnail/
│   │   └── randomize/
│   │
│   ├── library/
│   │   ├── scan/
│   │   ├── import/
│   │   ├── reconcile/
│   │   ├── resolve-folder-conflict/
│   │   └── resolve-shader-conflict/
│   │
│   ├── collection/
│   │   ├── manage/
│   │   └── apply/
│   │
│   ├── privacy/
│   │   ├── switch-mode/
│   │   └── classify-mod/
│   │
│   └── storage/
│       └── resolve-duplicates/
│
├── entities/
│   ├── game/
│   │   ├── api/
│   │   │   ├── gameQueries.ts
│   │   │   └── gameQueryKeys.ts
│   │   ├── model/
│   │   │   ├── game.ts
│   │   │   └── activeGameStore.ts
│   │   ├── ui/
│   │   └── index.ts
│   │
│   ├── game-object/
│   ├── mod/
│   ├── workspace/
│   ├── collection/
│   ├── conflict/
│   ├── duplicate-group/
│   ├── import-batch/
│   ├── download/
│   ├── task/
│   └── privacy-mode/
│
└── shared/
    ├── api/
    │   └── tauri/
    │       ├── bindings.gen.ts       # Generated; never edited manually
    │       ├── client.ts
    │       ├── channels.ts
    │       ├── events.ts
    │       ├── error.ts
    │       └── index.ts
    │
    ├── ui/
    │   ├── button/
    │   ├── confirm-dialog/
    │   ├── context-menu/
    │   ├── error-boundary/
    │   ├── list-state/
    │   ├── tag-input/
    │   └── toast/
    │
    ├── lib/
    │   ├── error/
    │   ├── format/
    │   ├── logger/
    │   ├── path/
    │   ├── promise/
    │   └── testing/
    │
    ├── i18n/
    │   ├── config.ts
    │   └── locales/
    │       ├── en/
    │       ├── id/
    │       └── zh/
    │
    ├── config/
    └── assets/
```

## Why should Folder Grid be a widget?

`FolderGrid` currently includes:

- Toolbar
- Breadcrumb
- Grid/list display
- Selection
- Drag-and-drop
- Bulk actions
- Navigation
- Conflict handling
- Modal orchestration

It is a large UI block that composes many entities and features. It fits better as `widgets/mod-explorer` than `features/folder-grid`. FSD defines a widget as a large, independent UI block, especially when a page contains several large blocks such as a sidebar, explorer, and preview panel.

## Avoid creating too many features

Not every button needs its own slice. FSD itself says that not everything needs to be a feature; this layer is most useful for important interactions or interactions used on multiple screens.

A well-structured `features/mod/toggle` example:

```text
features/mod/toggle/
├── api/
│   └── toggleMod.ts
├── model/
│   ├── useToggleMod.ts
│   └── optimisticToggle.ts
├── ui/
│   └── ToggleModButton.tsx
├── lib/
│   └── toggleErrorMapper.ts
└── index.ts
```

Public API:

```ts
// features/mod/toggle/index.ts
export { ToggleModButton } from './ui/ToggleModButton';
export { useToggleMod } from './model/useToggleMod';
```

The slice uses relative imports internally. Other slices may only import it through:

```ts
import { useToggleMod } from '@/features/mod/toggle';
```

Not through:

```ts
import { useToggleMod } from '@/features/mod/toggle/model/useToggleMod';
```

The public API should expose only the required contract, not `export *`. Official FSD also warns against wildcard barrels because they expand the surface area and can increase circular imports.

---

# 3. Frontend state ownership

The project currently distinguishes Zustand for client state from TanStack Query for async state. However, the root `stores/` directory can too easily become the owner of all state.

Use these rules:

| State type                                     | Owner                         |
| ---------------------------------------------- | ----------------------------- |
| Mods, objects, collections, settings from Rust | TanStack Query                |
| Loading/error/result async                     | TanStack Query                |
| Active game session                            | `entities/game/model`         |
| Privacy mode session                           | `entities/privacy-mode/model` |
| Grid selection                                 | `widgets/mod-explorer/model`  |
| Preview draft                                  | `widgets/mod-preview/model`   |
| Temporary form input                           | Related component/feature     |
| Theme and provider lifecycle                   | `app`                         |
| Toast                                          | `shared/ui/toast`             |
| Business modal                                 | Owning feature/widget         |

TanStack Query is intended for server/async state, while Zustand is a client-state manager. TanStack documentation notes that after async state moves to Query, global client state is usually much smaller.

Therefore, do not duplicate these in Zustand:

```ts
mods;
objects;
collections;
downloads;
dashboardStats;
settings;
```

when the same data already lives in the backend and TanStack Query.

---

# 4. Backend target: modular monolith, not global horizontal layers

Current structure:

```text
commands/
domain/
repo/
services/
```

is better than the original TRD structure, but at the current scale a single feature change is spread across four large trees. The original TRD did define `commands`, `database`, and `services` as horizontal layers.

A better-fitting target:

```text
src-tauri/src/
├── app/
├── modules/
├── platform/
├── shared/
├── lib.rs
└── main.rs
```

## Final backend structure

```text
src-tauri/
├── migrations/
├── permissions/
├── capabilities/
├── resources/
├── icons/
├── .sqlx/
│
├── src/
│   ├── app/
│   │   ├── bootstrap.rs
│   │   ├── state.rs
│   │   ├── command_registry.rs
│   │   ├── event_registry.rs
│   │   ├── error.rs
│   │   │
│   │   ├── runtime/
│   │   │   ├── mutation_coordinator.rs
│   │   │   ├── operation_journal.rs
│   │   │   ├── recovery_runner.rs
│   │   │   ├── task_registry.rs
│   │   │   └── event_sink.rs
│   │   │
│   │   └── mod.rs
│   │
│   ├── modules/
│   │   ├── system/
│   │   ├── games/
│   │   ├── catalog/
│   │   ├── library/
│   │   ├── ingestion/
│   │   ├── workspace/
│   │   ├── collections/
│   │   ├── storage_optimizer/
│   │   ├── privacy/
│   │   ├── automation/
│   │   ├── browser/
│   │   └── dashboard/
│   │
│   ├── platform/
│   │   ├── db/
│   │   │   ├── pool.rs
│   │   │   ├── transaction.rs
│   │   │   └── mod.rs
│   │   │
│   │   ├── fs/
│   │   │   ├── atomic_file.rs
│   │   │   ├── guard.rs
│   │   │   ├── locking.rs
│   │   │   ├── path.rs
│   │   │   └── mod.rs
│   │   │
│   │   ├── watcher/
│   │   │   ├── notify_adapter.rs
│   │   │   ├── suppression.rs
│   │   │   ├── debounce.rs
│   │   │   └── mod.rs
│   │   │
│   │   ├── archive/
│   │   ├── image/
│   │   ├── http/
│   │   ├── process/
│   │   ├── jobs/
│   │   ├── logging/
│   │   └── time/
│   │
│   ├── shared/
│   │   ├── ids.rs
│   │   ├── path_key.rs
│   │   ├── pagination.rs
│   │   ├── result.rs
│   │   └── mod.rs
│   │
│   ├── lib.rs
│   └── main.rs
│
├── tests/
│   ├── architecture/
│   │   ├── module_boundaries.rs
│   │   └── forbidden_dependencies.rs
│   ├── integration/
│   ├── fixtures/
│   └── common/
│
├── build.rs
├── Cargo.toml
├── Cargo.lock
└── tauri.conf.json
```

---

# 5. Backend module boundaries

| Module              | Responsibilities                                                   |
| ------------------- | ------------------------------------------------------------------ |
| `system`            | Bootstrap, app settings, maintenance, updater, log access          |
| `games`             | Game registry, path validation, launcher configuration             |
| `catalog`           | Master DB, game objects, schemas, aliases, metadata sync           |
| `library`           | Mod lifecycle, toggle, rename, trash, metadata, INI, thumbnails    |
| `ingestion`         | Archive extraction, scanning, matching, import batch, mod inbox    |
| `workspace`         | Explorer read model, source switching, projections, reconciliation |
| `collections`       | Collection CRUD, preview, apply, undo, reference healing           |
| `storage_optimizer` | Duplicate scan, grouping, ignore pairs, resolution                 |
| `privacy`           | Safe Mode policy, PIN, classification, safe-mode transition        |
| `automation`        | Game launch, hotkeys, randomizer, keyviewer                        |
| `browser`           | Webview, downloads, import jobs                                    |
| `dashboard`         | Read-only aggregation and recent activity                          |

This is intentionally **not an exact one-to-one match with the 13 Epics**.

For example:

- Epic 4 Folder Grid mostly belongs in a frontend widget.
- Epic 5 Core Operations belongs in `library/application`.
- Epic 6 Preview belongs in a frontend widget, while INI and metadata belong in `library`.
- Epic 12 metadata sync belongs in `catalog`, while the app updater belongs in `system`.
- Epic 13 Dashboard can be a read-model module and does not need a complex domain model.

---

# 6. Contents of a backend module

Example of a complex `library` module:

```text
modules/library/
├── domain/
│   ├── mod_entry.rs
│   ├── mod_status.rs
│   ├── mod_path.rs
│   ├── metadata.rs
│   ├── policies.rs
│   ├── error.rs
│   └── mod.rs
│
├── application/
│   ├── commands/                  # Business mutations, not Tauri commands
│   │   ├── toggle_mod.rs
│   │   ├── enable_only.rs
│   │   ├── rename_mod.rs
│   │   ├── trash_mod.rs
│   │   ├── update_metadata.rs
│   │   ├── save_ini.rs
│   │   └── set_thumbnail.rs
│   │
│   ├── queries/
│   │   ├── list_mods.rs
│   │   ├── get_mod_detail.rs
│   │   ├── list_preview_assets.rs
│   │   └── list_ini_files.rs
│   │
│   ├── ports/
│   │   ├── mod_repository.rs
│   │   ├── mod_files.rs
│   │   ├── metadata_store.rs
│   │   ├── ini_store.rs
│   │   └── event_sink.rs
│   │
│   ├── dto.rs
│   └── mod.rs
│
├── adapters/
│   ├── inbound/
│   │   └── tauri.rs
│   │
│   └── outbound/
│       ├── sqlx_mod_repository.rs
│       ├── windows_mod_files.rs
│       ├── info_json_store.rs
│       ├── line_ini_store.rs
│       └── thumbnail_cache.rs
│
├── facade.rs
├── tests.rs
└── mod.rs
```

Dependency flow:

```text
Tauri command
      ↓
Application use case
      ↓
Domain rules
      ↓ port
Outbound adapter
      ↓
SQLite / filesystem / HTTP
```

This combines vertical modules and Ports & Adapters: code is grouped by capability, while external systems remain outside the application core. Vertical Slice groups code by axis of change, while Ports & Adapters separates the application core from the database, filesystem, UI, and external environment.

## Do not force every module to have every directory

For `dashboard`, a simple structure is enough:

```text
modules/dashboard/
├── query.rs
├── dto.rs
├── sqlx_adapter.rs
├── tauri.rs
└── mod.rs
```

Dashboard is a read model. There is no need to create:

```text
domain/
ports/
repositories/
services/
factories/
```

if there are no domain rules that require them.

---

# 7. Backend public API

Rust is private by default. Use this for architecture enforcement instead of making every module `pub`. Official Rust documentation keeps implementation details private by default and supports `pub use` to create a public interface separate from the internal structure.

Example:

```rust
// modules/library/mod.rs

mod adapters;
mod application;
mod domain;
mod facade;

pub(crate) use facade::LibraryApi;
pub(crate) use application::dto::{
    ModDetailDto,
    ModListItemDto,
    ToggleModRequest,
};
```

Other modules may use:

```rust
use crate::modules::library::LibraryApi;
```

But they must not use:

```rust
use crate::modules::library::adapters::outbound::sqlx_mod_repository;
use crate::modules::library::application::commands::toggle_mod;
```

Cross-module operations must go through `facade.rs`.

---

# 8. Keep Tauri commands very thin

Tauri commands are inbound adapters, not places for business logic.

```rust
#[tauri::command]
async fn toggle_mod(
    state: tauri::State<'_, AppState>,
    request: ToggleModRequest,
) -> Result<ToggleModResponse, AppErrorDto> {
    state
        .library
        .toggle_mod(request)
        .await
        .map_err(AppErrorDto::from)
}
```

The following do not belong in a command:

```rust
sqlx::query!(...)
std::fs::rename(...)
WalkDir::new(...)
notify(...)
blake3::hash(...)
```

Tauri supports async commands and recommends them for heavy work so the UI does not freeze. For streaming progress such as scanning, importing, or hashing, use a channel; events are better suited to small broadcasts and are not designed for high-throughput streaming.

Use a typed error:

```rust
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    pub context: Option<serde_json::Value>,
}
```

Do not use only:

```rust
Result<T, String>
```

Tauri supports custom serializable error representations, allowing the frontend to map errors by code instead of parsing text.

---

# 9. Move the operation lock

The old TRD says:

> Acquire the operation lock in the Command layer so services remain reusable.

For the current EMMM2 structure, this rule is no longer sufficient because mutations can be triggered by:

- Tauri command
- Hotkey
- Collection apply
- Randomizer
- Background recovery
- Watcher reconciliation
- Startup repair

If the lock exists only in Tauri commands, other entry points can bypass it.

## New target

```text
app/runtime/mutation_coordinator.rs
```

All destructive mutations must go through:

```rust
pub struct MutationCoordinator {
    lock: tokio::sync::Mutex<()>,
    journal: OperationJournal,
}
```

Flow:

```text
Tauri / Hotkey / Recovery
          ↓
MutationCoordinator
          ↓
Module use case
          ↓
Filesystem + SQLite
```

Commands only translate `OperationBusy` into a toast/error DTO.

---

# 10. Do not treat disk and SQLite as one atomic transaction

EMMM2 has three forms of truth:

- Disk as physical truth.
- SQLite as the logical index.
- `info.json` as portable metadata.

A SQLite transaction cannot automatically roll back a Windows filesystem rename. Therefore, the best workflow does not claim that the two are one transaction; it uses a **recoverable mutation** instead:

```text
1. Validate request
2. Build mutation plan
3. Write journal: PENDING
4. Execute filesystem operations
5. Persist SQLite projection
6. Mark journal: COMPLETED
7. Emit frontend refresh
```

On a crash:

```text
Startup
   ↓
Read incomplete journal
   ↓
Inspect disk state
   ↓
   Complete, compensate, or reconcile
```

Structure:

```text
app/runtime/
├── mutation_coordinator.rs
├── operation_journal.rs
├── recovery_runner.rs
└── task_registry.rs
```

Meanwhile:

```text
modules/workspace/
├── application/
│   └── reconcile/
└── adapters/
    └── watcher_projection.rs
```

This better fits EMMM2's hybrid truth model and the `recovery`, `disk_reconcile`, and `workspace_mutation` structures already beginning to take shape in the current code.

---

# 11. Mapping the current frontend to the target

| Current                                     | Target                                                     |
| ------------------------------------------- | ---------------------------------------------------------- |
| `src/features/dashboard`                    | `src/pages/dashboard`                                      |
| `src/features/settings`                     | `src/pages/settings`                                       |
| `src/features/browser/BrowserPage`          | `src/pages/browser`                                        |
| `src/features/browser/DownloadsPage`        | `src/pages/downloads`                                      |
| `src/features/collections/CollectionsPage`  | `src/pages/collections`                                    |
| `src/features/scanner/StorageOptimizerPage` | `src/pages/storage-optimizer`                              |
| `src/features/mod-inbox`                    | `src/pages/mod-inbox`                                      |
| `src/features/folder-grid`                  | `src/widgets/mod-explorer`                                 |
| `src/features/object-list`                  | `src/widgets/object-sidebar`                               |
| `src/features/preview`                      | `src/widgets/mod-preview`                                  |
| `src/features/launch-bar`                   | `src/widgets/launch-bar`                                   |
| `src/shared/components/layout`              | `src/widgets/app-shell` and `top-bar`                      |
| `src/core/tauri`                            | `src/shared/api/tauri`                                     |
| `src/core/lib/queryClient`                  | `src/app/providers`                                        |
| `src/core/lib/i18n` + `src/locales`         | `src/shared/i18n`                                          |
| `src/features/file-watcher`                 | `src/app/runtime` + `features/library/reconcile`           |
| `src/features/runtime-sync`                 | `src/app/runtime/queryEventBridge`                         |
| `src/features/workspace-runtime`            | `entities/workspace`, widget model, and reconcile features |
| `src/stores`                                | Distributed to their respective owners                     |
| `src/types`                                 | Distributed to `entities/*/model`                           |
| Generic `components/hooks/utils`            | `ui/model/lib/api` based on responsibility                 |

---

# 12. Mapping the current backend to the target

| Current                               | Target                                                                 |
| ------------------------------------- | ---------------------------------------------------------------------- |
| `commands/*`                          | `modules/*/adapters/inbound/tauri.rs`                                  |
| `repo/*`                              | `modules/*/adapters/outbound/sqlx_*.rs`                                |
| `domain/*`                            | `modules/*/domain`                                                     |
| Business code in `services/*`         | `modules/*/application`                                                |
| Generic I/O in `services/fs_utils`    | `platform/fs`                                                          |
| `services/images`                     | `platform/image` or a library outbound adapter                         |
| `services/update`                     | `modules/system` and `modules/catalog`                                 |
| `services/match_engine`               | `modules/ingestion`                                                    |
| `services/import_batch`               | `modules/ingestion`                                                    |
| `services/mods`                       | `modules/library`                                                      |
| `services/objects`                    | `modules/catalog`                                                      |
| `services/collection*`                | `modules/collections`                                                  |
| `services/scanner/dedup`              | `modules/storage_optimizer`                                            |
| `services/scanner/deep_matcher`       | `modules/ingestion`                                                    |
| `services/workspace*`                 | `modules/workspace`                                                    |
| `services/projected_state`            | `modules/workspace`                                                    |
| `services/disk_reconcile`             | `modules/workspace/application/reconcile`                              |
| `pipeline/*`                          | `modules/collections/application/apply` if only for collection pipeline |
| `common/*`                            | Owning module or `shared` if truly generic                             |
| `types/dup_scan.rs`                   | `modules/storage_optimizer/domain` or `dto`                            |

---

# 13. Testing structure

```text
src/
└── .../
    ├── Component.tsx
    ├── Component.test.tsx
    ├── model.ts
    └── model.test.ts

src-tauri/src/
└── modules/library/
    ├── domain/
    │   ├── mod_status.rs
    │   └── mod_status_tests.rs
    └── application/
        ├── toggle_mod.rs
        └── toggle_mod_tests.rs

src-tauri/tests/
├── architecture/
├── integration/
└── fixtures/

tests/e2e/
├── specs/
├── fixtures/
└── support/
```

Rust distinguishes unit tests near a module, which can test private implementation, from integration tests in the `tests/` directory, which use only the public interface.

Existing architecture tests such as:

```text
arch_audit.rs
dal_audit.rs
```

should be kept and expanded to prohibit:

```text
domain -> sqlx
domain -> tauri
application -> tauri
application -> notify
module A -> internal module B
```

---

# 14. Architecture rules to enforce in CI

## Frontend

```text
app      → pages, widgets, features, entities, shared
pages    → widgets, features, entities, shared
widgets  → features, entities, shared
features → entities, shared
entities → shared
shared   → shared only
```

Add architecture linting. FSD documentation recommends an architectural linter to detect deep imports and public API violations.

## Backend

```text
app                 → modules, platform, shared
inbound adapter     → application
application         → domain, ports, shared
domain              → domain/shared pure types only
outbound adapter    → application ports, platform
cross-module access → facade only
```

## CI commands

```bash
pnpm lint
pnpm lint:arch
pnpm test
pnpm build

cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

---

# 15. Avoid these patterns

```text
src/types/
src/utils/
src/hooks/
src/components/
```

as a global dumping ground.

```text
src-tauri/src/services/
src-tauri/src/common/
src-tauri/src/models/
src-tauri/src/types/
```

as a place for code with unclear ownership.

Also avoid:

- Blindly creating one code folder per Epic.
- Frontend features importing other features.
- Deep imports that bypass public APIs.
- Scattering `invoke()` across components.
- SQLx queries in Tauri commands.
- Filesystem mutations in repositories.
- Business rules in React components.
- Copying backend data into Zustand.
- Adding a trait for every struct when there is no external boundary.
- Splitting the backend into many Cargo crates now just to look “clean.”

A Cargo workspace is useful when there are multiple packages to manage together; it is not required for a large Rust application. Rust module privacy is already strong enough to enforce modular-monolith boundaries at this stage.

---

# 16. Safest migration order

1. **Document dependency rules** and add architecture linting first.
2. Create `shared/api/tauri`, then prohibit direct `invoke()` calls elsewhere.
3. Move the clear frontend pages: Dashboard, Settings, Browser, Collections.
4. Move Folder Grid, Object List, and Preview into widgets.
5. Add entity public APIs and remove the root `types` directory.
6. Backend: migrate `storage_optimizer` as a pilot because its domain boundary is clear.
7. Continue with `collections`, `ingestion`, `system`, and `browser`.
8. Migrate `library` and `workspace` last because they are the most complex.
9. Route all destructive operations through `MutationCoordinator`.
10. Remove the legacy `commands/services/repo/common/types` directories once nothing imports them.

Do not perform a big-bang migration. Move one module, add its facade, test it, and then remove the old directory after all consumers have moved.

---

# Architecture standard

Use the following standard in EMMM2 architecture documents:

> **EMMM2 frontend follows Feature-Sliced Design with enforced downward dependencies and slice public APIs. The Rust backend is a domain-oriented modular monolith: each capability owns its application logic, domain rules, inbound adapters, and outbound adapters. Tauri acts as a thin interface and composition layer, while filesystem, SQLite, watcher, network, and process integrations remain replaceable infrastructure adapters.**

Summary:

```text
Frontend:
FSD vertical layers

Backend:
Vertical domain modules
+ ports/adapters inside complex modules

Tauri:
Thin adapter

Disk + SQLite:
Recoverable workflow, journal, reconciliation
```

The target is a **domain-oriented modular monolith** rather than global horizontal backend layers, with the frontend organized into `pages`, `widgets`, `features`, `entities`, and `shared`.
