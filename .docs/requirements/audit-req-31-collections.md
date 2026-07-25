# Audit — Epic 31 Virtual Collections (Loadouts)

**Scope:** `src/features/collections`, `src-tauri/src/commands/collections`, and their service/repo/pipeline dependencies, against `req-31-collections.md`.
**Method:** three parallel layer audits (backend Rust, frontend React, cross-cutting integration seams). Every claim below carries `file:line` evidence.
**Verdict:** Feature is largely functional and the hard parts (shared mutation engine, preview-tree semantics, missing-mods flow, recovery dialog) are wired. The real problems are **spec drift**, **one security gap**, and **~450+ lines of dead scaffolding from an abandoned model swap**. This is a cleanup/consolidation job, not a rebuild.

---

## Fix log (2026-07-10)

Model decision (H1): **commit to the live signature model.** Corridor-guard decision (H2 apply/preview): **enforce rejection** at the backend.

**Applied & verified** (backend `cargo check` + collection tests green, frontend `tsc` green):
- ✅ **H2 corridor enforcement** — `list_collections` now routes through `collection_repo::list_for_corridor` (corridor-scoped, excludes unsaved); `preview_apply` and the apply pipeline (`validate_corridor`) reject a cross-corridor collection before any FS mutation; the apply path now drives the pipeline with the **caller's** corridor (`request.is_safe`) instead of the collection's own `is_safe`, which was silently discarded. Two tests that asserted the old flag-ignore behavior were rewritten to assert rejection (`apply_collection_rejects_cross_corridor_request`, `preview_apply_rejects_cross_corridor_request`).
- ✅ **M1** — `resolve_recovery_task` now acquires the `OperationLock` before resuming an interrupted apply.
- ✅ **Dead code removed** — orphaned `pipeline/steps/snapshot_state.rs`, dead `apply_collection_internal`, three dead label constants in `corridor_constants.rs`; wired `DISABLED_REASON_COLLECTION` const into `batch_rename` (was a hardcoded `"COLLECTION"`); frontend dead PIN surface (`usePin.ts` + `pinKeys` + barrel export).

**Still open (deferred — larger/behavior-sensitive, not done this pass):**
- ⏳ **H1 scaffolding removal** — the model is decided (signature), but the physical removal of `is_unsaved`/`last_active` columns + `find_unsaved_for_corridor`/`resolve_restore_collection` plumbing is entangled across the domain struct, every SELECT, and a destructive migration. Deferred to a dedicated pass with full test coverage.
- ⏳ **M2** — unify manual toggle onto `runtime_mutation_engine` (two rename engines remain).
- ⏳ **M4** — propagate errors in `recompute_signature_tx` instead of `unwrap_or_default`.
- ⏳ **Preview-tree builder collapse** (~300 dead lines in `collection_preview_tree.rs`).
- ⏳ **Frontend tidy** — `useActiveCollectionSelection` hook (M3 + false comment), `resolveCorridorDisplayName` dedup, split `ApplyCollectionModal.tsx`, single tree-count field (L1/L2).
- ⏳ **Spec update** — kill `switch_mode` / `is_safe_context` / `RECOVERY_REQUIRED` drift.

---

## 0. Read this first — the spec is stale in three places

Auditing against the spec verbatim produces false positives. The code deliberately diverged:

| Spec says | Code reality | Action |
| --- | --- | --- |
| `switch_mode` / corridor mode-switching is a third mutation caller | **Removed** (`commands/collections/cmds.rs:297`). No `switch_mode` exists. | Update spec; drop it as a caller everywhere. |
| Column `is_safe_context` | Column is **`is_safe`** (`init.sql:147`). `is_safe_context` exists nowhere. | Spec rename. |
| Backend emits `RECOVERY_REQUIRED` on boot | No such literal. Recovery is **pull-based** via `app_startup_check` (`cmds.rs:229`). | Decide: emit event, or update spec to pull model (see H2). |
| Dirty state = persisted `is_unsaved` collection snapshot | Runs on a **live signature-diff** model with a synthetic `"__current_runtime__"`; the persisted path is mostly inert. | Pick one model (see H1). |

**Confirmed present** (not gaps): `tasks` table (`init.sql:229`); `collection_mods.preview_path/node_type/warnings_json` (`20260325150000_collection_mod_preview_metadata.sql`); `is_unsaved`/`last_active`/`disabled_reason` (`init.sql:135,137,111`); `undo_collection_id` on `corridor_state` (`init.sql:153`).

---

## 1. Gaps ranked by severity

### 🔴 High

**H1 — Two overlapping dirty-state models; the persisted one is dead scaffolding.**
There is a persisted `is_unsaved`-collection machinery (migration column, unique index, `find_unsaved_for_corridor`, repo plumbing) **and** a live signature-diff model (`get_corridor_state.is_dirty`, synthetic `"__current_runtime__"` at `collection_service.rs:302-303`). The live model is what actually runs; `handle_dirty_state` (`collection_service.rs:292`) writes little/nothing persistent and `list_collections` filters unsaved rows out. This half-and-half is the single biggest source of spec-vs-code drift and reader confusion.
→ **Decide one model.** The signature model is simpler and already load-bearing — recommend committing to it and deleting the persisted `is_unsaved`/`last_active` scaffolding (H1 feeds directly into the dead-code list §2). If AC-31.3.2's *persisted snapshot* semantics are actually required, restore them instead — but you can't keep both.

**H2 — Corridor enforcement is UI-only, not backend-enforced (security).**
The spec's core safety claim — "mathematically impossible to apply an unsafe collection in Safe Mode" — is not true at the backend.
- `list_collections` takes `_is_safe` and **ignores it**, calling the unfiltered `list_for_game` (`collection_service.rs:140`, `collection_repo.rs:14`). Violates AC-31.1.3 + Security §.
- `preview_apply` also ignores `_is_safe` (`collection_service.rs:856`).
- `apply_collection` fetches by id with **no corridor guard** (`get_by_id` = `WHERE id = ?`, `collection_repo.rs:68`); `validate_corridor` checks only `game_id`, never that the row's `is_safe` matches the app corridor. A direct invoke with an opposite-corridor `collection_id` is accepted.
- Irony: the correctly-filtered `list_for_corridor` (`collection_repo.rs:33`, `AND c.is_safe = ?`) **exists but has 0 callers** — it's dead.
→ **Route `list_collections`, `preview_apply`, and the apply-path fetch through the corridor filter.** Net code *reduction* (makes the dead fn live). Single choke point.

### 🟡 Medium

**M1 — Recovery re-apply runs without the OperationLock.** The lock is acquired only in the `apply_collection` *command* (`cmds.rs:97`), not the service. The crash-recovery retry (`cmds.rs:263` → `collection_service::apply_collection`) calls the service directly, so a resumed apply is not mutually excluded from concurrent runtime ops. → Move lock acquisition into the service, or re-acquire in the recovery path.

**M2 — Two divergent rename engines.** `runtime_mutation_engine` (`services/runtime_mutation_engine.rs`) handles rename + DB projection + rollback and is used by collection apply (`pipeline/steps/batch_rename.rs:29`, correctly setting `disabled_reason="COLLECTION"`). But manual toggle re-implements the whole thing (`services/mods/core_ops.rs:137,207` via `rename_cross_drive_fallback` + `update_mod_path_status_and_reason`), never touching the engine. Collision/traversal/rollback fixes must be duplicated → latent drift bug. AC-31.2.4 says they share one engine; only apply does. → Route manual toggle through `runtime_mutation_engine`.

**M3 — Frontend re-derives active collection client-side despite a comment claiming it was removed.** `CollectionsPage.tsx:74-124` runs a 6-branch `effectiveSource` `useMemo` + sync effect to resolve the active/selected collection, while `active_collection_id` is already shipped by the backend and the Topbar binds it directly (`ContextControls.tsx:19`). The file's own header comment (`CollectionsPage.tsx:3-9`) claims this derivation was removed — it wasn't. → Move to a `useActiveCollectionSelection` hook or bind the backend value; delete the false comment.

**M4 — Silent error swallowing on DB reads.** `recompute_signature_tx` uses `.unwrap_or_default()`/`.unwrap_or(None)` on every `try_get` (`collection_service.rs:782,796-804,822-824`). A schema/decode error is masked as an empty default instead of bubbling — violates "no silent failures." → Propagate the error.

### 🟢 Low

- **L1 — Count-source mismatch (cosmetic).** Runtime row + panel headers use `active_root_count` (`types.ts:62`, `CollectionPreviewPanel.tsx:89,142`) while stored rows use tree `mod_count` (`CollectionTreeView.tsx:232`). Both are tree-derived (AC-31.7.8 not violated) but they're *different numbers* a user can see side-by-side. → Surface one backend tree-count on the snapshot/preview, bind everywhere.
- **L2 — Read-path error handling inconsistent.** `CollectionPreviewPanel.tsx:110-117` doesn't surface `previewQuery.isError` (falls through to generic "not found"); `ApplyCollectionModal.tsx:82-103` does. → Align.
- **L3 — `MissingModsError` handled in two spots** (`useCollections.ts:313` silent + `ApplyCollectionModal.tsx:58-63` drives dialog). Works, but the contract lives in two places.

---

## 2. Dead / orphaned code (safe deletions — ~450+ backend lines + frontend)

| Item | Evidence | Note |
| --- | --- | --- |
| `pipeline/steps/snapshot_state.rs` (~110 lines) | not declared in `pipeline/steps/mod.rs`; 0 grep hits | **Orphaned — not in the module tree.** Builds "Undo Snapshot" collections nothing invokes. Matches Non-Goal (undo removed). Delete. |
| `collection_preview_tree::build_preview_tree` + tree-assembly half (~300 lines) | `collection_preview_tree.rs:33-148,150,167,188,298,355,544,638,659` — 0 callers | Live tree is `projected_state_service::build_preview_tree_from_projected_state`. Only `resolve_preview_terminal_metadata`/`build_preview_descriptor`/`descriptor_from_stored_metadata` + path helpers are used. Duplicate builder. |
| `collection_repo::list_for_corridor` | `collection_repo.rs:33-66` — 0 callers | The **only** corridor-filtered list fn. Make it live (H2) rather than delete. |
| `apply_collection_internal` | `collection_service.rs:1097-1120` — 0 non-test callers | Near-identical to `apply_collection` minus `.without_task()`. Delete or parameterize. |
| `DISABLED_REASON_COLLECTION` const | `corridor_constants.rs:1` — 0 usages | `batch_rename.rs:25` hardcodes `"COLLECTION"`. Wire the const or drop it. |
| `CORRIDOR_UNSAVED_SAFE_PRESET_LABEL`, `..._UNSAFE_...`, `..._ALL_DISABLED_...` | `corridor_constants.rs:9-11` — 0 usages | These ARE the AC-31.3.6 canonical labels; backend never emits them (frontend owns labels). Dead. |
| `last_active` column | `init.sql:137`, read in `collection_repo` SELECTs — never written | Write-dead. Active state pivoted to signature matching (`update_corridor.rs`). Tied to H1. |
| `batch_db_update::update` | `pipeline/steps/batch_db_update.rs:8-15` | Vestigial no-op (just logs); engine already wrote projection. |
| `usePin.ts` (90 lines, 5 hooks) + `pinKeys` | frontend barrel `hooks/index.ts:19` | Fully dead PIN surface. Delete unless a `PrivacyTab` consumer is imminent. |
| `getCorridorStateName`, `buildCorridorEmptyStateLabel`, `ALL_DISABLED_LABEL` | `corridorLabels.ts:7,48,57` — prod-unused (tests only) | `getCorridorStateName` is the intended shared adapter but surfaces bypass it (see M-frontend below). |

**Correction to my initial hypothesis:** `collection_service/tests.rs` is **NOT dead** — `collection_service.rs:1122` declares `#[cfg(test)] mod tests;`, which resolves to the sibling-dir file. It compiles under `cfg(test)` (1725 lines).

---

## 3. Not-best-practice (beyond the gaps above)

- **Files over the 350-line ceiling:** `collection_service/tests.rs` 1725, `collection_service.rs` 1123, `collection_preview_tree.rs` 694 (≈300 dead), `runtime_mutation_engine.rs` 493, `collection_repo.rs` 497, `cmds.rs` 464, `domain/collection.rs` 352; frontend `ApplyCollectionModal.tsx` 383 (only FE file over — inlines missing-mods view + before/after diff + result view + progress footer; extract the three view states).
- **SRP violation:** 5 PIN commands living in the collections module (`cmds.rs:423-464`) — unrelated to collections.
- **Magic strings** where enums/consts exist: `"COLLECTION"` (`batch_rename.rs:25`), `"__current_runtime__"`/`"Current Runtime"` (`collection_service.rs:302-303`), stringified `NodeType` comparisons `Some("VariantContainer") | Some("ModPackRoot")` (`collection_preview_tree.rs:179,674` — `count_node_mods` compares stringified enum values instead of the `NodeType` enum).
- **Duplicated corridor display-name adapter (frontend):** `ContextControls.tsx:24-30` and `CollectionPreviewPanel.tsx:69-73` both hand-roll the identical `{name: is_dirty ? null : active_collection_name, isUnsaved: is_dirty || active_collection_is_unsaved, isSafe}` triad. The intended shared adapter `getCorridorStateName` exists but is bypassed. → One `resolveCorridorDisplayName(snapshot, labels)` helper.
- **Imperative DOM hacks:** `document.activeElement.blur()` repeated 3× as a daisyUI dropdown-close workaround (`ContextControls.tsx:39-41,70-72,131-133`).
- **Overlapping in-flight tracking:** `apply_progress_service` (global in-memory singleton keyed by `(game_id,is_safe)`) runs parallel to the `tasks` table — two mechanisms tracking "operation in flight."

**Good, leave alone:** No `any` in audited FE files (errors typed `unknown`, funneled through `formatAppError`/`extractMissingModsPayload`). Preview semantics are **cleanly split** — `collectionPreviewSemantics.ts` does only presentation mapping (node_type→i18n, status→label, warnings→tooltip); all collapse/count logic lives backend in `collection_preview_tree.rs`. No FE/BE duplication there. Frontend renders backend `tree_nodes` directly, never infers hierarchy from flat rows (satisfies "Preview Panel Payload" Integration Point).

---

## 4. AC coverage snapshot

| Area | Status |
| --- | --- |
| US-31.1 Save / Save-As, empty-collection reject | ✅ wired (SaveCollectionModal, backend `Path::exists` validation) |
| US-31.2 Pre-apply validation + Exclusive Swap + Missing-Mods dialog + Skip&Apply | ✅ wired (AC-31.2.2/2.3 FE `ApplyCollectionModal.tsx:58-63,192-209`; AC-31.2.4 `batch_rename.rs`) |
| US-31.2 OperationLock + Watcher suppress | ✅ apply path (`cmds.rs:97`, `batch_rename.rs:9`); ⚠️ **not** on recovery re-apply (M1) |
| US-31.3 Dirty-state + Topbar sync + corridor-aware labels | 🟡 labels/topbar ✅ (`getCollectionDisplayName` shared, corridor-aware); **model is dual/inert** (H1); FE re-derives active (M3) |
| US-31.4 Cross-collection auto-healing | ✅ wired all 3 paths (rename `core_ops.rs:597`, move `organizer_move.rs:283`, external reconcile `projection_writer.rs:345`) |
| US-31.5 Task recovery | 🟡 PENDING write + boot scan ✅; **`RECOVERY_REQUIRED` never emitted** (pull-based, H0/H2); lock gap (M1) |
| US-31.6 Active deletion → snapshot to unsaved | ⚠️ depends on H1 model decision |
| US-31.7 Preview tree semantics + tree-derived counts | ✅ backend `count_node_mods` (Inactive→0, Variant/ModPack→1, leaf→1); FE binds tree, not raw rows |
| Security: corridor enforcement | 🔴 **UI-only, not backend-enforced** (H2) |

---

## 5. Recommended sequence (max feature, min complexity)

1. **Decide the dirty-state model (H1).** Everything downstream depends on it. Recommend committing to the live signature model.
2. **Close the corridor security gap (H2)** — route the 3 read/apply paths through `list_for_corridor`'s filter. Net deletion.
3. **Delete dead code (§2)** — ~450+ backend lines + FE PIN surface. Zero behavior change once H1 is decided (removes `is_unsaved`/`last_active` scaffolding, the duplicate preview builder, `snapshot_state.rs`, `apply_collection_internal`).
4. **Collapse the two preview-tree builders** into the live `projected_state_service` path; shrink `collection_preview_tree.rs` to the terminal-metadata resolver it actually contributes.
5. **Unify rename (M2)** — manual toggle → `runtime_mutation_engine`. One engine, per AC-31.2.4.
6. **Lock the recovery path (M1)** + decide RECOVERY_REQUIRED emit vs spec update.
7. **Frontend tidy** — `useActiveCollectionSelection` hook (M3, deletes false comment), one `resolveCorridorDisplayName` helper, split `ApplyCollectionModal.tsx`, one tree-count field (L1).
8. **Update the spec** to kill the `switch_mode` / `is_safe_context` / `RECOVERY_REQUIRED` drift so future audits stop generating false positives.

The pipeline itself can also shrink: `batch_db_update` (Step 7) is a no-op and `update_corridor` (Step 8) only records metadata, so the 8-step apply pipeline folds to ~5 real steps (`resolve_current_state` + `compute_diff` are trivially mergeable).
