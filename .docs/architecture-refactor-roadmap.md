# Architecture Refactor Roadmap — EMMM

> **[2026-08-09] Dokumen historis.** Log fase refactor 2026-07-26, sudah selesai dan kemudian DILAMPAUI oleh refactor disk-truth 2026-08-09 (`docs/enable-disable-flow.md`): `runtime_mutation_engine` kini rename-only (commit_db dihapus), status/`folder_path` ditulis hanya oleh Disk Reconcile, `disabled_reason` di-drop. Angka baris dan klaim arsitektur di bawah adalah snapshot lama.

> Hasil audit arsitektur 3 arah (FE, BE Rust, safety-net) — 2026-07-23.
> Satu fase = satu sesi kerja = rangkaian commit hijau penuh.
> Status: **SELURUH FASE 0–9 SELESAI (2026-07-26).**
> Verifikasi akhir: cargo test **600**, vitest **768**, clippy **blocking 0 warning**,
> tsc bersih, eslint exit 0, build produksi sukses, drift guard bindings hijau.
>
> Hasil Fase 9 — BE: setup startup ~170 baris keluar dari `lib.rs` (428→303) ke
> `services/bootstrap.rs` dengan URUTAN OPERASI DIPERTAHANKAN; layer vestigial
> `database/` DIBUBARKAN → `domain/models.rs` (169 rujukan di-rewrite, tanpa shim
> alias); `keyviewer/generator` 536→6 submodul, `explorer/listing` 526→5,
> `config/mod` 421→5, `domain/workspace` 404→5. **Clippy kini BLOCKING** dengan
> `-D warnings` — 71 warning lama diperbaiki tanpa satu pun `#[allow]`.
> Efek samping yang benar: ratchet `dal_audit` menolak `DELETE FROM tasks` yang
> ikut pindah, jadi query itu turun ke `repo/task_repo.rs::purge_old_tasks`.
> FE: `welcome` dilebur ke `onboarding/welcome/`; `useFolderMutations` 542 → 2 util
> murni bertest + `useBulkModMutations` + sisa; `useObjects.test.tsx` direname ke
> `useObjectQueries.test.tsx`; 2 entri mati di `EXCLUDED_FILES` audit dihapus
> (mempersempit skip set = rule MENGUAT).
>
> **DEDUP PIN DITOLAK** (seperti `BulkTagModal` di Fase 3): `PinEntryModal`
> memverifikasi PIN (lockout timer, hitungan percobaan, alur recovery, portal),
> `PinModal` hanya mengumpulkan PIN (tanpa backend, `<dialog>`). Tumpang tindih
> hanya ~15 baris. Menggabungkan = 3 flag mode + 2 strategi mount.
>
> CATATAN GATE: menghapus `continue-on-error` saja TIDAK cukup untuk clippy —
> `cargo clippy` polos exit 0 walau ada warning. Harus `-D warnings`. Ini kelas
> cacat yang sama dengan `git diff` yang mengabaikan file untracked (Fase 7).
>
> SISA >350 baris (di luar scope fase mana pun, untuk pekerjaan lanjutan):
> `projected_state_service` 630, `runtime_mutation_engine` 627,
> `workspace_switch_service` 510, `disk_reconcile/rename_healer` 464,
> `hotkeys/manager` 440, plus beberapa file test besar.
>
> PERLU DIVERIFIKASI: doc comment `run_startup_reconcile` di `services/bootstrap.rs`
> menyebut "fails downloads and import jobs a crash left in flight", padahal
> fungsinya hanya purge task + disk reconcile — doc mendahului kode, atau ada
> perubahan paralel yang belum mendarat.
>
> Hasil Fase 8 — FE: `folder-grid` 40 file flat → `components/ | hooks/ | modals/ | utils/`
> (entry `FolderGrid.tsx` tetap di root); 7 importer lintas fitur + 13 spesifier
> `vi.mock` diperbarui; 3 path hardcoded di `runtimeArchitecture.audit` disesuaikan
> (rule TIDAK dilemahkan). Test: vitest **755**.
>
> Review kohesi preview: **1 dari 4 dipecah, 3 SENGAJA TIDAK** — hasil yang sah.
> - `AdvancedKeybindModal` 310 → 287: dua fungsi murni saling invers (parse/format
>   nilai keybind, a.l. aturan `no_ctrl` menegasikan `ctrl`) diekstrak ke
>   `utils/keybindValue.ts` + 10 test termasuk round-trip. Nol prop-drilling.
> - `usePreviewPanelState` 313: fasad state, logic murninya SUDAH di `previewPanelUtils.ts`.
> - `IniEditorSection` 308: ~260 baris JSX; memecahnya = 7 prop tembusan.
> - `GallerySection` 301: sudah terdekomposisi internal + util bertest.
>
> **BUG UI DIPERBAIKI**: `usePreviewPanelState` memakai `field.id.split('::')[0]`
> untuk nama file, padahal ID dibangun dengan titik dua TUNGGAL
> (`file:section:kind:line`) — jadi modal unsaved-INI menampilkan ID field utuh,
> bukan nama file. Kini memakai `field.fileName` yang memang tersedia.
>
> Dikeluarkan dari scope Fase 8 (sudah tidak berlaku): `preview_cmds.rs` sudah
> tertipiskan 356→167 di Fase 1; merger `BulkTagModal` sudah DITOLAK di Fase 3.
>
> PERLU DIVERIFIKASI: `folder-grid/modals/ConflictResolveDialog.test.tsx:13` —
> `vi.mock` menunjuk ke luar `src/` (mati sejak sebelum dipindah, kedalamannya
> digeser agar tetap mati); path yang benar `'../../../stores/useToastStore'`.
> Kandidat hapus: `folder-grid/components/ContextMenu.test.tsx` — menguji
> `components/ui/ContextMenu` dan subset lebih lemah dari test yang sudah ada di sana.
>
> Hasil Fase 7 — BE: seluruh SQL mentah keluar dari `services/browser/` ke
> `repo/browser_repo/` baru (settings/downloads/import_jobs) + `game_repo`;
> `dal_audit` SERVICES_BASELINE untuk browser TURUN KE NOL (ratchet, bukan
> pelonggaran). `import_service` 823 → 6 submodul, `browser_service` 532 → 5.
> Orkestrasi cancel-download pindah ke service. +14 test (browser_service,
> download_service, download_handler) — modul yang sebelumnya nol coverage.
> FE: fitur `downloads` DILEBUR ke `browser` (folder dihapus, 1 importer);
> `BrowserPage` 505 → 309 + util `browserUrl` bertest. Test: cargo **599**,
> vitest **745**.
>
> **CACAT FASE 0 YANG DITEMUKAN & DIPERBAIKI**: `src/lib/bindings.gen.ts` ternyata
> UNTRACKED, sedangkan gate CI memakai `git diff --exit-code` yang MENGABAIKAN file
> untracked — jadi drift guard type itu tidak pernah benar-benar aktif sejak Fase 0.
> File kini di-track, dan gate diperkuat dengan `git add -N` lebih dulu agar kasus
> yang sama tidak lolos lagi.
>
> Klarifikasi: `browser_cmds.rs` masih menyebut `SqlitePool` 13x, tapi semuanya
> sebagai `State<'_, SqlitePool>` — itu pola wajib Tauri dan seragam dengan seluruh
> command module lain. Menghapusnya butuh newtype se-repo; DITOLAK sebagai abstraksi
> tanpa peminta.
>
> PERLU DIVERIFIKASI (belum ditelusuri tuntas):
> - `services/browser/browser_service/settings.rs:38` — `normalize_url` mencocokkan
>   skema case-sensitive, sedangkan `validate_http_url:26` meng-lowercase dulu;
>   input `HTTP://example.com` jadi `https://HTTP://example.com` lalu gagal parse.
>   Perilaku ini DIKUNCI test apa adanya, bukan diperbaiki.
> - `src-tauri/app.db` tertinggal dari `migrations/` → statement yang menyentuh
>   `match_entry_key`/`match_alias_name` harus tetap `sqlx::query` runtime, bukan `query!`.
> - `migrations/20260406000000_fix_browser_downloads_schema.sql` masih untracked
>   (bukan buatan sesi ini) — ia DROP+recreate `browser_downloads`; perlu keputusan commit.
>
> Hasil Fase 6 — BE: `mechanical_rerank` 506 → 5 submodul; `sync/commit` 484 → 5
> (per fase nyata: temp-move → linking → execute → orkestrator); `gamebanana` 454 → 4;
> `full_pipeline` 442 → 4; `projection_writer` 434 → 9; `orchestrator` 372 → 6.
> `QueryBuilder` mentah pindah dari `deepmatch_scanner_cmds` ke
> `repo/mod_repo/listing.rs::get_folder_paths_by_object_ids`. Test: cargo **585**,
> vitest **740**.
> Sisa >350 di scanner (ditunda, bukan target Fase 6): `dedup/scanner.rs` 440,
> `dedup/signals.rs` 433, `content/tokenizer.rs` 354.
>
> PERLU DIVERIFIKASI (belum ditelusuri tuntas):
> - `analysis/mechanical_rerank/penalties.rs:20-32` — `is_multi_entity` dan
>   `has_type_mismatch` selalu `false` (placeholder), jadi `PENALTY_MULTI_ENTITY`
>   dan `PENALTY_TYPE_MISMATCH` efektif mati.
> - `disk_reconcile/projection_writer/prune.rs:57` — `prune_missing_mods` tidak
>   menerima `game_id` (mengandalkan `DbIndex.mods` sudah ter-scope game), berbeda
>   dari `prune_missing_objects:14` yang memfilter eksplisit.
>
> Fase 6 SELESAI: kontrak `dup_resolve_batch` DIPERBAIKI (rusak sejak Fase 0).
> Penyebabnya beda kosakata — UI memutuskan per GRUP, backend menyelesaikan per
> PASANGAN (`keepA` membuang folderB, `ignore` mem-whitelist pasangan). Sekarang
> diterjemahkan di batas oleh `src/features/scanner/utils/resolutionRequests.ts`
> (+5 test): "keep T" → satu pasangan `keepA` per anggota lain; "ignore" → seluruh
> kombinasi pasangan agar grup benar-benar ter-whitelist. Type legacy di
> `src/types/scanner.ts` diganti re-export dari `bindings.gen`; kosakata UI kini
> bernama `DuplicateSelection` supaya tidak bentrok dengan tipe wire.
>
> Fase 6 BELUM: pecah `mechanical_rerank` 506 / `sync/commit` 484 / `gamebanana` 454
> / `full_pipeline` 442; `QueryBuilder` mentah di `deepmatch_scanner_cmds` → repo;
> pecah `disk_reconcile/projection_writer` 434 + `orchestrator` 372.
> Dua agen mati kena limit sesi di tengah split; pohon kerja SUDAH dipulihkan hijau
> (cargo test 585) dengan mengembalikan `mechanical_rerank.rs` + `skin_resolver.rs`
> dari HEAD lalu memasang ulang rewrite import Fase 1 (`services::*` → `common::*`).
> SISA YANG PERLU DIBERSIHKAN (butuh izin hapus, tidak memengaruhi build):
> direktori parsial `analysis/mechanical_rerank/`, `sync/commit/`,
> `disk_reconcile/projection_writer/` berisi file yatim + `*_ABANDONED_mod.rs.bak`.
>
> PELAJARAN: `cargo check` sempat melapor 0 error padahal pohon rusak (artefak
> cache). Selalu `touch src/lib.rs` sebelum cargo check saat memverifikasi
> pekerjaan agen yang mati di tengah jalan.
>
> Hasil Fase 5 — BE: `resolve_recovery_task` (~193 baris business logic) keluar dari
> command layer ke `services/recovery_service.rs`; `commands/collections/cmds.rs`
> 458 → 286 dan akses repo langsung **8 → 0**; +6 test mengunci perilaku recovery
> yang sebelumnya nol coverage (a.l. retry gagal MEMBIARKAN task tetap terbuka,
> task legacy `switch_corridor` dipensiunkan bukan diputar ulang).
> `collection_service.rs` 917 → 9 submodul (terbesar 238); `collection_repo.rs`
> 756 → 6 submodul (terbesar 198); API publik keduanya tetap.
> Dihapus: `repo/tests/collection_repo_test.rs` — file mati, tidak dideklarasikan
> di mana pun dan memanggil API yang sudah tidak ada (takkan compile bila di-wire).
> FE: `ApplyCollectionModal` 383 → di bawah batas; action bar diekstrak ke
> `ApplyCollectionActions.tsx`. Test: cargo **585**, vitest **735**.
>
> PERLU DIVERIFIKASI (temuan agen, BELUM ditelusuri tuntas — jangan diperlakukan
> sebagai bug sebelum dicek):
> 1. Signature collection disimpan DENGAN `mods_path` (`create_collection`,
>    `replace_collection_with_current_state`) tapi dicocokkan TANPA `mods_path`
>    (`list_collections` → `compute_signature` → `build_projected_state(..., None)`).
>    Bila ada root hilang di disk, `is_active` bisa selalu `false`. Petunjuk bahwa
>    niat desainnya "signature bebas mods_path": `load_projected_collection_state`
>    sengaja tidak menulis cache signature saat `mods_path.is_some()`.
> 2. Asimetri cakupan corridor: `create_collection` mode SaveCurrentState memakai
>    live state TANPA filter `is_safe` (kedua corridor), sedangkan
>    `replace_collection_with_current_state` memakai filter `is_safe`.
>
> Hasil Fase 4 — BE: `mod_repo.rs` 938 → 10 submodul (terbesar 225);
> `workspace_cmds.rs` 521 → **41** (orkestrasi ke `services/workspace_switch_service.rs`,
> 347 baris prod) + kebocoran tipe Tauri `State` di `enable_only_this_service` ditutup;
> `core_ops` 634 / `bulk` 480 / `object_switch` 437 / `trash` 407 → direktori submodul,
> semua ≤350. Sisa file Rust prod >350 tinggal 21, semuanya milik fase berikutnya.
> FE: `useWorkspaceSwitchActions` 511→312, `useSharedObjectActions` 412→309,
> `useWorkspaceViewModel` 391→163 (logic murni ke `workspaceSwitchOps.ts`,
> `selectionReconciliation.ts`, `sharedObjectActionsState.ts`); `useObjHandlers*`
> DIGANTI nama sesuai domain (`useArchiveImportFlow`, `useDropImportFlow`,
> `useObjectBulkActions`, `useScanReviewFlow`), `useObjHandlersCrud` dihapus
> (pass-through dengan useMemo yang tak pernah memoize). Duplikasi pipeline import
> (3 salinan) → `utils/importPipeline.ts`. Test 687 → **732** (+45).
> Kedua allowlist audit MENGECIL (tidak ada rule yang dilemahkan).
>
> **BUG NYATA DIPERBAIKI**: bulk-pin memanggil `pinObject({id, isPinned})` padahal
> Rust mewajibkan `{id, pin}` → gagal serde senyap, error ditelan `catch`, toast
> sukses tetap muncul. Akar penyebabnya wrapper `bindings.ts` bertipe serba-opsional;
> tipe wrapper ikut diketatkan agar tidak terulang.
>
> DIVERIFIKASI BUKAN BUG (klaim agen, ditelusuri sendiri): (a) `folder_path_key(path, None)`
> di jalur delete/batch — path absolut short-circuit sebelum `mods_path` dipakai, jadi
> key identik; prasyarat "harus absolut" kini didokumentasikan di 3 fungsi itu.
> (b) `bulk_toggle` tidak mengisi `collection_impact` — toggle satuan juga tidak;
> healing memang hanya milik rename/delete, karena enable/disable justru bekerja lewat
> prefiks `DISABLED` yang dikelola collection.
> DITUNDA: template handler bersama (validate→mutate→reconcile→finalize, ~25 site)
> — digabung ke fase yang menyentuh command layer terkait.
>
> Hasil Fase 3: BE `object_repo.rs` 1621 → direktori `repo/object_repo/` berisi 8
> submodul (types/listing/counts/lookup/mutate/update/sync/matching, semua ≤296
> baris) dengan API publik di-re-export apa adanya; orkestrasi match pindah ke
> `services/objects/matching.rs` sehingga `object_cmds` (148 baris) tak lagi
> memanggil repo untuk data-access (sisa: impor tipe DTO saja).
> FE: 64 file object-list pindah ke `components/ | hooks/ | modals/ | utils/`
> (root tinggal `ObjectList.tsx`); 196 import + 27 spesifier `vi.mock` ditulis ulang
> terhitung; formatting murni ekstraksi arsip pindah ke `utils/archiveSummary.ts`
> (+6 test baru). **BUG NYATA DIPERBAIKI**: bulk-delete object memanggil
> `useDeleteCollection` (alias `useDeleteObject`) → command `delete_collection`
> dengan ID object; kini pakai `useDeleteObject` yang benar (`{id, force}`).
> Guard: allowlist path lama di `scannerImportArchitecture.audit` &
> `runtimeArchitecture.audit` ikut diperbarui.
> CATATAN LINT: `npx eslint . | tail` MENUTUPI exit code — klaim "lint bersih" di
> Fase 0–2 tidak tervalidasi. Ternyata `bindings.gen.ts` melanggar ban-ts-comment
> sejak Fase 0; file generated kini masuk ignore list. Selalu cek `echo $?` terpisah.
> DITOLAK (klaim audit tidak akurat): merger `BulkTagModal` object-list vs
> folder-grid — setelah dibaca keduanya beda fundamental (controlled add/remove
> dengan callback vs self-contained add-only yang memanggil mutation + toast +
> namespace i18n berbeda). Menggabungkannya menambah kompleksitas, bukan mengurangi.
> SISA KE FASE 4: konsolidasi `useObjHandlers*` (masih 5 file; archive 448 baris)
> — dilakukan bersama penipisan workspace-runtime karena saling terkait.
>
> Hasil Fase 2: `useAppStore.ts` 497 → 97 baris; state pecah ke 7 slice di
> `src/stores/appStore/` (game, navigation, layout, selection, objectList,
> explorer, workspaceRuntime) + reducer/transitions murni terpisah; API publik
> identik (0 dari 38 importer berubah). `workspaceReducer.ts` + `workspaceSelectors.ts`
> TERHAPUS (terserap ke appStore); `workspaceStoreBridge.ts` tinggal fasad tipis
> 128 baris (dipertahankan: 15+ importer lintas fitur + 13 test mock store parsial).
> Audit rule "legacy conflict dialog" diperluas memindai folder slice. Test
> karakterisasi lulus tanpa diubah.
>
> Hasil Fase 1: modul leaf `src-tauri/src/common/` (path_key, corridor_constants,
> classifier, normalizer) — siklus `repo → services` PUTUS; `GameObject` pindah ke
> `database/models.rs`; keluarga preview `_inner` pindah ke
> `services/mods/preview_ops.rs` — edge `services → commands` PUTUS (preview_builder
> & dedup scanner kini impor ke bawah; shim `commands/folder_grid/classifier` dihapus).
> FE: `RuntimeEffectDescriptor` dkk. pindah ke `src/lib/runtimeEffects.ts` — siklus
> `runtime-sync ↔ workspace-runtime` PUTUS + guard test anti-kambuh di
> `runtimeArchitecture.audit.test.ts`. Error: `types/errors.rs` (CommandError) DIHAPUS;
> 68 command String → `AppError` (+`From<sqlx::Error>` terpusat); map_err stringify
> 113 → 39 (residu di scanner sync); FE 16 call-site pakai `formatAppError`.
> SISA KE FASE 6: 15 command scanner/import/archive masih String karena protokol
> string dengan FE (`DUPLICATE|` di commit_scan, deteksi password di extract).
> Template handler bersama (validate→mutate→reconcile→finalize) DITUNDA ke Fase 4.
>
> Hasil Fase 0: specta codegen aktif (`src/lib/bindings.gen.ts` = sumber type tunggal,
> `src/types/*` jadi re-export tipis; drift guard: cargo test regenerate + CI
> `git diff --exit-code`); max-lines warn 350 + clippy advisory di CI; 20
> characterization test Rust (disk_reconcile, runtime_mutation_engine,
> projected_state_service) + 15 smoke test useAppStore; 6 bug drift runtime NYATA
> diperbaiki (SyncResult/WhitelistEntry/ExtractionResult/ArchiveEntryInfo camelCase,
> HotkeyConfig kurang field variant, specta salah meng-export enum serde_repr).
> KNOWN BROKEN (ditunda ke Fase 6): kontrak `dup_resolve_batch` FE masih group-based
> lama — resolusi duplikat gagal serde di runtime; type legacy ditandai di
> `src/types/scanner.ts`.

## Baseline temuan

- **Type drift tak terjaga**: `src/types/*` (~1200 baris) mirror manual dari `src-tauri/src/domain/*`; specta export dikomentari di `lib.rs`. Hanya nama command yang dijaga (`commandRegistry.audit.test.ts`, 4 registri).
- **Siklus dependensi**: BE `repo ↔ services` (repo impor `services::path_key`, `corridor_constants`, `explorer::classifier`, `scanner::core::normalizer`) + edge `services → commands` (`preview_builder` → `preview_cmds`); FE `runtime-sync ↔ workspace-runtime` (via `RuntimeEffectDescriptor`).
- **God-file**: BE `object_repo.rs` 1621, `mod_repo.rs` 938, `collection_service.rs` 917, `import_service.rs` 823, `core_ops.rs` 634; FE `useAppStore` 497 (~209 field, 38 importer), `useFolderMutations` 533, `useWorkspaceSwitchActions` 511, `BrowserPage` 505.
- **3 skema error BE**: `AppError` (benar) vs `CommandError` (duplikat mati) vs `Result<_, String>` (~83 command) + 113 `.map_err(|e| e.to_string())`.
- **Tanpa test**: BE `disk_reconcile` (~2000 LOC, mutasi fs), `runtime_mutation_engine` 457, `projected_state_service` 385, `commands/browser`, `commands/collections`; FE `conflict-report`, `downloads`, `welcome`, `useAppStore`.
- **Boilerplate handler**: validate_path (27×) → mutate → emit_reconcile (25×) → finalize_side_effects (14×) di ~25 handler.
- **Sehat (jangan diutak-atik)**: IPC 100% terpusat `bindings.ts`; audit tests (`commandRegistry`, `runtimeArchitecture`, `scannerImportArchitecture`); `pipeline/` & `domain/`; coverage padat object-list/folder-grid/scanner/settings/preview/workspace-runtime.
- **Koreksi**: `keyviewer` HIDUP (dipakai `post_apply`, `hotkeys` — internal tanpa command); `commandRegistryAuditUtils.ts` = infrastruktur drift-guard, bukan dead code.

## Tabel kesehatan per fitur (FE)

| Fitur | LOC | Grade | Masalah utama |
|---|---|---|---|
| object-list | 9509 | D | 48 file flat, 6 file >350, coupling terberat |
| workspace-runtime | 3197 | D | hub 10 fitur, 4 file >350, state nyangkut di god-store |
| folder-grid | 5026 | C | 27 file flat, coupling berat |
| preview | 4009 | C | 5 file "mengakali" limit (301–313), coupling 4 arah |
| scanner | 3147 | C | overlap ScanReview dengan object-list |
| mod-runtime | 1099 | C | useSharedModActions 370 |
| runtime-sync | 265 | C | siklus dengan workspace-runtime |
| file-watcher | 704 | C | — |
| settings | 2893 | B | PIN modal duplikat dengan safe-mode |
| collections | 2240 | B | ApplyCollectionModal 383 |
| browser | 1472 | B | BrowserPage 505 |
| downloads | 200 | B | shim 100% atas browser → fold (Fase 7) |
| welcome | 786 | B | gabung ke onboarding (Fase 9) |
| onboarding/randomizer/safe-mode/launch-bar/file-management | kecil | B | minor |
| dashboard / conflict-report | 903/96 | A | conflict-report tanpa test |

## Grade modul BE

| Modul | LOC prod | Grade | Masalah |
|---|---|---|---|
| repo/ | ~4900 | D | object_repo 1621, mod_repo 938, collection_repo 756; impor terbalik ke services; 7 repo tanpa test |
| commands/ | ~5700 | C | bypass repo/sqlx langsung di 6+ file; business logic di collections/cmds |
| services/ | ~24000 | C | 20+ file >350; disk_reconcile & runtime engine tanpa test |
| types/ | 149 | C+ | CommandError duplikat → hapus |
| pipeline/ | 626 | B | bersih, tapi tanpa test |
| database/ | 190 | B | vestigial → lebur ke repo (Fase 9) |
| domain/ | 1133 | B+ | workspace.rs 404 |

## Fase

### Fase 0 — Safety Net ✅ (sesi ini)
1. Roadmap ini ditulis ke `.docs/`.
2. Specta codegen aktif: export ke `src/lib/bindings.gen.ts`; `bindings.ts` jadi shim re-export; type manual tergantikan dihapus; guard "generated up-to-date" di cargo test.
3. `max-lines` warn 350 di eslint; `cargo clippy` non-blocking di CI.
4. Characterization test: `disk_reconcile` (orchestrator/projection_writer/rename_healer), `runtime_mutation_engine`, `projected_state_service`, smoke test `useAppStore`.

### Fase 1 — Fondasi lintas-lapisan
- BE: modul leaf `common/` ← pindahkan `path_key`, `corridor_constants`, `explorer/classifier`, `scanner/core/normalizer` (putus repo→services).
- BE: putus `preview_builder` → `preview_cmds` (turunkan logic ke services).
- BE: unifikasi error — hapus `types/errors.rs`; ~83 command String → `AppError` + `#[from]`; babat 113 `.map_err(|e| e.to_string())`.
- FE: putus siklus runtime-sync ↔ workspace-runtime (pindah `RuntimeEffectDescriptor` dkk. ke modul leaf, mis. `src/lib/runtimeEffects.ts`).
- BE: mulai ekstrak template handler (validate_path → mutate → emit_reconcile → finalize).

### Fase 2 — State & store (FE)
- Pecah `useAppStore` jadi slice per domain (nav, selection, workspace-preview, object-list filter, panel, theme/safe-mode/game); API publik via re-export.
- Serap `workspaceReducer` + `workspaceStoreBridge` ke slice workspace. Test per slice.

### Fase 3 — object-list (grade D)
- FE: 48 file flat → components/hooks/modals; gabung `useObjHandlers*` per domain nyata; rapikan 7 modal + 3 wrapper; perbaiki `useDeleteCollection as useDeleteObject`.
- Dedup: `BulkTagModal` bersama (vs folder-grid); konsolidasi ScanReview* vs scanner.
- BE: pecah `object_repo.rs` 1621 (queries/projections/classification); `object_cmds` lewat services.

### Fase 4 — Runtime core (workspace-runtime, mod-runtime, runtime-sync)
- FE: pangkas `useWorkspaceSwitchActions` 511, `useSharedObjectActions` 412, `useWorkspaceViewModel` 391; kurangi peran hub.
- BE: `workspace_cmds` 522 → workspace_service; refactor runtime engine (terlindungi test Fase 0); terapkan template handler ke ~25 site; pecah `mod_repo` 938 + mods services (`core_ops` 634, `bulk` 480, `object_switch` 437, `trash` 407); bersihkan unwrap (workspace_cmds 16, corridor_service 15, runtime_mutation_engine 11).

### Fase 5 — collections
- BE: `resolve_recovery_task` dari cmds → collection_service; repo langsung → services; test command layer; pecah `collection_service` 917 & `collection_repo` 756.
- FE: pecah `ApplyCollectionModal` 383.

### Fase 6 — scanner, file-watcher, duplicates
- BE: raw sqlx di `deepmatch_scanner_cmds` → repo; pecah deep_matcher >350 (`mechanical_rerank` 506, `gamebanana` 454, `full_pipeline` 442) & `sync/commit` 484; refactor `disk_reconcile`, bersihkan 12 unwrap.
- FE: konsolidasi overlap ScanReview (lanjutan Fase 3).

### Fase 7 — browser + downloads
- FE: fold `downloads` ke `browser`; pecah `BrowserPage` 505.
- BE: `browser_cmds` berhenti terima `SqlitePool` mentah; pecah `import_service` 823; test browser command layer + `browser_service`/`download_service`/`download_handler`.

### Fase 8 — folder-grid + preview
- folder-grid: 27 file flat → subfolder; pakai BulkTagModal bersama.
- preview: review kohesi 5 file 301–313; kurangi coupling; BE `preview_cmds` 356 tipiskan.

### Fase 9 — Fitur kecil + kebersihan akhir
- welcome → onboarding; dedup 2 UI PIN (safe-mode vs settings).
- `useFolderMutations` 533 pecah; rename `useObjects.test.tsx`.
- BE: lebur `database/` ke `repo/`; setup inline `lib.rs` (~170 baris) → service bootstrap; pecah `keyviewer/generator` 536, `explorer/listing` 526, `config/mod` 429, `domain/workspace` 404.
- Test: `conflict-report`, browser, repos tanpa test, `pipeline/`.
- Clippy jadi blocking; pertimbangkan `max-lines` jadi error.

## Prinsip eksekusi

- Pecah berdasarkan kohesi nyata, bukan angka 350 (pelajaran `useObjHandlers*` & preview).
- Reuse: `test_utils.rs`, `test/support/*`, pola audit-test yang ada.
- Hapus dead code terverifikasi saja.

## Verifikasi tiap fase

`pnpm lint` → `pnpm vitest run` (langsung, tanpa rtk) → `cargo test` (src-tauri) → `pnpm build`; E2E smoke manual (WDIO, lokal Windows) untuk fase yang menyentuh runtime/disk: 0, 4, 6.
