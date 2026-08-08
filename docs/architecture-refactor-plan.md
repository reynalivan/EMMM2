# Backend Overhaul Plan (pre-release — no compatibility constraints)

The app has not shipped. That changes the strategy: **no shims, no gradual
baselines, no deprecation windows.** Each area is rewritten straight to its
target shape, bindings break freely (regenerate + fix frontend call sites in
the same commit), and dead code is deleted, not kept "just in case".

What does NOT change: the tree is always green. One workstream at a time,
each step ends with `cargo clippy --all-targets`, full `cargo test`, bindings
regen, `/simplify` on the step's diff, and `/code-review` (`ultra` for steps
1, 3 and 4 — privacy gate and mutation ordering). Behavior that must survive
gets pinned by a test *before* the code moves, not after.

## Best-practice evidence (Context7, Aug 2026)

- **Tauri v2, error handling** (v2.tauri.app/develop/calling-rust): the
  official pattern is a `thiserror` enum with a manual `serde::Serialize`
  emitting a tagged object — `#[serde(tag = "kind", content = "message")]` —
  so the frontend switches on `e.kind`. This is the model for `AppError` and
  every subsystem error below.
- **Tauri v2, async commands**: async commands already run on a separate
  task, but *blocking* work (filesystem walks, archive extraction) still
  stalls a Tokio worker — route it through `spawn_blocking`. `.setup()` must
  return before the app is usable; nothing slow belongs in it.
- **sqlx**: prepared statements are cached per connection (LRU, default 100
  entries) and re-prepared on other connections on first use. Consequence:
  hot-path SQL must be a **fixed string** — every `format!`-assembled variant
  is a cache miss and a fresh parse.

---

## Target architecture (the contract everything is rewritten toward)

```
commands/   IPC adapters only. Parse input → resolve Corridor → acquire
            OpGuard (if mutating) → call ONE service fn → typed error out.
            No fs. No SQL. No orchestration. ~20 lines per command.

services/   All domain logic and orchestration. Typed errors only
            (one thiserror enum per subsystem, #[from] into AppError).
            Blocking fs work wrapped in spawn_blocking.

repo/       SQL only. No std::fs, no policy, no specta. Returns domain
            types. Fixed SQL text on hot paths.

domain/     Types that cross layers, incl. ALL IPC wire types (the only
            place specta::Type appears) and the capability types below.
```

Invariants become **types**, not comments:

| Type | Kills |
|---|---|
| `Corridor` (Copy enum, *not* serde/specta) | Safe Mode as a client-supplied bool |
| `OpGuard` (private field, only `OperationLock::acquire` builds it) | lock acquired at two altitudes; re-entrant deadlock |
| `ValidatedPath` (only `fs_utils::guard` builds it) | raw client paths reaching `std::fs` |
| `#[must_use] MutationOutcome` | "callers MUST remember to refresh the projection" |

Enforced by `tests/arch_audit.rs` (same shape as the existing `dal_audit.rs`)
— but written with **target values (0), landed at the end of each step**, not
shrinking baselines. Pre-release means there is nothing to ratchet down from;
the sweep goes to zero in the step itself.

---

## Step 1 — Corridor + capability types (foundations)

Three small, independent commits.

**1a. `Corridor`.** `domain/corridor.rs`: `enum Corridor { Safe, Unsafe }`,
deliberately not serializable. `ConfigService::current_corridor()` folds
`safe_mode.enabled` + PIN elevation (the logic inlined today in
`dup_scan_cmds.rs:197-215` moves here and is deleted there). Every command
drops its `safe_mode: bool` / `is_safe: Option<bool>` parameter
(`dashboard_cmds.rs:11`, `object_cmds.rs:46`, `collections/cmds.rs:23,53,199`,
`mod_meta_cmds.rs:36`); `ObjectFilter` loses `safe_mode` from the wire.
Frontend stops sending the flag — same commit, regenerated bindings.
Smoke-test the corridor end to end (workspace, dashboard, dup-scan, folder
grid, collections, PIN elevation) before merging.

**1b. `OpGuard`.** Wrap the `OwnedMutexGuard<()>`; every fs-mutating service
fn takes `_guard: &OpGuard` first. All 11 service-side `acquire()` calls move
up into their commands — one altitude, by construction.

**1c. `ValidatedPath`.** `validate_path` returns it; services touching disk
for a command require it. The eight commands that today accept raw client
paths (`archive_cmds.rs:44,56,132`, `scanner/conflict_cmds.rs:36`,
`dup_scan_cmds.rs:55`, `folder_grid/mod.rs:16`, `app_cmds.rs:83,91`) get
routed through the guard. Add `validate_paths(&[String])` (canonicalize the
mods root **once**) and use it in `bulk_toggle_mods`/`bulk_delete_mods` —
deletes the 500-canonicalizations-per-bulk-toggle cost as a side effect.
Open decision, do not guess: `ensure_dir_cmd` currently `create_dir_all`s an
arbitrary client path — constrain it to app-data/mods roots or delete it.

---

## Step 2 — Error sweep: `Result<_, String>` is removed, not deprecated

One subsystem per commit, converted completely, String signature deleted:

1. **browser** — 17 commands of `.map_err(AppError::Internal)`; `BrowserError`
   enum, mechanical.
2. **scanner** (core/sync/dedup/deep_matcher) — `ScannerError`.
3. **keyviewer + post_apply** — `KeyviewerError`; `run_post_apply_tasks`'s
   callers all `let _ =` today, so typing it is free.
4. **remaining tail** (`images`, `game`, `explorer` string paths).

Rules: every enum is `thiserror` + `#[from]` into `AppError`; `AppError`'s
serialization follows the official tagged `{kind, message}` shape so the
frontend switches on `kind` (it already keys the retry dialog on `FileInUse`
— this makes that pattern universal). Fs rename/remove errors route through
`map_toggle_error` so `FileInUse`/`PathBusy` classification is never
stringified away. `arch_audit`: zero `Result<_, String>` in `services/` at
the end of this step.

---

## Step 3 — Mutation finalizer

`#[must_use] MutationOutcome { scope: RefreshScope /* Objects(Vec<id>) |
FullGame */ }` returned by every mutation service; a single
`finalize_mutation(...)` consumes it, owning the projection refresh (scoped
`refresh_projection_for_object_ids` when ids are known — today's blanket
`rebuild_game_projection` in post_apply becomes the fallback, not the norm)
and the runtime side effects, with errors logged once instead of `let _ =`
at five call sites (`conflict_cmds.rs:69-80`, `bulk/toggle.rs:107-112`,
`bulk/delete.rs:116-117`, `scanner/conflict/duplicates.rs:219-220`,
`mod_bulk_cmds.rs:86-96`).

`rebuild_game_projection`/`refresh_*` go `pub(crate)`-private to the
finalizer module. Riders: `PostApplyContext.settings: AppSettings` (full
deep clone per mutation) shrinks to the `hotkeys` it reads;
`status_fields: Option<_>` becomes explicit `PresetSource::Known | Derive`.
**Pin first:** snapshot the emitted event order in a test before touching
the sequence.

---

## Step 4 — Repo purge

**4a. `object_repo/counts.rs`** — ~250/297 lines are domain logic;
`classify_terminal_type` does `read_dir` + INI parsing inside the data
layer, per row per ancestor, unmemoized. Move terminal resolution to
`services/objects/terminal.rs`; add the `HashMap<PathBuf, Option<NodeType>>`
memo while moving (sibling mods share ancestors); wrap the walk in
`spawn_blocking`. Repo keeps the row query only.

**4b. `object_repo/sync.rs`** — `ensure_object_exists` owns identity order,
conflict policy, and uses `db_thumbnail.is_some()` as a proxy for "MasterDB
match". Repo shrinks to `find_by_name_key` / `find_by_folder_key` /
`insert` / field updates; `services/objects/reconcile.rs` owns resolution
order with an explicit `MatchSource::MasterDb | Disk`. **Pin first** with
fixtures for both match arms and the conflict case. Rider: the per-column
backfill becomes one fixed-text
`UPDATE … SET col = COALESCE(col, ?), …` — one round trip, statement-cache
friendly (see sqlx evidence).

**4c. specta out of `repo/`** — the six repo files deriving `specta::Type`
move their wire types to `domain/`; repos map rows into them. No third
mapping layer: wire type = domain type where the shapes already coincide
(KISS — a DTO per table is over-engineering; the boundary rule is only
"repo row structs are not IPC types").

`arch_audit`: zero `std::fs` under `repo/`, zero `specta` under `repo/`.

---

## Step 5 — Command flattening + startup

- **`resolve_conflict`** (`conflict_cmds.rs:23-83`) — the lock → rename →
  repo → collection-heal → finalize orchestration moves beside
  `core_ops/rename.rs:185`, which already does this sequence correctly.
- **One import pipeline** — `services/mods/import_service` owns per-item
  outcome, disabled-on-arrival (via `standardize_prefix`, deleting the four
  hand-built `DISABLED `-prefix sites) and progress; the browser download
  path (`import_service/placement.rs`) and drag-drop path share it.
- **MasterDB never crosses IPC again** — delete `db_json: String` from all
  five commands; `master_db::get_cached(app, game_type) -> Arc<MasterDb>` in
  the service layer (the existing `MasterDbCache` generalized). Removes a
  ~5 MB round trip *and* five re-parses. Frontend stops threading the blob.
- **Startup**: `.setup()` keeps only `purge_old_tasks` + transfer recovery
  (two fast UPDATEs); `reconcile_disk_state` moves to
  `tauri::async_runtime::spawn`, reporting through its existing progress
  events. Window first, 10k-mod walk second.
- Blocking walks reached from async commands (`post_apply` harvest, archive
  extraction) get `spawn_blocking` per the Tauri guidance.

`arch_audit`: commands contain no `crate::repo::` mutation calls, no
`std::fs` outside the guard module.

---

## Step 6 — Perf pass (structure is now fixed, optimize inside it)

- Dedup candidate filtering: sort snapshots by file count once, walk a ≤4
  window — O(N log N) replaces the 50M-iteration pair loop at
  `dedup/scanner.rs:271-290`.
- `PreparedTokenFilters` built once per scan instead of four `BTreeSet`
  rebuilds per INI (`tokenizer.rs:91-94`).
- Per-snapshot precomputes (`file_set`, `normalized_name`,
  `version_stripped_name`) instead of per-pair recomputation in
  `signals.rs:288-314`.
- Group assembly via union-find roots instead of the quadratic
  `members.contains` scan (`scanner.rs:313-316`).
- Move the dedup tunables (`45`, `> 4`, `0.70`, `>= 80`) into the
  `mod weights` block that documents itself as holding all of them.

---

## Explicitly deferred (feature work, not overhaul)

- True rollback (`tasks.snapshot_json` column).
- `corridor_state.active_collection_id` write path.
- Unifying the four corridor SQL predicates — they differ *semantically*
  (dashboard has no manual/unknown escape hatch); unification would change
  counts. Documented at each site instead.

## Order & effort

1 → 2 → 3 → 4 → 5 → 6, strictly serial (later steps rewrite call sites the
earlier ones create). Rough effort: 1: ~2d · 2: ~2d · 3: 1–2d · 4: 2–3d ·
5: ~2d · 6: 1–2d. Every step independently mergeable and revertable.
