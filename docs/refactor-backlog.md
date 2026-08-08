# Backend Refactor Backlog

Written to survive a context reset: every item names the file, the line, the
evidence, and how to prove the change worked. Nothing here is a known bug —
those were fixed and pushed. This is the remaining debt, ordered by value.

## Where things stand

Six phases of `docs/architecture-refactor-plan.md` are done and on `main`.
The invariants below are enforced by **nine gates** in
`src-tauri/tests/arch_audit.rs`, all target-zero. Read that file first: it is
the fastest way to learn what the layers are allowed to do.

| Invariant | Type that enforces it |
|---|---|
| Safe Mode is server-derived | `Corridor` (not serde/specta) |
| The operation lock is held | `OpGuard` (private field) |
| A path is inside a mods root | `ValidatedPath` (only `fs_utils::guard` builds it) |
| A mutation settles the projection | `#[must_use] MutationOutcome` + `finalize_mutation` |
| Errors keep their discriminant | no `Result<_, String>` in `services/` |
| Repos only do SQL | no `std::fs`, no `specta::Type` in `repo/` |

### The loop every change runs

```bash
cd src-tauri
cargo clippy --all-targets    # must be 0 warnings
cargo fmt
cargo test --all-targets      # 591 passing as of this writing
cargo test --lib specta       # regenerates src/lib/bindings.gen.ts
```

Frontend, from the repo root — **this project uses pnpm**, not npm:

```bash
pnpm install
node_modules/.bin/tsc --noEmit
node_modules/.bin/vitest run   # 683 passing
```

### One rule that caught two real mistakes in the last session

**Never trust a green test you have not seen fail.** Both times a fix was
written here, temporarily reverting the fix and re-running proved the test
actually exercised it. One test passed for the wrong reason (it wrote its
fixture into the wrong directory) and would have shipped a no-op guarantee.
Budget the extra minute.

---

## Tier 1 — performance the user feels

The scanner is the only place with a genuine algorithmic problem. A 10k-mod
library is the design target.

### 1.1 Duplicate-scan pair loop is unbucketed O(N²)

`src-tauri/src/services/scanner/dedup/scanner.rs:266` —
`phase1_candidate_filtering`:

```rust
for left in 0..snapshots.len() {
    for right in (left + 1)..snapshots.len() {
        if first.files.len().abs_diff(second.files.len()) > 4 { continue; }
        if super::size_ratio(..) < 0.70 { continue; }
```

10k mods is ~50M iterations, serial, while the phases around it use
`par_iter`. Both surviving predicates are **1-D range tests**, so the pair
set can be enumerated instead of filtered:

- Sort snapshot indices once by `files.len()`.
- For each index, walk forward only while the file-count delta is ≤ 4.
- Apply the size-ratio test inside that window.

O(N log N + N·k) where k is the window width. Note the sort must be by file
count, not by size, because the file-count bound is the tighter one.

**Verify:** a synthetic snapshot set where the expected pair list is known.
Assert the new function returns the same pairs as the old nested loop for a
few hundred randomised snapshots — a property test against the naive
implementation is the honest check here, not a hand-written expectation.

**Watch for:** `pairs.push((left, right))` currently emits `left < right`.
Downstream `canonical_pair` and `build_groups` assume nothing about order,
but keep the invariant anyway to avoid surprises.

### 1.2 Group assembly re-scans every pair per component

`scanner.rs:308` — inside `build_groups`:

```rust
let component_pairs: Vec<_> = pairs.iter()
    .filter(|(left, right, _, _, _)| members.contains(left) && members.contains(right))
```

`members` is a `Vec<usize>`, so this is O(#components × #pairs × |members|).
The union-find `parent` array two lines above already answers it: bucket the
pairs once into `HashMap<root, Vec<&ScoredPair>>` via `find()`, then index.
O(#pairs · α).

### 1.3 Per-pair work that belongs per-snapshot

`src-tauri/src/services/scanner/dedup/signals.rs`:

- `:296` — `phase2_name_and_structure` builds a `BTreeSet<&str>` of every
  file `rel_path` for **both** snapshots, on **every pair**. A mod in k pairs
  builds its set k times.
- `:203` and `:289` — `strip_version` (regex `replace_all` + `to_string` +
  `replace` + `to_lowercase`) and `normalize_name` run per pair on
  `display_name`.

Add `file_set: BTreeSet<String>`, `normalized_name: String` and
`version_stripped_name: String` to `ModSnapshot`, computed once in
`collect_snapshot` — which already walks the files.

Also: `build_signal` allocates two `String`s from `&'static str` literals
4–5 times per pair. `DupScanSignal.key`/`.detail` want `Cow<'static, str>`.

### 1.4 Tokenizer rebuilds its filter sets per INI file

`src-tauri/src/services/scanner/deep_matcher/analysis/content/tokenizer.rs:91`
— `extract_structural_ini_tokens` calls `merged_stopwords`,
`normalized_set`, `merged_key_blacklist`, `merged_key_whitelist` on entry.
They depend only on `&IniTokenizationConfig`, which is constant for a whole
scan, yet each call allocates ~40 `String`s and four B-trees. 10k mods × ~3
INIs is over a million throwaway allocations.

Build a `PreparedTokenFilters` once via `IniTokenizationConfig::prepare()`
and pass it by reference.

Same file, smaller:
- `:331` `looks_like_path` lowercases the whole RHS just to run `contains`.
- `:313` `strip_section_prefixes` re-lowercases and reallocates per stripped
  prefix; track a `usize` offset into one lowercase copy.
- `:247` `tokenize_structural` returns `Vec<String>`, then `insert_tokens`
  drops most of them. Yield `&str` and `to_string()` only on insert.

### 1.5 post_apply still walks each enabled mod twice

`src-tauri/src/services/app/post_apply.rs`. Already reduced from three walks
to two and three INI reads to two (`harvester::harvest_mod` does one pass).
What remains: the conflict scan uses `walker::scan_folder_content(path, 3)`
(recursive, depth 3) while the harvest uses `list_ini_files` (top level
only).

**This one is not a pure refactor.** Merging the walks changes *which* INI
files get harvested — nested INIs would start contributing hashes and
keybinds. Decide the intended semantics first; if nested INIs should count,
that is a behaviour change worth its own commit and its own test.

---

## Tier 2 — architecture debt named in the plan

### 2.1 Object identity and merge policy still live in the repo

`src-tauri/src/repo/object_repo/sync.rs` (189 lines) —
`ensure_object_exists` decides identity resolution order (match by
`name_key`, else by `folder_path_key`), conflict handling, and which fields
a re-match may overwrite. That is domain policy inside the data layer.

Already done: `MatchSource::MasterDb | Disk` replaced the
`db_thumbnail.is_some()` proxy, and the five conditional backfill UPDATEs
became one fixed-text `COALESCE`/`CASE` statement.

Remaining: repo exposes `find_by_name_key`, `find_by_folder_key`,
`insert_object`, field updates; a new `services/objects/reconcile.rs` owns
the resolution order.

**Pin first.** `services/scanner/tests/sync_tests.rs` already contains
`test_ensure_object_case_insensitive_merge`, which encodes a real intent: a
canonical match (`matched_entry_key`) must enrich `object_type`. Add
fixtures for the name-match arm, the folder-match arm, and the
`has_folder_conflict` case before moving anything.

### 2.2 The import pipeline is still in the command layer

`src-tauri/src/commands/mods/mod_import_cmds.rs` (208 lines) emits progress
events, loops sources, detects archive format and routes extraction.

Already done: the "new arrivals land disabled" rule is shared —
`services/mods/arrival.rs::land_disabled`, used by both this path and
`services/browser/import_service/placement.rs`.

Remaining: a `services/mods/import_service` owning per-item outcome and
progress, so the browser download path and the drag-drop path run the same
code instead of two similar ones.

### 2.3 Row shape is still the wire contract

Six files under `src-tauri/src/domain/` derive both `sqlx::FromRow` and
`specta::Type`: `objects.rs`, `dashboard.rs`, `browser.rs`, `conflicts.rs`,
`collection.rs`, `models.rs`. Moving them out of `repo/` fixed *ownership*,
not *coupling* — renaming a column is still a frontend breaking change.

**This is a deferred decision, not debt to pay down blindly.** Splitting a
table into a row struct plus a DTO is worth doing **per table, when one
actually diverges**. Doing all six pre-emptively is the ceremony the KISS
rule warns about. Revisit when a column rename is actually needed.

---

## Tier 3 — hygiene

### 3.1 Twenty files over the 350-line rule

```
870  services/scanner/tests/sync_tests.rs
779  services/scanner/dedup/tests/dedup_scanner_tests.rs
566  commands/objects/tests/object_cmds_tests.rs
521  services/scanner/deep_matcher/tests/required_tests.rs
493  services/mods/archive/tests/mod_tests.rs
457  services/objects/tests/query_tests.rs
449  services/scanner/dedup/tests/dedup_resolver_tests.rs
446  services/scanner/dedup/signals.rs
412  commands/mods/tests/mod_meta_cmds_tests.rs
408  services/scanner/dedup/scanner.rs
408  repo/tests/dashboard_repo_test.rs
401  domain/errors.rs
387  services/objects/tests/mutate_tests.rs
372  services/scanner/deep_matcher/models/acceptance.rs
371  services/scanner/master_db.rs
370  repo/tests/mod_repo_test.rs
358  commands/scanner/deepmatch_scanner_cmds.rs
356  services/scanner/deep_matcher/tests/models/acceptance_tests.rs
354  .../deep_matcher/analysis/content/tokenizer.rs
352  services/scanner/core/walker.rs
```

Eleven are test files. **Deduplicate, do not split** — the two largest each
cover one concern and are roughly 40% repeated fixture boilerplate:

- `sync_tests.rs`: `ConfirmedScanItem` is written out inline 16 fields at a
  time in six places despite a `confirmed_scan_item(..)` helper existing at
  the top; `CommitScanRequest` is spelled nine times with only `items`
  varying. Add `#[derive(Default)]` to `ConfirmedScanItem`
  (`services/scanner/sync/types.rs:65` — every field is
  `String`/`bool`/`Option`) and route the literals through
  `..Default::default()`. Roughly 350 lines.
- `dedup_scanner_tests.rs`: the same 15-line `insert_test_mod` /
  `TestModFixture` block appears 18 times, differing only in `id`,
  `actual_name` and `folder_path`. One `register_mod(..)` helper takes it to
  ~380 lines.

Four of these were grown during the last session and are fair game:
`domain/errors.rs` (two error enums + nine `From` impls),
`master_db.rs` (the cache), `deepmatch_scanner_cmds.rs`, and
`repo/tests/mod_repo_test.rs`.

### 3.2 Dedup tunables outside the module that claims to hold them

`signals.rs:138` opens `mod weights` with "Every tunable in the
duplicate-similarity model, in one place". These are not in it:

- `scanner.rs:121` — accept cutoff `score < 45`
- `scanner.rs:272` — file-count window `> 4`
- `scanner.rs:275` — size-ratio floor `< 0.70`
- `signals.rs:277` — `logical_overlap > 0.8`, while `:232` uses
  `w::LOGICAL_OVERLAP_BONUS_MIN` for the same threshold
- `signals.rs:279` — `score >= 80`

Move them into `mod weights` and make `:277` reuse the existing constant.
Retuning the model currently means grepping two files for naked numbers.

Also `signals.rs:10`: `TEXTURE_EXTS: &[&str] = &["dds"]` is a one-element
slice used with `.contains()`; `const TEXTURE_EXT: &str = "dds"` and `==`
says it better. The sibling `["ib", "buf"]` at `:112` is an inline literal
that wants a name next to it.

---

## Suggested order

1.1 first — it is the only item a user can feel, it is localised, and a
property test against the current naive loop makes it provable. Then 1.3 and
1.4 (both pure allocation removal, both cheap). 1.2 is small enough to ride
along with 1.1.

Leave 1.5 alone until someone decides whether nested INIs should count.

Tier 2 items each need their pinning tests written first; 2.1 is the most
valuable and the most delicate. Tier 3 is filler for a slow afternoon,
except 3.2, which is a five-minute change that makes the dedup model
tunable without a treasure hunt.
