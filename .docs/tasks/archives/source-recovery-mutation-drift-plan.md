# Rencana Eksekusi: Source Recovery dan Mutation Anti-Drift

## Ringkasan

Perubahan ini menyempurnakan recovery ketika `DiskReconcileStatus::SourceUnavailable` dan menutup celah drift pada auto-organize, auto-category, thumbnail, preview image, `info.json`, dan INI. Filesystem tetap menjadi source of truth untuk keberadaan, path, identity, serta enabled/disabled state; database mempertahankan metadata aplikasi yang tidak dapat diturunkan dari disk.

Baseline saat ini sudah memiliki `WorkspaceSourceUnavailableDialog`, banner persisten, Retry, dan Locate Folder. Gap utamanya adalah kandidat directory disimpan sebelum diperiksa, belum ada klasifikasi matching/empty/different, belum ada konfirmasi destructive, dan beberapa mutation command belum menggunakan backend preflight yang sama dengan toggle/rename/bulk.

Tidak diperlukan migrasi database. Kontrak IPC dan generated bindings akan berubah.

## Invariant Arsitektur

- Reconcile selalu membaca snapshot disk terbaru sebelum menulis projection DB.
- `SourceUnavailable`, folder conflict, dan rename confirmation tidak boleh menghapus atau menulis projection parsial.
- Perubahan directory tidak boleh menyimpan `mod_path` sebelum kandidat diinspeksi dan, bila perlu, dikonfirmasi.
- Apply directory harus mengulang inspeksi di bawah operation lock; hasil yang berubah sejak preview ditolak sebagai stale.
- Mutation filesystem dari aplikasi mengikuti urutan: full preflight, operation lock, path validation, watcher suppression, disk mutation, release lock, trailing reconcile.
- Narrow mutation menggunakan path-scoped suppression. Blanket suppression hanya untuk write set yang memang tidak dapat diketahui sebelumnya.
- Category dan metadata aplikasi tidak boleh ditimpa reconcile hanya karena filesystem event lain terjadi.
- Collection tidak dihapus saat library kosong/different dipilih. Durable references dipertahankan dan anggota yang tidak ada ditandai missing.
- Watcher session lama tidak boleh mengirim reconcile/event setelah root game diganti.

## Klasifikasi Kandidat Directory

Tambahkan kontrak terstruktur berikut, dengan nama akhir menyesuaikan konvensi Rust/Specta repo:

- `Matching`: ada filesystem identity yang sama tanpa mapping kontradiktif, atau set relative stable path key sama persis. Aman untuk auto-apply.
- `NewLibrary`: DB belum memiliki object/mod runtime dan kandidat berisi library. Aman untuk auto-apply karena tidak ada projection lama yang akan hilang.
- `Empty`: directory benar-benar tidak memiliki entry. Memerlukan konfirmasi eksplisit.
- `Different`: directory berisi data tetapi tidak memenuhi bukti matching. Ini juga mencakup directory non-kosong yang tidak menghasilkan mod projection. Memerlukan konfirmasi keras.
- Kandidat missing, bukan directory, unreadable, atau gagal discan dikembalikan sebagai actionable validation error dan tidak dapat diterapkan.

Partial overlap tidak dianggap matching. Tidak digunakan percentage threshold atau heuristic magic number. UI tetap menampilkan identity/path match counts agar keputusan pengguna dapat diaudit.

`GameModsDirectoryInspection` minimal berisi:

- `game_id`, canonical candidate path, classification, dan deterministic fingerprint.
- Existing/candidate object dan mod counts.
- Filesystem identity match dan relative path match counts.
- Physical entry count agar empty dapat dibedakan dari layout non-empty yang tidak dikenali.
- Object/mod rows yang akan ditambah/dihapus, dibatasi untuk preview.
- Collection impact: affected count/names dan missing path count.
- `requires_confirmation` serta reason code terstruktur.

Fingerprint berasal dari canonical path, classification, dan sorted projection identities/path keys. Apply command wajib re-scan dan menolak fingerprint stale.

## Alur Apply Directory

1. Dialog meminta native folder picker memilih exact mods directory, bukan XXMI instance root.
2. `inspect_game_mods_directory(game_id, path)` melakukan full read-only scan tanpa mengubah config atau DB.
3. `Matching`/`NewLibrary` langsung diterapkan; `Empty` dan `Different` masuk confirmation view.
4. `change_game_mods_directory(game_id, path, fingerprint, confirmation)` memperoleh operation lock dan per-game reconcile lock, lalu mengulang inspeksi.
5. Bila fingerprint/classification berubah, command berhenti dan meminta user mereview ulang.
6. Watcher session lama di-invalidasi, settings lama disimpan untuk rollback, lalu `mod_path` baru dipersist.
7. Full reconcile dijalankan terhadap root baru dengan snapshot terbaru. Projection write tetap transaksional.
8. Error atau `SourceUnavailable` setelah switch memulihkan settings lama best-effort dan mempertahankan snapshot DB valid terakhir. Error menyebut status rollback.
9. `BlockedByFolderConflicts` atau `NeedsRenameConfirmation` bukan kegagalan directory switch: root baru tetap aktif dan resolution manager terkait dibuka.
10. Frontend memperbarui settings cache hanya dari result command yang berhasil. Watcher hook memulai session baru; session start selalu melakukan full reconcile untuk menutup event-history gap.

## UX Source Recovery

- Pertahankan auto-open sekali per report/source key dan kemampuan menutup dialog.
- Banner tetap persisten dan tombolnya membuka kembali dialog yang sama; hapus duplikasi implementasi Retry/Locate antara banner dan modal.
- State dialog: unavailable summary, selecting, inspecting, matching sync, empty confirmation, different confirmation, applying, success/error.
- Empty confirmation menjelaskan bahwa object/mod projection akan dibangun dari nol, sementara collection tetap ada dan referensi yang tidak ditemukan menjadi missing.
- Different confirmation menampilkan old/new path, counts, sample changes, dan collection impact. Pengguna harus mengetik nama game untuk mengaktifkan `Use this folder`.
- Kedua confirmation view mempunyai `Change directory`, `Cancel`, dan tombol confirm yang jelas.
- Setelah sukses, dialog menutup hanya setelah result `Applied` atau berpindah ke conflict/rename resolution manager bila blocked.
- Gunakan semantic DaisyUI tokens, focus trap/ring, keyboard navigation, reduced motion, responsive single column, dan i18n EN/ID/ZH.

## Mutation Coverage Matrix

| Area                            | Source of truth                                                | Perbaikan wajib                                                                                                                                  |
| ------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Auto-organize/move              | Disk path/identity                                             | Pertahankan preflight, lock, scoped suppression, source+destination reconcile, dan collection rewrites; tambah partial-failure regression tests. |
| Mod category                    | DB app metadata                                                | Reconcile mempertahankan category existing bila owner object tidak berubah; mod baru atau move antar-object mengambil category target.           |
| Object category/auto-category   | DB app metadata                                                | Update object dan seluruh child mods dalam transaction/lock yang sama; watcher reconcile tidak boleh mengembalikan value lama.                   |
| Auto-recognize                  | DB app metadata                                                | Gabungkan match provenance dan object metadata update secara atomik per object atau batch terkontrol; jangan meninggalkan half-applied row.      |
| `info.json`                     | Disk untuk isi file, DB untuk indexed projection/manual policy | Preflight, lock, scoped suppression, atomic replace/recovery, trailing reconcile.                                                                |
| Safe/favorite/pin/tags          | Explicit app metadata plus `info.json` mirror                  | Disk+DB update memiliki rollback yang terukur; reconcile tidak boleh menimpa manual policy.                                                      |
| Thumbnail update/paste/delete   | Disk image                                                     | Preflight, canonical game-scoped validation, atomic write/Trash, cache invalidation, trailing reconcile.                                         |
| Preview image save/remove/clear | Disk image                                                     | Preflight, atomic write/Trash, scoped suppression, thumbnail/detail refresh.                                                                     |
| INI edit                        | Disk file                                                      | Pertahankan source-hash stale check dan backup recovery; tambahkan preflight dan path-scoped suppression.                                        |
| External image/INI/info changes | Disk event                                                     | Watcher mengklasifikasi thumbnail/runtime paths, invalidates query/cache, dan reconcile menggunakan latest snapshot.                             |

## Task Eksekusi

### Task 1: Candidate inspection contract dan classifier

**Description:** Tambahkan DTO dan pure classifier untuk membandingkan projection DB valid terakhir dengan full snapshot kandidat directory. Inspection bersifat read-only dan tidak memakai `resolve_game_folder`, karena folder yang dipilih adalah exact mods root.

**Acceptance criteria:**

- [ ] Matching identity, exact relative-key copy, new library, physical empty, non-empty unrecognized, different, unreadable, dan missing path memiliki hasil deterministik.
- [ ] Partial overlap diklasifikasikan `Different`.
- [ ] Inspection tidak mengubah settings, objects, mods, collection members, atau watcher.

**Verification:**

- [ ] RED lalu GREEN Rust unit tests untuk seluruh classification matrix.
- [ ] Rust integration test membandingkan DB counts sebelum/sesudah inspection.

**Dependencies:** None.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/source_recovery.rs`
- `src-tauri/src/services/disk_reconcile/types.rs`
- `src-tauri/src/commands/scanner/disk_reconcile_cmds.rs`
- `src-tauri/src/lib.rs`

**Estimated scope:** Medium.

### Task 2: Stale-safe directory apply dan config rollback

**Description:** Tambahkan apply service/command yang re-inspect kandidat di bawah lock, memverifikasi fingerprint, mengganti `mod_path`, menjalankan full reconcile, dan rollback config pada terminal failure.

**Acceptance criteria:**

- [ ] Matching/NewLibrary dapat diterapkan tanpa destructive confirmation; Empty/Different ditolak tanpa confirmation yang sesuai.
- [ ] Candidate berubah setelah preview menghasilkan stale error tanpa settings/DB write.
- [ ] Reconcile failure memulihkan old `mod_path`; blocked conflict/rename mempertahankan new path dan result terstruktur.

**Verification:**

- [ ] RED/GREEN Rust tests untuk success, stale token, empty/different authorization, config-write failure, reconcile failure, rollback failure, dan blocked result.
- [ ] Test memastikan collection rows tidak dihapus ketika empty/different dikonfirmasi.

**Dependencies:** Task 1.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/source_recovery.rs`
- `src-tauri/src/commands/scanner/disk_reconcile_cmds.rs`
- `src-tauri/src/services/config/service.rs`
- `src-tauri/src/repo/game_repo.rs` atau module repo settings yang sudah ada

**Estimated scope:** Medium.

### Task 3: Watcher generation untuk root replacement

**Description:** Beri setiap watcher lifecycle sebuah session generation. Start/stop/root change meng-invalidasi generation lama, dan event loop lama berhenti sebelum reconcile atau emit result.

**Acceptance criteria:**

- [ ] Event dari root lama setelah directory switch tidak mereconcile game memakai path baru.
- [ ] Session baru melakukan full recovery pass sebelum scoped events.
- [ ] Failed switch tidak meninggalkan dua watcher aktif.

**Verification:**

- [ ] RED/GREEN watcher lifecycle tests dengan delayed old-session event.
- [ ] Test rapid start/stop/start dan root A ke B.

**Dependencies:** Task 2 contract, tetapi dapat diimplementasikan setelah DTO Task 1 stabil.

**Files likely touched:**

- `src-tauri/src/services/scanner/watcher/mod.rs`
- `src-tauri/src/services/scanner/watcher/lifecycle.rs`
- `src-tauri/src/commands/scanner/watcher_cmds.rs`
- `src-tauri/src/services/scanner/tests/watcher_tests.rs`

**Estimated scope:** Medium.

### Checkpoint A: Backend source recovery

- [ ] Targeted source recovery dan watcher tests lulus.
- [ ] `cargo fmt --check` lulus.
- [ ] Kandidat read-only dan apply failure terbukti tidak mengubah projection parsial.

### Task 4: Source recovery dialog state machine

**Description:** Refactor dialog yang sudah ada agar memakai inspection/apply commands. Banner hanya menjadi persistent entry point ke dialog yang sama.

**Acceptance criteria:**

- [ ] Auto-open once, close, persistent banner, dan reopen bekerja per game/report.
- [ ] Matching auto-sync; Empty dan Different menampilkan confirmation serta `Change directory`.
- [ ] Different membutuhkan typed game name; invalid/stale/apply errors tetap berada di dialog dengan aksi retry/change directory.

**Verification:**

- [ ] RED/GREEN component tests untuk semua state dan keyboard/focus behavior.
- [ ] Test memastikan settings cache tidak berubah sebelum backend apply sukses.

**Dependencies:** Tasks 1-3.

**Files likely touched:**

- `src/features/folder-grid/components/WorkspaceSourceUnavailableDialog.tsx`
- `src/features/folder-grid/components/WorkspaceSourceUnavailableBanner.tsx`
- `src/features/folder-grid/components/WorkspaceSourceUnavailableDialog.test.tsx`
- `src/features/workspace-runtime/state/workspaceDialogs.ts`

**Estimated scope:** Medium.

### Task 5: Source recovery i18n dan generated contract

**Description:** Regenerate Specta bindings, register permissions, dan menambahkan copy EN/ID/ZH untuk inspection, counts, empty/different warnings, stale candidate, rollback, dan change-directory actions.

**Acceptance criteria:**

- [ ] Semua command baru allowed oleh Tauri dan typed bindings digunakan tanpa handwritten duplicate types.
- [ ] Tidak ada missing/extra translation key di tiga locale.
- [ ] Dialog responsive dan semantic tokens tidak memperkenalkan hardcoded color/text.

**Verification:**

- [ ] `pnpm i18n:lint` dan targeted frontend tests lulus.
- [ ] `pnpm build` membuktikan contract frontend/backend konsisten.

**Dependencies:** Task 4.

**Files likely touched:**

- `src-tauri/permissions/app-commands.toml`
- `src/lib/bindings.gen.ts`
- `src/locales/en/grid.json`
- `src/locales/id/grid.json`
- `src/locales/zh/grid.json`

**Estimated scope:** Medium; split locale/generated changes bila file limit perlu dijaga.

### Task 6: Category persistence dan atomic auto-category

**Description:** Perbaiki projection writer agar existing per-mod category dipertahankan selama owner object sama. Explicit object-category update tetap memperbarui object dan seluruh child secara atomik; move antar-object menggunakan target category.

**Acceptance criteria:**

- [ ] `set_mod_category` bertahan setelah manual, watcher, scoped, dan full reconcile.
- [ ] Object category update mengubah child mods dalam transaction yang sama.
- [ ] Mod baru/moved mengambil target object category; rename/toggle tidak mengubah category.

**Verification:**

- [ ] RED/GREEN projection writer tests untuk category preserve/move/new.
- [ ] Command/service tests untuk object+children transaction rollback.

**Dependencies:** None; dikerjakan setelah Checkpoint A untuk mengurangi overlap pada projection writer.

**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/projection_writer/mods.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/tests.rs`
- `src-tauri/src/commands/mods/mod_meta_cmds.rs`
- `src-tauri/src/repo/mod_repo/update.rs`

**Estimated scope:** Medium.

### Task 7: Atomic auto-recognize metadata

**Description:** Satukan match provenance dan object metadata/category update per object dalam satu backend transaction yang dikoordinasikan dengan reconcile lock. Bulk frontend memanggil satu atomic command per object dan tetap melaporkan matched/skipped secara parsial.

**Acceptance criteria:**

- [ ] Failure pada salah satu write tidak meninggalkan half-applied match metadata.
- [ ] Watcher reconcile tidak dapat interleave di tengah update object.
- [ ] Existing object metadata tetap dipertahankan oleh subsequent reconcile.

**Verification:**

- [ ] RED/GREEN Rust transaction rollback test dan frontend payload/result tests.
- [ ] Regression test watcher event yang tiba bersamaan dengan auto-recognize.

**Dependencies:** Task 6.

**Files likely touched:**

- `src-tauri/src/commands/objects/object_cmds.rs`
- `src-tauri/src/services/objects/mutate.rs`
- `src-tauri/src/repo/object_repo/update.rs`
- `src/features/object-list/utils/runBulkAutoRecognize.ts`

**Estimated scope:** Medium.

### Task 8: Metadata/info.json mutation convergence

**Description:** Terapkan backend preflight, path-scoped watcher suppression, operation lock, atomic `info.json` replace, dan rollback yang terukur untuk update info serta safe/favorite/pin/tag mirrors.

**Acceptance criteria:**

- [ ] Source unavailable/conflict/rename confirmation memblokir file mutation sebelum side effect.
- [ ] DB failure setelah disk write memulihkan file lama atau melaporkan recovery path; tidak silent drift.
- [ ] Successful write diikuti reconcile yang memperbarui runtime metadata dan selection/query refresh.

**Verification:**

- [ ] RED/GREEN tests untuk blocked preflight, atomic replace, concurrent external edit, DB failure, rollback, dan watcher echo suppression.
- [ ] Existing info JSON tests tetap lulus.

**Dependencies:** Checkpoint A.

**Files likely touched:**

- `src-tauri/src/commands/mods/mod_meta_cmds.rs`
- `src-tauri/src/services/mods/info_json.rs`
- `src-tauri/src/services/mods/metadata.rs`
- `src-tauri/src/services/mods/tests/info_json_tests.rs`

**Estimated scope:** Medium.

### Checkpoint B: App metadata stability

- [ ] Category dan auto-recognize survives full reconcile.
- [ ] Info JSON and DB rollback tests lulus.
- [ ] No SQL error atau partial projection pada injected failures.

### Task 9: Thumbnail CRUD hardening

**Description:** Tambahkan preflight untuk update/paste/delete thumbnail, gunakan `game_id` eksplisit dan canonical `ValidatedPath`, stage image sebelum atomic replace, Trash untuk delete, serta invalidate cache folder dan source yang tepat.

**Acceptance criteria:**

- [ ] Direct IPC tidak dapat menulis di luar game root atau saat reconcile blocked.
- [ ] App thumbnail write tidak meninggalkan partial file atau stale L1/L2 cache.
- [ ] External thumbnail create/modify/delete tetap menginvalidasi card dan preview queries lewat watcher.

**Verification:**

- [ ] RED/GREEN command tests untuk preflight, traversal/symlink, replace failure, Trash, cache invalidation, dan reconcile payload.
- [ ] Frontend hook test memastikan exact thumbnail query invalidation.

**Dependencies:** Checkpoint A.

**Files likely touched:**

- `src-tauri/src/commands/mods/mod_thumbnail_cmds.rs`
- `src-tauri/src/commands/folder_grid/mod.rs`
- `src-tauri/src/services/mods/metadata.rs`
- `src-tauri/src/commands/mods/tests/mod_thumbnail_cmds_tests.rs`

**Estimated scope:** Medium.

### Task 10: Preview image dan INI CRUD convergence

**Description:** Tambahkan preflight dan path-scoped suppression pada save/remove/clear preview serta INI write. Preview save memakai staged atomic write; remove/clear tetap soft-delete ke Trash. Pertahankan INI source fingerprint dan backup generations.

**Acceptance criteria:**

- [ ] Semua mutation diblokir sebelum write saat source unavailable/conflict.
- [ ] Unrelated external watcher event tidak hilang selama narrow mutation.
- [ ] App-originated event tidak menghasilkan duplicate refresh/toast, tetapi trailing reconcile selalu memperbarui metadata/thumbnail scopes.

**Verification:**

- [ ] RED/GREEN command/service tests untuk save/remove/clear/write, stale INI, partial image failure, watcher suppression, dan reconcile result.
- [ ] Existing preview/INI suites tetap lulus.

**Dependencies:** Checkpoint A.

**Files likely touched:**

- `src-tauri/src/commands/mods/preview_cmds.rs`
- `src-tauri/src/services/mods/preview_image.rs`
- `src-tauri/src/services/mods/tests/preview_image_tests.rs`
- `src-tauri/src/services/mods/tests/preview_ops_tests.rs`

**Estimated scope:** Medium.

### Task 11: Auto-organize and collection regression gate

**Description:** Pertahankan implementasi auto-organize yang sudah benar dan tambahkan tests untuk partial filesystem success, DB failure, parent/child moves, category transfer, collection path rewrites, dan watcher events pada source/destination.

**Acceptance criteria:**

- [ ] Partial move tetap converges dari disk melalui source+destination reconcile.
- [ ] Collection member mengikuti identity/path baru atau menjadi missing bila benar-benar hilang.
- [ ] Parent/child dan nested target subpath tidak menghasilkan duplicate object/mod rows.

**Verification:**

- [ ] Targeted Rust integration tests dan frontend move result tests lulus.

**Dependencies:** Task 6.

**Files likely touched:**

- `src-tauri/src/commands/mods/tests/mod_meta_cmds_tests.rs`
- `src-tauri/src/services/mods/tests/organizer_move_tests.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/tests.rs`
- `src/features/mod-runtime/operations/sharedOperations.test.ts`

**Estimated scope:** Medium.

### Task 12: End-to-end recovery and external-mutation coverage

**Description:** Tambahkan E2E untuk missing source recovery, directory classification/confirmation, watcher restart, serta file/category mutation convergence.

**Acceptance criteria:**

- [ ] Missing root auto-opens dialog; Retry memulihkan root yang kembali.
- [ ] Matching auto-sync, Empty confirm/change-directory, dan Different typed confirm bekerja tanpa stale DB view.
- [ ] External rename/category-relevant info/thumbnail/INI changes setelah app restart atau watcher restart memperbarui UI dan DB tanpa duplicate toast.

**Verification:**

- [ ] Targeted E2E specs lulus pada fixture terisolasi.
- [ ] Manual Windows check untuk native picker, Explorer, Trash, dan Recycle Bin restore.

**Dependencies:** Tasks 4-11.

**Files likely touched:**

- `test/specs/source-unavailable-recovery.e2e.ts`
- `test/specs/phase3c-folder-conflicts.e2e.ts`
- `src/features/file-watcher/ExternalChangeHandler.test.tsx`
- `src/features/file-watcher/hooks.test.ts`

**Estimated scope:** Medium.

### Task 13: Verification, architecture audit, dan documentation

**Description:** Jalankan seluruh gates, audit ulang setiap filesystem writer, dan dokumentasikan final invariants serta manual limitations.

**Acceptance criteria:**

- [ ] Semua mutation command yang menyentuh mods root memiliki preflight/lock/suppression/reconcile contract atau documented read-only/DB-only exemption.
- [ ] Tidak ada generated binding/permission/i18n drift.
- [ ] `.agents/` dan unrelated user changes tetap tidak disentuh.

**Verification:**

- [ ] Targeted Rust dan frontend tests.
- [ ] `cargo test`.
- [ ] `pnpm test -- --run`.
- [ ] `pnpm lint`.
- [ ] `pnpm i18n:lint`.
- [ ] `pnpm build`.
- [ ] `cargo fmt --check`.
- [ ] `git diff --check`.
- [ ] Relevant E2E specs.

**Dependencies:** Tasks 1-12.

**Files likely touched:**

- `src-tauri/tests/arch_audit.rs`
- `.docs/history/<timestamp>-source-recovery-mutation-anti-drift.md`
- `tasks/source-recovery-mutation-drift-todo.md`

**Estimated scope:** Small.

## Risks dan Mitigasi

| Risk                                                                           | Impact | Mitigation                                                                                                                    |
| ------------------------------------------------------------------------------ | ------ | ----------------------------------------------------------------------------------------------------------------------------- |
| Empty/different switch menghapus runtime rows yang sebenarnya masih diperlukan | High   | Read-only preview, explicit confirmation, collection durable refs, transactional reconcile, config rollback.                  |
| Candidate berubah antara preview dan apply                                     | High   | Deterministic fingerprint dan mandatory re-inspection under lock.                                                             |
| Old watcher emits after root change                                            | High   | Session generation invalidation plus full reconcile on new session.                                                           |
| OperationLock deadlock karena nested orchestrator acquisition                  | High   | Apply service memakai already-held `OpGuard` dan direct reconcile under per-game lock; tidak memanggil acquiring entry point. |
| Category custom ditimpa projection                                             | Medium | Preserve existing type on same owner; explicit tests for scoped/full/watcher reconcile.                                       |
| Cross-store info.json/DB update gagal separuh                                  | High   | Staged replace, backup, DB transaction, verified rollback/error reporting.                                                    |
| Narrow app mutation menelan external event lain                                | Medium | Path-scoped suppression; blanket only for unknown write sets.                                                                 |
| Thumbnail cache menunjuk file lama                                             | Medium | Invalidate source and identity-keyed folder caches after success/delete/external event.                                       |
| Existing dirty worktree overlap                                                | High   | Re-read exact diff before each edit, patch narrowly, never revert unrelated changes.                                          |

## Urutan Checkpoint

1. Tasks 1-3: backend inspection/apply/watcher lifecycle.
2. Checkpoint A.
3. Tasks 4-5: dialog, contract, i18n.
4. Tasks 6-8: category, auto-recognize, info metadata.
5. Checkpoint B.
6. Tasks 9-11: thumbnail, preview/INI, auto-organize regression.
7. Task 12: E2E and manual recovery checks.
8. Task 13: full verification and documentation.

Implementation harus mengikuti TDD per task: tulis test, jalankan dan pastikan gagal karena behavior belum ada, implement minimal fix, lalu jalankan targeted regression sebelum lanjut.
