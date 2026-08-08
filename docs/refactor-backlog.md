# Backend Refactor Backlog

Written to survive a context reset: every item names the file, the line, the
evidence, and how to prove the change worked. Nothing here is a known bug —
those were fixed and pushed. This is the remaining debt, ordered by value.

## Where things stand

Six phases of `docs/architecture-refactor-plan.md` are done and on `main`.
The invariants below are enforced by **eleven gates** in
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
| Repos do not decide identity | no `type_is_authoritative` in `repo/` |
| A stored path is not a filesystem path | `ModFolderPath` (no `AsRef<Path>`) |

### The loop every change runs

```bash
cd src-tauri
cargo clippy --all-targets    # must be 0 warnings
cargo fmt
cargo test --all-targets      # 612 passing as of this writing
cargo test --lib specta       # regenerates src/lib/bindings.gen.ts
```

Frontend, from the repo root — **this project uses pnpm**, not npm:

```bash
pnpm install
node_modules/.bin/tsc --noEmit
node_modules/.bin/vitest run   # 683 passing
```

### One rule, and it keeps paying

**Never trust a green test you have not seen fail.** Break the code
deliberately and confirm the test goes red before believing it.

It has caught five real mistakes across two sessions. Twice a test passed for
the wrong reason — one wrote its fixture into the wrong directory and would
have shipped a no-op guarantee. Three times a function was rewritten and
*nothing at all* was checking the behaviour being preserved, which only became
visible because a deliberate mutation stayed green.

The cheapest form: a small script that applies each plausible mistake to the
file, runs the narrowest test filter, and restores. One caveat learned the
hard way — restore in a `finally`, or an assertion failure mid-run leaves a
mutation sitting in your working tree.

---

## Done

All of Tier 1 except 1.5, all of 3.2, and 2.1.

| Item | Commit |
|---|---|
| 1.1 pair prefilter windowed, 1.2 group bucketing, 1.3 per-snapshot precompute, 3.2 tunables | `c79a094` |
| 1.4 `PreparedTokenFilters` + three tokenizer allocations | `f2a76dd` |
| 2.1 pins for identity and merge policy | `63381f9` |
| 2.1 policy moved to `services/objects/reconcile.rs`, tenth arch gate | `4c97580` |
| 3.1 dedup-test fixture shared, safe-mode propagation covered | `3bc891a` |
| 3.1 sync-test base shared, committed enabled state covered | `5344083` |

Two things worth knowing before reading the sections below, which are kept
for the reasoning rather than as work items:

**Declined: `Cow<'static, str>` on `DupScanSignal`.** 1.3 suggested it to save
four small allocations per candidate pair. It is a serde/specta wire type, and
the allocations it saves are invisible next to BLAKE3-hashing the .dds files
in the same loop. Churning an IPC contract for an unmeasurable win is the
ceremony the KISS rule warns about. Revisit only if a profile says otherwise.

**Mutation testing earned its keep.** Every changed function was checked by
breaking it deliberately and confirming a test went red. On the dedup side all
four mutations were caught. On the tokenizer, three of five were **not** —
case-insensitive path detection and repeated section-prefix stripping had no
coverage at all, and the third turned out to be dead code that has since been
deleted. Rewriting those functions without this step would have shipped two
silent regressions. Do it for anything in Tier 2.

---

## What the last pass turned up

The three parked items were resolved (see "Decisions" below), and preparing
that decision uncovered a bug family worth recording, because the shape
repeats.

**One ambiguous column produced six bugs.** `mods.folder_path` is relative to
the mods root. Six readers treated it as a complete path. Every one failed
identically and *silently*: a relative path resolves against the process
working directory, the existence check says no, and the code takes its
nothing-found branch. "No conflicts", "no duplicates" and "no keybinds" are
all ordinary answers, so nothing ever surfaced.

| Reader | What the user saw | Fixed in |
|---|---|---|
| `conflicts_for_enabled_paths` | conflict count always zero | `481510b` |
| `link_disk_to_db` rename pass | every row looked gone from disk | `a6259dd` |
| `build_candidates` | duplicate scan found **nothing** | `61e6d9a` |
| `resolve_mod_path_for_object` | **deleted the row** on "reveal in Explorer" | `61e6d9a` |
| `get_active_keybindings_service` | overlay keybind list empty | `61e6d9a` |
| scan commit (writer) | wrote absolute, disagreeing with disk reconcile | `a6259dd` |

Two lessons worth more than the fixes:

- **A comment is not a check.** `bulk/attributes.rs` documented the convention
  correctly — "the form the DB stores" — while four neighbours got it wrong.
  The convention only became reliable once `ModFolderPath` had no
  `AsRef<Path>`, at which point the compiler listed every offender.
- **Fixtures that use a shape production never produces prove nothing.** All
  thirteen dedup tests passed over a scanner returning zero folders, because
  every fixture stored an absolute temp path. Registering the relative form
  makes five of them fail against the old code.

If another column ever grows two conventions, the fix is a newtype, not a
sweep — the sweep only finds today's callers.

---

## Tier 1 — performance the user feels

The scanner is the only place with a genuine algorithmic problem. A 10k-mod
library is the design target.

### 1.1 Duplicate-scan pair loop is unbucketed O(N²) — DONE (`c79a094`)

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

### 1.2 Group assembly re-scans every pair per component — DONE (`c79a094`)

`scanner.rs:308` — inside `build_groups`:

```rust
let component_pairs: Vec<_> = pairs.iter()
    .filter(|(left, right, _, _, _)| members.contains(left) && members.contains(right))
```

`members` is a `Vec<usize>`, so this is O(#components × #pairs × |members|).
The union-find `parent` array two lines above already answers it: bucket the
pairs once into `HashMap<root, Vec<&ScoredPair>>` via `find()`, then index.
O(#pairs · α).

### 1.3 Per-pair work that belongs per-snapshot — DONE (`c79a094`)

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

### 1.4 Tokenizer rebuilds its filter sets per INI file — DONE (`f2a76dd`)

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

### 2.1 Object identity and merge policy still live in the repo — DONE (`4c97580`)

Was `src-tauri/src/repo/object_repo/sync.rs`. `ensure_object_exists` decided
identity resolution order, the folder-conflict guard, and which fields a
re-match may overwrite — all inside the data layer.

The rules are now `services/objects/reconcile.rs`. The repo kept
`find_by_name_key`, `find_by_folder_key`, the three field updates,
`backfill_empty_columns` and `insert_object`, and nothing that chooses
between them. `MatchSource` moved to `domain/objects.rs` with them.

Two things the pinning exercise turned up that were not in the plan:

- The lookups selected nine columns and read four. The other five were the
  row's JSON blobs, fetched twice per object to feed a read-back that had
  already been deleted when the backfill moved into SQL.
- An incoming `[]`/`{}` was filtered to NULL before the backfill UPDATE, but
  the `CASE` beside it already refuses to write unless the column still holds
  the sentinel. Neither guard was individually pinnable because either alone
  sufficed; the redundant one is gone.

### 2.2 The import pipeline is still in the command layer — DECLINED

The plan asked for a shared `services/mods/import_service` "so the browser
download path and the drag-drop path run the same code instead of two similar
ones". Reading both, they are not two similar ones.

- `commands/mods/mod_import_cmds.rs::import_mods_from_paths` moves arbitrary
  files into a directory the user picked. No database, no matching, no job.
  It continues past a failed item and returns a per-item `BulkResult`.
- `services/browser/import_service/placement.rs::place_mod` resolves or
  creates the target *object*, writes the canonical match, closes the import
  job, marks the download imported and emits a disk reconcile — one
  transaction over four tables, aborting whole on any failure.
- `ingest_dropped_folders` is a third thing again: it stages loose `.ini` and
  image drops into `.emmm_temp` so Deep Match can see them as folders. They
  never reach the game from there.

What the first two genuinely share is already shared: `arrival::land_disabled`,
the operation lock and the watcher suppression guard. What is left to extract
is "loop, land each, record the outcome" — and the two outcomes are
*continue-on-failure* versus *abort-the-transaction*. An abstraction over that
difference is the premature generalization the KISS rule warns about.

Revisit only if a third caller appears, or if the same bug has to be fixed in
both places.

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

### 3.1 Files over the 350-line rule — PARTLY DONE, target not reachable

The two files the plan named as "roughly 40% repeated fixture boilerplate"
were deduplicated (`3bc891a`, `5344083`). The estimate was wrong, and the
measurement is worth keeping so nobody re-derives it:

| File | Before | After | Tests | Lines per test |
|---|---|---|---|---|
| `dedup/tests/dedup_scanner_tests.rs` | 779 | 654 (+2 tests) | 13 | ~50 |
| `scanner/tests/sync_tests.rs` | 870 | 880 (+1 test) | 11 | ~80 |

`dedup_scanner_tests.rs` had real copy-paste — one 16-line fixture written 18
times — and shed 125 lines. `sync_tests.rs` did not: it is long because it
holds eleven tests that each build a temp tree, a game row and a MasterDB.
Collapsing its literals saved 60 lines, which the new coverage then spent.

**Neither reaches 350, and neither should be split to get there.** Each file
covers one concern. The rule is aimed at production modules where length
signals tangled responsibility; a test file's length signals how much
behaviour is pinned.

The deduplication was worth doing anyway, and not for the line count —
mutating the now-shared fixtures is what exposed two untested behaviours
(safe-mode propagation into duplicate groups, and a scanned mod's enabled
state surviving the commit). Both are covered now. That is the argument for
sharing test fixtures: an inert field is invisible when it is written out
eighteen times and obvious when it is written once.

Remaining production files over the rule, none urgent:

```
470  services/scanner/dedup/signals.rs
451  services/scanner/dedup/scanner.rs
401  domain/errors.rs
387  .../deep_matcher/analysis/content/tokenizer.rs
372  services/scanner/deep_matcher/models/acceptance.rs
371  services/scanner/master_db.rs
358  commands/scanner/deepmatch_scanner_cmds.rs
352  services/scanner/core/walker.rs
```

Much of the growth in the first four is doc comments added during Tier 1
explaining why the windowed prefilter and the prepared filters are shaped the
way they are. Do not trade those for a file split.

### 3.2 Dedup tunables outside the module that claims to hold them — DONE (`c79a094`)

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

Tier 1 (except 1.5), 3.2 and 2.1 are done. What is left, in the order it
is worth doing:

**2.2** is worth doing when the browser download path and the drag-drop path
next need the same fix in two places. Until then it is duplication with a
known shape, which is cheaper to live with than a premature abstraction.

**1.5** stays parked. It needs a product decision — should INIs nested inside
a mod folder contribute hashes and keybinds? — not a refactor. Whoever answers
that owns the commit and its test.

**2.3** stays parked by design. Split a table into a row struct and a DTO when
that table actually diverges, not before.

**3.1** is filler for a slow afternoon, and only the two test files with
named, measured boilerplate (`sync_tests.rs`, `dedup_scanner_tests.rs`) are
worth touching. Deduplicate them; do not split them.

---

## Decisions taken

**1.5 — nested INIs stay out of the harvest.** post_apply keeps two walks: the
conflict scan recurses to depth 3, the KeyViewer harvest reads top-level INIs
only. Merging them would put keybinds from inactive variant subfolders into
the overlay. The asymmetry is deliberate and now pinned by
`a_nested_ini_still_counts_as_a_conflict`. The item is closed, not deferred.

**2.3 — row and wire stay coupled until one diverges.** Split a table into a
row struct plus a DTO the first time a column actually needs to change
independently. Six pre-emptive splits is ceremony.

**2.2 — declined**, see the section above.
