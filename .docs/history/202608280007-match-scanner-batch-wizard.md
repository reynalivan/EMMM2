# Match Scanner Batch Wizard

## Outcome

The previous scan/import-specific matching paths now converge on one read-only Match Engine and one shared frontend wizard. Matching no longer writes object metadata, moves folders, or updates Collection references. All filesystem placement is committed through the workspace mutation coordinator and finalized by Disk Reconcile before classification metadata is written.

## Architecture

- `services/match_engine` owns source inspection, category inference, canonical matching, and destination suggestions. Existing `scanner/deep_matcher` remains a scoring primitive.
- `services/import_batch` and `repo/import_batch_repo.rs` own persisted batch/item state, archive staging, user decisions, refresh, resume, and ReadyToMove discovery.
- `services/workspace_mutation/import_commit.rs` owns stale-preview validation, path checks, operation locking, scoped watcher suppression, move journaling/rollback, terminal reconcile, and post-reconcile classification.
- `services/objects/classification.rs` is the atomic writer for stable category/subcategory, metadata, canonical relation, child-mod type, learned user aliases, and runtime projection.
- Disk Reconcile remains the only owner of filesystem identity, object/mod membership, and Collection path/ID rebinding.
- File Watcher observes only configured workspace mod paths and forwards external changes to Disk Reconcile. ReadyToMove and app staging are not watched.

## Contracts and migrations

- Stable category keys are `Character`, `Weapon`, `UI`, and `Other`; game-specific names are labels/subcategories.
- Legacy categories are normalized by `20260828090000_normalize_legacy_object_categories.sql`.
- MasterDB entries now distinguish `canonical` identities from `taxonomy` inference helpers. Taxonomy entries cannot create physical folders or become canonical relations.
- `20260828091000_import_batches.sql` adds persistent batches and enriches import jobs with source/staging, classification, canonical/destination suggestions, confidence/evidence, decisions, fingerprints, and results.
- ReadyToMove paths are configurable per game and default to OS Downloads `/mods/<sanitized-game-name>`.

## Matching and commit flow

1. Folder/archive/browser/ReadyToMove sources create a batch. Archives are extracted into isolated app staging; each discovered mod root becomes one item.
2. Source inspection gathers normalized root/nested/file/INI signals to depth three and stores a source fingerprint.
3. The user confirms category and optional game-schema metadata before canonical matching is available.
4. Canonical candidates are category-filtered and exclude taxonomy entries. Metadata reranks candidates without overriding user choices.
5. Destination resolution prefers a specific target, then existing canonical/folder/alias matches, then canonical folder creation. No canonical DB identity defaults to Skip or explicit existing-target selection. `Other` remains a category and still uses folder-name matching.
6. The backend persists decisions; commit accepts item IDs rather than trusting frontend scoring payloads.
7. Commit revalidates state/fingerprint/paths, moves through a journal under scoped watcher suppression, reconciles once with path hints, then applies idempotent classification metadata.

All four arrival flows (auto, specific-target, ReadyToMove, and Browser) normalize the planned and committed physical folder name to one idempotent `DISABLED ` prefix. Existing-structure classification and relocation preserve current filesystem status by default; each exposes an explicit checkbox to set affected folders to Disabled. Optional classification disabling uses the same journal, mutation lease, watcher suppression, path-hinted reconcile, and warning contract as other workspace mutations.

Interrupted commits remain resumable. Before moving, the exact planned target is persisted. A restart during metadata finalization becomes `MetadataPending`; a retry can rerun reconcile and/or classification idempotently. If a crash interrupts a move, recovery distinguishes source-only, target-only, collision, and missing-source/target cases under the same operation lock and watcher suppression contract. Terminal batch staging is removed on completion/cancel and swept on startup.

Interrupted analysis is reset to a resumable batch, failed extraction can be re-staged after cleaning only its owned staging directory, and a commit that rolled back successfully returns to `Ready`. ReadyToMove archives use an explicit `archive_pending` recovery state and cannot make the batch `Done` or delete staging until the archive reaches `Processed`. A persisted-batch resume action is available globally and from the Browser queue.

Cross-volume folder/file moves publish through a temporary path on the target volume, preserve the exact planned destination shape, and register an owned partial target with the rollback journal if cleanup cannot restore the pre-move state.

Suggestion refresh invalidates any prior destination decision. Existing-object classification also revalidates every frontend canonical selection against the current game MasterDB immediately before the atomic write: the key must exist, be a canonical entry, match the confirmed stable category, and use a registered alias.

Windows canonical and non-canonical representations of the same archive staging path are normalized before fingerprint comparison. Content freshness still requires identical modified time, size, and file count.

## Frontend

- `features/match-wizard` provides the shared Sources, Category & metadata, Object/destination, Validation, and Result flow.
- `features/import-batches` owns batch host/launchers for auto import, specific-target import, Browser, and ReadyToMove.
- ObjectList, FolderGrid, and Preview classification actions use the object classification wizard.
- Relocation keeps the stable move commit service but obtains read-only destination suggestions from the shared matcher.
- Relocation preserves current enabled/disabled state unless the user checks the optional Disabled checkbox.
- Existing Object Classification preserves status unless its optional Disabled checkbox is checked.
- Settings Rescan is now Repair Index (Disk Reconcile); ReadyToMove scanning is on demand.
- The previous Sync/Drop/Scan Review/Needs Review/archive import modals, direct scan-and-commit APIs, Browser confirmation path, obsolete hooks, and related translations were removed after callers migrated.
- Bulk Auto Recognize is now Classify & Match and always opens the confirmation wizard.

## Safety and regression coverage

- Category normalization and legacy browser job migration.
- Canonical versus taxonomy behavior, aliases, metadata reranking, transliteration, nested depth three, and required Ayaka/Raiden/Shogun/Hutao examples.
- `Other` folder-name matching without a physical `Other` fallback.
- Archive multi-root staging, ReadyToMove exclusions/lifecycle, stale previews, path validation, collisions, move rollback, and Windows staging path equivalence.
- Cross-game workspace-source rejection, interrupted commit/metadata recovery, terminal staging cleanup, and stale decision/rename clearing.
- Destination category isolation for imports, explicit specific-target mismatch warnings, and category-as-context matching for relocation.
- Classification batches preflight every fingerprint and write every object in one transaction.
- Watcher suppression/external event behavior and Disk Reconcile path hints.
- Collection path/ID rebinding and runtime projection/classification consistency.
- Shared wizard decisions and Browser high-confidence confirmation behavior.

## Validation

- `cargo test`: 883 passed, 2 ignored across 9 suites.
- `cargo test --test arch_audit`: 11 passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- Vitest: 141 files, 752 passed, 1 skipped.
- TypeScript `--noEmit`: passed.
- ESLint: passed with no errors (six existing max-lines warnings).
- Vite production build: passed.
- i18n lint: passed.
- WDIO phase 5 import safety: 3 passed, including folder bulk, collision, and archive staging/commit.
- WDIO phase 6 scan/sync: 4 passed.
