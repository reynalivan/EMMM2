# Dead Code & Unwired Audit (repo-wide)

**Date:** 2026-07-10. **Method:** Tauri command-registration diff, `cargo clippy`, `ts-prune`, and three reference-counting agents (grep-verified, false-positives excluded: `#[tauri::command]` macro registration, trait/dynamic dispatch, serde/DB string-keyed fields, barrel re-exports, test usage). Every item below was verified to have **zero production references**.

**Headline:**
- Command surface is **clean** — 138 defined = 138 registered, no unwired commands.
- `cargo clippy` finds nothing — all dead code is `pub`, invisible to the compiler.
- **~55 confirmed-dead backend `pub` items** (incl. 3 whole dead modules), **~10 test-only-dead**, **~24 dead frontend exports**, and **4 built-but-unreachable features** ("tidak wired").

---

## 1. UNWIRED features (built but not reachable) — the "tidak wired" list

These are the notable ones: code exists and compiles, but nothing invokes it. Decide **wire or delete** per item.

| Feature | Location | State |
| --- | --- | --- |
| **Variant cycling** | `services/mods/variant_service.rs` (whole module: `discover_variants`, `VariantGroup`, `VariantEntry`) | Never wired. `discover_variants` is a plain fn (not a command), so the frontend can't even call it. Abandoned or WIP. |
| **Browser download sessions** | `services/browser/session_service.rs` (whole module: `create_session`, `mark_session_downloading/complete`, `find_recent_active_session`, `DownloadSessionDto`) | Whole session-tracking module unreferenced. |
| **Workspace duplicate-conflict dialog** | opener `features/workspace-runtime/state/workspaceDialogs.ts:18` (`openWorkspaceDuplicateConflictDialog`) | ⚠️ Dialog UI is **fully rendered** (`FolderGridModals.tsx:92-166`, state `workspaceState.ts:39`) but **no call site** ever opens it — a dead branch parallel to the wired mod-runtime `duplicateWarning` flow. Wire the opener or delete the whole branch. |
| **Optimistic + rollback runtime effects** | `features/workspace-runtime/optimistic/applyOptimisticEffects.ts` (`applyOptimisticEffects`, `rollbackOptimisticEffects`) | Superseded by `applyRuntimeEffects` (used ~10 places). The snapshot/rollback pair is never called. |

Also ties to a prior finding: `runtime_mutation_engine::toggle_mods` (single-op) is unused — only `toggle_mods_mixed` is wired (collection apply). This is the same "two toggle paths" seam from the collections audit (M2).

---

## 2. Backend — CONFIRMED DEAD `pub` items

**Whole dead modules** (delete file + `pub mod` line):
- `services/browser/session_service.rs` — 5 items
- `services/operation_outcome.rs` — `OperationOutcomeKind`, `OperationIssueSeverity`, `OperationIssue`
- `services/mods/variant_service.rs` — 3 items

**Repo layer** (`repo/`):
```
corridor_repo.rs:38   get_runtime (fn)                         — sole caller is dead get_snapshot
corridor_repo.rs:117  clear_collection_references (fn)          — 0 callers; live _tx variant kept
corridor_repo.rs:191  get_snapshot (fn)                         — 0 refs
corridor_repo.rs:302  record_switch (fn)                        — 0 refs
game_repo.rs:84       get_all_game_mod_paths (fn)               — 0 refs
mod_repo.rs:14        OrphanMod (struct)                        — only used by dead get_orphan_mods
mod_repo.rs:21        ModPathInfo (struct)                      — only used by dead get_disabled_mods_by_object_id
mod_repo.rs:143       update_mod_path_only (fn)                 — 0 refs
mod_repo.rs:160       update_mod_path_by_old_path (fn)          — 0 refs
mod_repo.rs:460       batch_mark_system_disabled (fn)           — 0 refs
mod_repo.rs:480       mark_enabled_clear_reason (fn)            — 0 refs
mod_repo.rs:490       get_orphan_mods (fn)                      — 0 refs
mod_repo.rs:528       get_disabled_mods_by_object_id (fn)       — 0 refs
mod_repo.rs:666       update_mod_identity (fn)                  — 0 refs
mod_repo.rs:736       delete_mod_by_path_and_game (fn)          — 0 refs
mod_repo.rs:765       get_mods_with_uuid_format (fn)            — 0 refs
mod_repo.rs:775       update_mod_id (fn)                        — 0 refs
mod_repo.rs:819       update_mod_status_and_reason_tx (fn)      — 0 refs
mod_repo.rs:870       get_enabled_auto_tagged_mods_outside_corridor (fn) — 0 refs
mod_repo.rs:987       get_system_disabled_mods (fn)             — 0 refs
mod_repo.rs:1004      get_all_mods_mapping (fn)                 — 0 refs
object_repo.rs:842    get_objects_folder_paths (fn)             — 0 refs
pin_repo.rs:89        record_failed_attempt (fn)               — 0 refs
task_repo.rs:53       get_pending_tasks (fn)                    — 0 refs (live variant: get_all_pending_tasks_global)
```

**Services / domain / config** (scattered):
```
domain/corridor.rs:9       CorridorId (struct)         — transitively dead with mode_label
domain/corridor.rs:32      mode_label (fn)             — 0 refs
config/mod.rs:342          set_pin_with_recovery (fn)  — 0 callers (live: reset_pin_with_recovery_code)
config/mod.rs:467          set_safe_mode_enabled (fn)  — 0 refs
explorer/helpers.rs:75     apply_safe_mode_filter (fn) — called only inside dead _to_response
explorer/helpers.rs:114    apply_safe_mode_filter_to_response (fn) — 0 callers
fs_utils/guard.rs:51       validate_filename (fn)      — 0 refs
fs_utils/path_utils.rs:39  resolve_safe_path (fn)      — 0 refs
images/thumbnail_cache.rs:241  prune_orphans (fn)      — 0 refs
keyviewer/resource_pack.rs:68  KvResourcePack (struct) — nothing constructs/deserializes it
mods/archive/classify.rs:177   is_mod_file (fn)        — 0 refs
mods/core_ops.rs:191       toggle_mod_inner_service (fn) — redundant wrapper (callers use _with_duplicate_policy)
mods/info_json.rs:242      batch_update_info_jsons (fn) — 0 refs
pin_service.rs:78          verify_recovery (fn)        — 0 refs
runtime_mutation_engine.rs:18  RuntimeToggleRequest (struct) — param of dead toggle_mods only
runtime_mutation_engine.rs:61  toggle_mods (fn)        — 0 callers (live: toggle_mods_mixed)
runtime_projection_service.rs:164  refresh_paths_projection (fn) — 0 refs
apply_progress_service.rs:127  clear (fn)              — 0 qualified calls
scanner/core/types.rs:29   CollisionResolution (enum)  — 0 refs
scanner/core/types.rs:87   ScanResultItem (struct)     — only used by dead builder
scanner/core/types.rs:140  staged_auto_matched_object_name (fn) — only inside dead builder
scanner/core/types.rs:148  build_result_item_from_staged (fn) — 0 callers
scanner/deep_matcher/analysis/skin_resolver.rs:10  detect_skin_for_staged (fn) — 0 callers
commands/duplicates/mod.rs:8  DupScanMatchedEvent (struct) — legacy event, never emitted (no collect_events!)
pipeline/apply_pipeline.rs:89  without_task (method)   — no caller opts out of task tracking
```

**Dangling `pub use` re-exports** to remove alongside the above:
`commands/folder_grid/helpers.rs:5`, `commands/folder_grid/helpers.rs:6`, `services/scanner/deep_matcher/mod.rs:22`.

**Removal-order clusters** (delete bottom-up in ONE pass — `pub` only hides the *top* of each chain; removing the top surfaces compiler warnings on the rest):
- `get_orphan_mods` + `OrphanMod`
- `get_disabled_mods_by_object_id` + `ModPathInfo`
- `get_snapshot` + `get_runtime`
- `mode_label` + `CorridorId`
- `discover_variants` + `VariantGroup` + `VariantEntry`
- `toggle_mods` + `RuntimeToggleRequest`
- `build_result_item_from_staged` + `ScanResultItem` + `staged_auto_matched_object_name` + `CollisionResolution`
- `apply_safe_mode_filter_to_response` + `apply_safe_mode_filter` + re-exports `folder_grid/helpers.rs:5-6`
- `detect_skin_for_staged` + re-export `deep_matcher/mod.rs:22`

---

## 3. Backend — TEST-ONLY (dead in production, alive only via own unit tests)

Deleting these means deleting their tests too. Decide per item — they may be worth keeping as coverage, or the whole feature is dead.
```
collection_preview_tree.rs:33  build_preview_tree + tree-assembly cluster (~300 lines: count_preview_mods,
                               append_mod_branch, ensure_mod_node, ensure_folder_node, ancestor_paths_for_terminal,
                               sort_children, prune_empty_branches, count_node_mods)
                               — prod uses projected_state_service::build_preview_tree_from_projected_state;
                                 this whole builder is exercised only by collection_preview_tree_tests.rs.
                                 (KEEP resolve_preview_terminal_metadata — the file's only live export.)
validation/consistency.rs:6/29/43  ConsistencyResult, is_consistent, verify_fs_db_consistency — asserted only in #[cfg(test)]
mods/bulk.rs:212/380           bulk_toggle_inner / bulk_delete_inner — only tests call them (prod uses bulk_toggle/bulk_delete)
scanner/deep_matcher/.../content.rs:110  decode_ini_content — test-only
scanner/deep_matcher/.../gamebanana.rs:80  from_key — test-only
scanner/deep_matcher/golden_corpus.rs:41  run_golden_case — test-only
```

---

## 4. Frontend — DEAD exports (verified, not ts-prune noise)

```
lib/corridorLabels.ts:5/6/57   UNSAVED_SAFE_PRESET_LABEL, UNSAVED_UNSAFE_PRESET_LABEL, buildCorridorEmptyStateLabel
types/collection.ts:7/8        CollectionKind, MemberKind
types/mod.ts:21/38             NodeType, isNavigable
types/object.ts:217            GetObjectsResult
types/operationOutcome.ts:1/9  OperationOutcomeKind, OperationIssue        (mirrors dead backend operation_outcome)
types/scanner.ts:34/309        CollisionResolution, MetadataDraftValues     (dup — live copy in useMetadataDraft.ts)
types/workspace.ts:104         isWorkspaceObjectNode                        (dup — local copy in useWorkspaceSwitchActions.ts)
features/object-list/scanReviewHelpers.tsx:44  matchLevelLabel
features/preview/hooks/usePreviewData.ts:28/168  IniDocument (dup — live in lib/bindings.ts), usePastePreviewImage
features/settings/theme/themeOptions.ts:2/14   BuiltinTheme, isThemeSetting
features/mod-runtime/actions/sharedModDialogs.ts:173  openModDuplicateWarningDialog (superseded by state-managed path)
features/workspace-runtime/actions/sharedRuntimeResultMapper.ts:26  applyRuntimeQueryInvalidationResult (unwired 3rd variant)
features/workspace-runtime/actions/workspaceActionAvailability.ts:24  maskWorkspaceCapabilities (unwired variant)
features/workspace-runtime/actions/workspaceSwitchPolicy.ts:16  getWorkspaceSwitchNextEnabledState (unwired)
features/workspace-runtime/optimistic/applyOptimisticEffects.ts:26/96  applyOptimisticEffects, rollbackOptimisticEffects (superseded)
features/workspace-runtime/state/workspaceDialogs.ts:18  openWorkspaceDuplicateConflictDialog (unwired — §1)
testing/mockData.ts:91/98/105  DUMMY_OBJECTS, getGradient, generateDummyItems (unused test fixtures)
testing/mocks/tauri-plugin-fs.ts (whole file)  readFile/readDir/exists/writeFile — module never imported
```

Three are **duplicate definitions** shadowing a live copy — delete the flagged one, keep the live: `MetadataDraftValues`, `isWorkspaceObjectNode`, `IniDocument`.

**Not deletable (ts-prune noise):** the `features/collections/hooks/index.ts` barrel entries are `export *` re-exports; 4 symbols (`useCollections`, `useCorridor`, `useDeleteCollection`, `useUpdateCollection`) are consumed through it, so the barrel stays.

---

## 5. Suggested deletion order (if we proceed)

1. **Frontend dead** (§4) — isolated, `tsc` verifies instantly. Lowest risk.
2. **Backend whole modules** (§2 variant_service, session_service, operation_outcome) — delete file + `pub mod` line.
3. **Backend clusters** (§2 removal-order) — bottom-up, one `cargo check` after.
4. **Backend scattered singles** (§2) — batch, `cargo check`.
5. **Test-only** (§3) — separate decision (delete code + its tests, or keep as coverage).
6. **Unwired features** (§1) — per-item wire-or-delete decision, especially `openWorkspaceDuplicateConflictDialog` (delete opener **and** the orphaned rendered dialog branch, or wire it).
