# Kesimpulan final

Setelah saya pelajari ulang struktur aktual, TRD, hubungan antar-Epic, serta dokumentasi resmi FSD, Rust, Cargo, Tauri, TanStack Query, dan Zustand, saya mengoreksi kesimpulan sebelumnya:

> **Backend horizontal murni `commands → services → repo` bukan target terbaik untuk EMMM2 sebesar sekarang.**

Struktur terbaik untuk EMMM2 adalah:

| Bagian                  | Arsitektur target                                                                   |
| ----------------------- | ----------------------------------------------------------------------------------- |
| Frontend                | **Feature-Sliced Design yang benar-benar ditegakkan**                               |
| Backend                 | **Modular Monolith berdasarkan domain/capability**                                  |
| Internal backend module | **Application + Domain + Ports/Adapters**, hanya ketika kompleksitasnya membutuhkan |
| Tauri                   | **Thin inbound adapter dan composition root**                                       |
| Data consistency        | **Recoverable mutation workflow + operation journal + reconciliation**              |
| Async frontend data     | **TanStack Query**                                                                  |
| Client-only state       | **Zustand kecil dan dimiliki domain/widget terkait**                                |

---

# 1. Struktur repository target

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

Dokumentasi proyek berada di `docs/`, sehingga lebih mudah ditemukan oleh developer dan tooling standar.

---

# 2. Frontend target: FSD yang benar

Official FSD menetapkan urutan layer:

```text
app
pages
widgets
features
entities
shared
```

Import hanya diperbolehkan menuju layer yang lebih rendah. Slice pada layer yang sama harus independen, dan akses dari luar slice harus melewati public API-nya. FSD juga menganjurkan segment berdasarkan tujuan seperti `ui`, `api`, `model`, `lib`, dan `config`, bukan generic grouping seperti `components`, `hooks`, atau `types`.

## Struktur frontend yang saya rekomendasikan

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

## Kenapa Folder Grid menjadi Widget?

`FolderGrid` sekarang mencakup:

- Toolbar
- Breadcrumb
- Grid/list display
- Selection
- Drag-and-drop
- Bulk actions
- Navigation
- Conflict handling
- Modal orchestration

Itu merupakan blok UI besar yang menyusun banyak entity dan feature. Definisi tersebut lebih tepat sebagai `widgets/mod-explorer`, bukan `features/folder-grid`. FSD mendefinisikan widget sebagai blok UI besar dan independen, khususnya ketika satu page mempunyai beberapa blok besar seperti sidebar, explorer, dan preview panel.

## Jangan terlalu banyak membuat Feature

Tidak setiap tombol perlu menjadi satu slice. FSD sendiri menyatakan tidak semua hal harus menjadi feature; layer ini paling berguna untuk interaksi penting atau interaksi yang digunakan pada beberapa screen.

Contoh `features/mod/toggle` yang sehat:

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

Internal slice menggunakan relative import. Slice lain hanya boleh:

```ts
import { useToggleMod } from '@/features/mod/toggle';
```

Bukan:

```ts
import { useToggleMod } from '@/features/mod/toggle/model/useToggleMod';
```

Public API harus mengekspos hanya kontrak yang dibutuhkan, bukan `export *`. Official FSD juga memperingatkan wildcard barrel karena memperbesar surface area dan dapat meningkatkan circular import.

---

# 3. Penempatan frontend state

Project saat ini sudah membedakan Zustand untuk state dan TanStack Query untuk async state. Namun root `stores/` saat ini terlalu mudah menjadi pemilik semua state.

Gunakan aturan berikut:

| Jenis state                                    | Pemilik                       |
| ---------------------------------------------- | ----------------------------- |
| Mods, objects, collections, settings dari Rust | TanStack Query                |
| Loading/error/result async                     | TanStack Query                |
| Active game session                            | `entities/game/model`         |
| Privacy mode session                           | `entities/privacy-mode/model` |
| Grid selection                                 | `widgets/mod-explorer/model`  |
| Preview draft                                  | `widgets/mod-preview/model`   |
| Form input sementara                           | Komponen/feature terkait      |
| Theme dan lifecycle provider                   | `app`                         |
| Toast                                          | `shared/ui/toast`             |
| Modal bisnis                                   | Feature/widget pemiliknya     |

TanStack Query ditujukan untuk server/async state, sementara Zustand adalah client-state manager. Dokumentasi TanStack menyebut bahwa setelah async state dipindahkan ke Query, global client state biasanya menjadi jauh lebih kecil.

Jadi jangan menyimpan ulang:

```ts
mods;
objects;
collections;
downloads;
dashboardStats;
settings;
```

di Zustand apabila data yang sama sudah berada di backend dan TanStack Query.

---

# 4. Backend target: Modular Monolith, bukan horizontal global layers

Struktur sekarang:

```text
commands/
domain/
repo/
services/
```

sudah lebih baik daripada struktur awal TRD, tetapi pada skala saat ini menyebabkan satu perubahan feature tersebar ke empat pohon besar. TRD awal memang mendefinisikan `commands`, `database`, dan `services` sebagai horizontal layers.

Target yang lebih cocok:

```text
src-tauri/src/
├── app/
├── modules/
├── platform/
├── shared/
├── lib.rs
└── main.rs
```

## Struktur backend final

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

# 5. Pembagian backend module

| Module              | Tanggung jawab                                                     |
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

Ini sengaja **tidak sama persis dengan 13 Epic**.

Contohnya:

- Epic 4 Folder Grid sebagian besar berada di frontend widget.
- Epic 5 Core Operations berada di `library/application`.
- Epic 6 Preview berada di frontend widget, tetapi INI dan metadata berada di `library`.
- Epic 12 metadata sync masuk `catalog`, sedangkan app updater masuk `system`.
- Epic 13 Dashboard cukup menjadi read-model module dan tidak membutuhkan domain model kompleks.

---

# 6. Isi sebuah backend module

Contoh module kompleks `library`:

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
│   ├── commands/                  # Business mutations, bukan Tauri commands
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

Alur dependensinya:

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

Ini merupakan kombinasi vertical module dan Ports & Adapters: code dikelompokkan berdasarkan capability, sementara external systems tetap berada di luar application core. Vertical Slice mengelompokkan code berdasarkan axis of change, sedangkan Ports & Adapters memisahkan application core dari database, filesystem, UI, dan environment eksternal.

## Jangan paksa semua module mempunyai semua folder

Untuk `dashboard`, struktur sederhana cukup:

```text
modules/dashboard/
├── query.rs
├── dto.rs
├── sqlx_adapter.rs
├── tauri.rs
└── mod.rs
```

Dashboard adalah read model. Tidak perlu membuat:

```text
domain/
ports/
repositories/
services/
factories/
```

apabila tidak ada domain rule yang membutuhkan itu.

---

# 7. Backend public API

Rust private by default. Gunakan itu sebagai architecture enforcement, bukan menjadikan semua module `pub`. Official Rust documentation memang menjadikan implementation detail private secara default dan mendukung `pub use` untuk membuat public interface yang terpisah dari internal structure.

Contoh:

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

Module lain boleh:

```rust
use crate::modules::library::LibraryApi;
```

Tetapi tidak boleh:

```rust
use crate::modules::library::adapters::outbound::sqlx_mod_repository;
use crate::modules::library::application::commands::toggle_mod;
```

Cross-module operation harus melewati `facade.rs`.

---

# 8. Tauri command harus sangat tipis

Tauri commands adalah inbound adapter, bukan tempat business logic.

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

Yang tidak boleh ada di command:

```rust
sqlx::query!(...)
std::fs::rename(...)
WalkDir::new(...)
notify(...)
blake3::hash(...)
```

Tauri mendukung command async dan merekomendasikannya untuk heavy work agar UI tidak freeze. Untuk progress streaming seperti scan, import, atau hashing, gunakan channel; event lebih cocok untuk small broadcast dan tidak dirancang untuk high-throughput streaming.

Gunakan typed error:

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

Jangan hanya:

```rust
Result<T, String>
```

Tauri mendukung custom serializable error representation sehingga frontend bisa memetakan error berdasarkan code, bukan parsing teks.

---

# 9. Operation lock perlu dipindahkan

TRD lama mengatakan:

> Operation lock diperoleh di Command layer agar services reusable.

Untuk struktur EMMM2 sekarang, aturan tersebut sudah tidak cukup karena mutation dapat dipicu oleh:

- Tauri command
- Hotkey
- Collection apply
- Randomizer
- Background recovery
- Watcher reconciliation
- Startup repair

Kalau lock hanya berada di Tauri command, entrypoint lain dapat melewatinya.

## Target baru

```text
app/runtime/mutation_coordinator.rs
```

Semua destructive mutation harus melalui:

```rust
pub struct MutationCoordinator {
    lock: tokio::sync::Mutex<()>,
    journal: OperationJournal,
}
```

Alur:

```text
Tauri / Hotkey / Recovery
          ↓
MutationCoordinator
          ↓
Module use case
          ↓
Filesystem + SQLite
```

Command hanya menerjemahkan `OperationBusy` menjadi toast/error DTO.

---

# 10. Disk dan SQLite jangan disebut satu transaksi atomik

EMMM2 mempunyai tiga bentuk truth:

- Disk sebagai physical truth.
- SQLite sebagai logical index.
- `info.json` sebagai portable metadata.

SQLite transaction tidak dapat secara otomatis me-rollback Windows filesystem rename. Karena itu, workflow terbaik bukan mengklaim keduanya sebagai satu transaksi, tetapi menggunakan **recoverable mutation**:

```text
1. Validate request
2. Build mutation plan
3. Write journal: PENDING
4. Execute filesystem operations
5. Persist SQLite projection
6. Mark journal: COMPLETED
7. Emit frontend refresh
```

Kalau crash:

```text
Startup
   ↓
Read incomplete journal
   ↓
Inspect disk state
   ↓
Complete, compensate, atau reconcile
```

Struktur:

```text
app/runtime/
├── mutation_coordinator.rs
├── operation_journal.rs
├── recovery_runner.rs
└── task_registry.rs
```

Sedangkan:

```text
modules/workspace/
├── application/
│   └── reconcile/
└── adapters/
    └── watcher_projection.rs
```

Ini lebih sesuai dengan hybrid truth model EMMM2 dan struktur `recovery`, `disk_reconcile`, serta `workspace_mutation` yang sebenarnya sudah mulai terbentuk di code saat ini.

---

# 11. Pemetaan frontend sekarang ke target

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
| `src/shared/components/layout`              | `src/widgets/app-shell` dan `top-bar`                      |
| `src/core/tauri`                            | `src/shared/api/tauri`                                     |
| `src/core/lib/queryClient`                  | `src/app/providers`                                        |
| `src/core/lib/i18n` + `src/locales`         | `src/shared/i18n`                                          |
| `src/features/file-watcher`                 | `src/app/runtime` + `features/library/reconcile`           |
| `src/features/runtime-sync`                 | `src/app/runtime/queryEventBridge`                         |
| `src/features/workspace-runtime`            | `entities/workspace`, widget model, dan reconcile features |
| `src/stores`                                | Didistribusikan ke owner masing-masing                     |
| `src/types`                                 | Didistribusikan ke `entities/*/model`                      |
| Generic `components/hooks/utils`            | `ui/model/lib/api` berdasarkan tanggung jawab              |

---

# 12. Pemetaan backend sekarang ke target

| Current                               | Target                                                                 |
| ------------------------------------- | ---------------------------------------------------------------------- |
| `commands/*`                          | `modules/*/adapters/inbound/tauri.rs`                                  |
| `repo/*`                              | `modules/*/adapters/outbound/sqlx_*.rs`                                |
| `domain/*`                            | `modules/*/domain`                                                     |
| Business code dalam `services/*`      | `modules/*/application`                                                |
| Generic I/O dalam `services/fs_utils` | `platform/fs`                                                          |
| `services/images`                     | `platform/image` atau library outbound adapter                         |
| `services/update`                     | `modules/system` dan `modules/catalog`                                 |
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
| `pipeline/*`                          | `modules/collections/application/apply` jika hanya collection pipeline |
| `common/*`                            | Module pemilik atau `shared` jika benar-benar generik                  |
| `types/dup_scan.rs`                   | `modules/storage_optimizer/domain` atau `dto`                          |

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

Rust membedakan unit test yang berada dekat module dan dapat menguji private implementation, dengan integration test di directory `tests/` yang hanya memakai public interface.

Architecture test yang sudah ada seperti:

```text
arch_audit.rs
dal_audit.rs
```

sebaiknya dipertahankan dan diperluas untuk melarang:

```text
domain -> sqlx
domain -> tauri
application -> tauri
application -> notify
module A -> internal module B
```

---

# 14. Aturan architecture yang harus masuk CI

## Frontend

```text
app      → pages, widgets, features, entities, shared
pages    → widgets, features, entities, shared
widgets  → features, entities, shared
features → entities, shared
entities → shared
shared   → shared only
```

Tambahkan architecture lint. Dokumentasi FSD merekomendasikan architectural linter untuk mendeteksi deep import dan pelanggaran public API.

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

# 15. Jangan lakukan ini

```text
src/types/
src/utils/
src/hooks/
src/components/
```

sebagai global dumping ground.

```text
src-tauri/src/services/
src-tauri/src/common/
src-tauri/src/models/
src-tauri/src/types/
```

sebagai tempat semua code yang tidak jelas pemiliknya.

Juga hindari:

- Satu folder code per Epic secara buta.
- Feature frontend mengimpor feature lain.
- Deep import melewati public API.
- `invoke()` tersebar di component.
- SQLx query di Tauri command.
- Filesystem mutation di repository.
- Business rule di React component.
- Menyalin backend data ke Zustand.
- Trait untuk setiap struct meskipun tidak ada external boundary.
- Memecah backend menjadi banyak Cargo crates sekarang hanya agar terlihat “clean”.

Cargo workspace berguna ketika memang terdapat beberapa package yang perlu dikelola bersama, bukan kewajiban untuk aplikasi Rust besar. Rust module privacy sudah cukup kuat untuk menegakkan batas modular-monolith pada tahap ini.

---

# 16. Urutan migrasi paling aman

1. **Dokumentasikan dependency rules** dan tambahkan architecture lint terlebih dahulu.
2. Buat `shared/api/tauri`, lalu larang `invoke()` langsung di tempat lain.
3. Pindahkan frontend page yang sudah jelas: Dashboard, Settings, Browser, Collections.
4. Pindahkan Folder Grid, Object List, dan Preview menjadi widgets.
5. Tambahkan entity public APIs dan hilangkan root `types`.
6. Backend: migrasikan `storage_optimizer` sebagai pilot karena batas domainnya jelas.
7. Lanjutkan `collections`, `ingestion`, `system`, dan `browser`.
8. Migrasikan `library` dan `workspace` terakhir karena paling kompleks.
9. Masukkan semua destructive operations ke `MutationCoordinator`.
10. Hapus folder legacy `commands/services/repo/common/types` setelah tidak memiliki import.

Jangan melakukan big-bang migration. Satu module dapat dipindahkan, diberi facade, diuji, lalu folder lama dihapus setelah semua consumer berpindah.

---

# Rekomendasi yang dipakai sebagai standar resmi

Gunakan kalimat ini di dokumen architecture EMMM2:

> **EMMM2 frontend follows Feature-Sliced Design with enforced downward dependencies and slice public APIs. The Rust backend is a domain-oriented modular monolith: each capability owns its application logic, domain rules, inbound adapters, and outbound adapters. Tauri acts as a thin interface and composition layer, while filesystem, SQLite, watcher, network, and process integrations remain replaceable infrastructure adapters.**

Versi sederhananya:

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

**Ini target terbaik untuk EMMM2.** Struktur sekarang sudah menjadi fondasi yang cukup baik, tetapi final state sebaiknya bergerak dari **global horizontal backend layers** menuju **domain-oriented modular monolith**, sementara frontend dipisahkan secara benar menjadi `pages`, `widgets`, `features`, `entities`, dan `shared`.
