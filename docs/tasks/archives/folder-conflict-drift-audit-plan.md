# Implementation Plan: Anti-Drift Reconcile, Watcher, dan Folder Conflict

## Ringkasan

Audit ini memeriksa perubahan folder-conflict yang sedang ada beserta jalur lama yang berinteraksi dengannya: reconcile penuh/scoped, enable-disable tunggal dan bulk, file watcher, operasi rename/delete/import, stable ID, serta collection references. Implementasi conflict preflight saat ini sudah mencegah SQLite 1555 pada jalur reconcile normal, tetapi belum cukup untuk menjamin konvergensi pada semua interleaving filesystem, watcher, dan operasi aplikasi.

Plan ini tidak mengubah asumsi utama bahwa filesystem adalah source of truth. Targetnya adalah satu state machine yang dapat membuktikan bahwa setelah setiap operasi, database dan collection berada pada salah satu kondisi eksplisit berikut:

1. `Applied`: snapshot disk, row DB, stable ID, status, object link, dan collection references konsisten.
2. `BlockedByFolderConflicts`: tidak ada projection write generik; DB tetap pada snapshot valid terakhir, sementara antrean konflik berasal dari satu snapshot disk koheren.
3. `SourceUnavailable`: tidak ada destructive prune atau partial write.
4. `NeedsRenameConfirmation`: hanya untuk rename eksternal yang tidak memiliki bukti identitas satu-ke-satu; aplikasi tidak menebak.
5. `Failed`: mutation atau convergence gagal dan state ditandai dirty untuk full retry, bukan dilaporkan sukses diam-diam.

## Scope audit

- Diff folder-conflict backend/frontend yang sedang ada.
- Reconcile projection, rename healer, prune, orchestrator, dan event emission.
- Watcher classification, filtering, buffering, suppression, dan error toast.
- Toggle/rename/delete/import/trash tunggal maupun bulk.
- Collection path rewrite, missing references, projected signatures, dan isolasi antar-game.
- Conflict manager, details scan, game switching, dan path rewrite UI.
- Security boundary Tauri: `game_id`, absolute path, `group_id`, rename drafts, Explorer, Trash, dan recursive directory scan.

Tidak termasuk perubahan `.agents/` dan pekerjaan pengguna lain. `tasks/plan.md` serta `tasks/todo.md` yang sudah ada juga tidak ditimpa karena merupakan plan 3DMigoto terpisah.

## Definisi aplikasi tidak aktif

Plan membedakan tiga kondisi yang harus mempunyai recovery berbeda:

1. **Window tidak fokus/minimized, proses masih hidup.** Watcher tetap menerima event. Event disimpan/coalesce per game; focus hanya menjadi verification trigger, bukan satu-satunya sumber perubahan.
2. **Suspend/sleep, watcher restart, drive/network source sempat hilang.** Resume/refocus menjalankan availability check dan full reconcile bila watcher melaporkan gap, overflow, atau session generation berubah.
3. **Proses aplikasi benar-benar tertutup.** Tidak ada event yang dapat dipercaya. Pada startup/game activation, full recovery reconcile wajib selesai sebelum workspace Mods ditandai hydrated atau collection dapat diterapkan.

Untuk kondisi ketiga, create/delete dapat ditentukan dari full snapshot. Prefix-only enable/disable dapat dipetakan melalui normalized identity. Semantic rename/move membutuhkan optional filesystem identity persisten; bila tidak tersedia atau ambigu, aplikasi tidak boleh menebak dan harus menghasilkan `NeedsRenameConfirmation`.

## Invariant yang wajib dipenuhi

### I1 — Satu snapshot untuk satu keputusan

Conflict detection dan projection writer harus memakai snapshot immutable yang sama. Scoped writer boleh memakai view terfilter dari full snapshot tersebut, bukan melakukan dua filesystem walk pada waktu berbeda.

### I2 — Satu game, satu urutan mutation/reconcile

Filesystem mutation aplikasi dan reconcile untuk game yang sama tidak boleh berjalan bersamaan. Semua entry point mengikuti lock order yang sama dan trailing reconcile tidak melakukan re-acquire lock secara reentrant.

### I3 — Suppression selalu punya recovery

Path-scoped suppression hanya menahan echo pada path milik operasi. Blanket suppression yang membuang event wajib menghasilkan dirty generation dan full reconcile sebelum operasi dianggap converged.

### I4 — Collection selalu game-scoped

Lookup, rewrite, missing impact, dan signature recomputation tidak boleh menyentuh collection milik game lain walaupun relative path sama.

### I5 — Tidak ada partial identity

Perubahan folder yang dianggap selesai harus memperbarui dalam satu transaksi: path/display metadata, normalized key, stable ID, status, object link, child paths, collection references, dan projected signature.

### I6 — Rename eksternal tidak ditebak

Stitched watcher rename boleh dianggap bukti. Rename yang terjadi saat aplikasi mati atau saat event hilang hanya boleh auto-heal bila ada identitas filesystem persisten dan unik. Bila buktinya ambigu, state harus eksplisit dan meminta konfirmasi.

### I7 — Failure tetap terlihat

Kegagalan DB, rollback, reconcile, runtime projection, atau collection impact tidak boleh hanya dicatat ke log lalu command mengembalikan sukses.

## Coverage saat ini

| Skenario                                                             | Status saat ini     | Catatan                                                                                                                                                                                                     |
| -------------------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Mod A` + `DISABLED Mod A` saat full reconcile                       | Tercakup            | Preflight mengembalikan `BlockedByFolderConflicts` sebelum transaksi.                                                                                                                                       |
| Konflik pada object root enabled/disabled                            | Tercakup            | Object identity ikut masuk conflict groups.                                                                                                                                                                 |
| Scoped reconcile melihat konflik di root lain                        | Tercakup            | Ada full conflict preflight, tetapi masih memakai walk kedua yang tidak koheren.                                                                                                                            |
| Toggle child dari aplikasi                                           | Tercakup sebagian   | Path-scoped suppression dan trailing reconcile ada; reconcile masih dapat beradu dengan mutation lain/refocus.                                                                                              |
| Toggle parent object                                                 | Tercakup sebagian   | Child path berubah dan child status dipertahankan; concurrency dan collection scoping masih bermasalah.                                                                                                     |
| Bulk toggle                                                          | Tercakup sebagian   | Item berhasil direconcile; kegagalan trailing reconcile hanya di-log.                                                                                                                                       |
| Rename langsung di disk saat watcher memberi event `Renamed` depth 2 | Tercakup            | Rename healer memperbarui row dan collection.                                                                                                                                                               |
| Rename nested terminal mod depth 3+                                  | Belum tercakup      | Rename hint dan healer membatasi depth tepat 2.                                                                                                                                                             |
| Rename saat aplikasi tertutup/event watcher hilang                   | Belum dapat dijamin | Path-derived ID tidak memberi bukti bahwa old dan new adalah folder yang sama.                                                                                                                              |
| Create/delete saat aplikasi tertutup                                 | Tercakup sebagian   | Full scan dapat menemukan delta, tetapi startup hydration belum mempunyai explicit recovery gate dan parent-delete collection masih bermasalah.                                                             |
| Prefix toggle saat aplikasi tertutup                                 | Tercakup sebagian   | Normalized runtime key dapat heal status/path; harus dibuktikan untuk parent, child, nested, dan collection sebelum UI hydrate.                                                                             |
| Suspend/resume atau watcher restart                                  | Tercakup sebagian   | Refocus/TTL refresh ada; belum ada watcher-session generation yang memastikan full scan setelah event gap.                                                                                                  |
| Delete child langsung dari disk                                      | Tercakup sebagian   | Row dipruning dan collection ditandai missing, tetapi belum game-scoped.                                                                                                                                    |
| Delete parent langsung dari disk                                     | Belum tercakup      | Object prune menghapus child rows tanpa missing impact; FK `collection_mods.object_id` dan `collection_objects.object_id` memakai `ON DELETE CASCADE`, sehingga membership snapshot juga dapat ikut hilang. |
| Bulk delete saat ada perubahan eksternal lain                        | Belum aman          | Blanket suppression membuang event lain dan hanya scoped reconcile ke item sukses.                                                                                                                          |
| Resolve satu group saat group lain masih konflik                     | Belum aman          | DB exact path ditulis, lalu full reconcile tetap blocked; stable ID/metadata dapat tertinggal parsial.                                                                                                      |
| Crash di tengah two-phase rename                                     | Belum aman          | Folder `.emmm-conflict-stage-*` dapat tertinggal dan tidak terlihat snapshot.                                                                                                                               |
| Watcher error toast duplikat                                         | Tercakup sebagian   | Dedupe 3 detik ada, tetapi map tidak dibatasi/prune global.                                                                                                                                                 |
| Command conflict IPC                                                 | Tercakup            | Ketiga command sudah ada di permission allow-list.                                                                                                                                                          |

## Temuan audit terprioritas

### F01 — Collection rewrite dapat melintasi game

**Severity:** High  
**Evidence:** `repo/collection_repo/references.rs` melakukan update/lookup berdasarkan `mod_path` tanpa filter `collections.game_id`; service API juga tidak menerima `game_id`.  
**Impact:** rename/delete `Alice/Blue` pada game A dapat rewrite atau menandai collection game B yang memiliki relative path sama.  
**Fix direction:** semua reference API wajib menerima `game_id`; query join ke `collections` dan filter game dalam transaksi yang sama.

### F02 — Resolve group dapat meninggalkan DB parsial bila konflik lain masih ada

**Severity:** High  
**Evidence:** conflict resolver hanya mengubah exact path/key dan collection reference. Full reconcile sesudahnya kembali `BlockedByFolderConflicts` bila group lain tersisa.  
**Impact:** row group yang baru selesai dapat memiliki stable `mods.id`, `actual_name`, status, atau child identity lama sampai seluruh antrean selesai. DB bukan lagi snapshot valid yang utuh.  
**Fix direction:** commit selected group melalui transaction khusus yang memakai before/after snapshot dan menulis seluruh identity fields; generic reconcile tetap diblokir oleh group lain.

### F03 — Delete parent langsung melewati collection missing impact anak

**Severity:** High  
**Evidence:** `prune_missing_objects` memanggil `delete_object_and_mods_by_folder`; `prune_missing_mods` dilewati untuk child milik object yang sudah ditandai deleted. Selain itu, schema menggunakan `collection_mods.object_id -> objects.id ON DELETE CASCADE` dan `collection_objects.object_id -> objects.id ON DELETE CASCADE`.  
**Impact:** collection bukan hanya gagal mendapat missing impact; member mod/object dapat terhapus dari snapshot ketika runtime object dihapus. Restore folder kemudian tidak mempunyai reference yang dapat direbind.  
**Fix direction:** pisahkan logical collection identity dari nullable runtime foreign key melalui migration aditif/rebuild table. Setelah itu, capture semua child paths, null/rebind runtime IDs, hitung missing roots, persist signature/snapshot, baru hapus runtime object dalam transaksi yang sama.

### F04 — Reconcile dan filesystem mutation memakai lock berbeda

**Severity:** High  
**Evidence:** app mutation memakai global `OperationLock`, sedangkan reconcile hanya memakai per-game lock di `DiskReconcileState`.  
**Impact:** refocus/manual/watcher reconcile dapat memotret staging atau setengah batch rename/delete dan menulis projection sementara.  
**Fix direction:** tambahkan game mutation lease yang dipakai mutation dan reconcile; trailing reconcile menerima proof guard agar tidak deadlock/reentrant.

### F05 — Blanket suppression dapat kehilangan perubahan eksternal

**Severity:** High  
**Evidence:** suppressor secara eksplisit membuang semua event. Beberapa flow, termasuk bulk delete, hanya menjalankan scoped reconcile atau tidak menjamin full reconcile setelah guard.  
**Impact:** perubahan user di folder lain selama import/delete/scan tidak pernah masuk antrean sampai kebetulan ada full refresh berikutnya.  
**Fix direction:** gunakan path-scoped suppression bila write-set diketahui; blanket guard wajib menandai `dirty_generation` dan full reconcile.

### F06 — Rename eksternal setelah restart/overflow tidak dapat dikorelasikan

**Severity:** High bila auto-heal penuh adalah requirement  
**Evidence:** stable ID dibuat dari normalized path; watcher event adalah satu-satunya bukti hubungan old/new untuk semantic rename. Manual full reconcile tanpa event melihat satu row missing dan satu folder baru.  
**Impact:** collection path menjadi missing dan folder baru mendapat identity baru walaupun sebenarnya hanya rename.  
**Fix direction:** simpan optional filesystem identity persisten (`volume/device + file index/inode + creation discriminator`) dalam tabel aditif. Auto-heal hanya pada match unik; unsupported/ambiguous masuk `NeedsRenameConfirmation`.

### F07 — Nested terminal rename dibatasi depth 2

**Severity:** Medium-High  
**Evidence:** `watcher_batch.rs` hanya menerima mod rename bila `from_depth == 2 && to_depth == 2`; `rename_healer.rs` juga menolak destination dengan component count selain 2, sementara snapshot mendukung terminal mod nested.  
**Impact:** rename `Object/Container/Mod A` tidak merewrite collection sebagai rename.  
**Fix direction:** klasifikasi berdasarkan entry terminal pada snapshot dan owning object root, bukan component count tetap.

### F08 — Scoped reconcile memakai dua walk disk berbeda

**Severity:** Medium-High  
**Evidence:** scoped projection dikumpulkan dulu, lalu full projection terpisah hanya untuk preflight.  
**Impact:** disk dapat berubah di antara kedua walk; keputusan conflict dan data yang ditulis tidak berasal dari state yang sama.  
**Fix direction:** satu full immutable snapshot; derive scoped view dari snapshot itu. Queue version/generation memicu retry bila event baru masuk selama walk.

### F09 — Removed/renamed deep folder bertitik dapat terfilter

**Severity:** Medium  
**Evidence:** event filter menggunakan extension/stat; path directory seperti `Variant v1.2` yang sudah dihapus tidak lagi `is_dir()` dan dapat dianggap asset noise.  
**Impact:** tidak ada changed root sampai refocus/full scan; rename correlation juga hilang.  
**Fix direction:** filter berdasarkan event kind; selalu pertahankan Removed dan kedua sisi Renamed di bawah mods root, lalu coalesce ke root.

### F10 — Two-phase staging tidak crash-safe

**Severity:** Medium-High  
**Evidence:** rollback hanya berjalan dalam process; hidden `.emmm-conflict-stage-*` diabaikan snapshot.  
**Impact:** power loss/process crash dapat membuat folder tampak hilang permanen dari aplikasi.  
**Fix direction:** journal durable sebelum phase 1, fsync, recovery idempotent saat startup/sebelum reconcile; tidak pernah menghapus staging otomatis tanpa bukti.

### F11 — Beberapa mutation menelan convergence failure

**Severity:** Medium-High  
**Evidence:** bulk toggle/delete dan beberapa rename/import flow hanya `log::warn!`/`log::error!` saat DB atau reconcile gagal lalu tetap mengembalikan success item.  
**Impact:** UI menyatakan selesai ketika DB/collection belum converged.  
**Fix direction:** result terstruktur memisahkan filesystem applied dari convergence status; kegagalan menandai game dirty dan menyediakan retry.

### F12 — Recursive details scan tidak dibatasi dan berjalan sinkron

**Severity:** Medium  
**Evidence:** async Tauri command menjalankan full `WalkDir`, menyimpan semua file lalu truncate preview ke 20; walk errors diabaikan.  
**Impact:** folder besar dapat memblokir runtime thread, memakai memori besar, dan menampilkan total yang tampak final walau partial.  
**Fix direction:** `spawn_blocking`, streaming top-20, checked/saturating totals, cancellation/limit, serta `partial`/warnings pada DTO.

### F13 — Buffer watcher dan dedupe map tidak bounded

**Severity:** Medium  
**Evidence:** watcher memakai `mpsc::unbounded_channel`; frontend menyimpan setiap kombinasi error dalam module-level `Map` tanpa periodic pruning.  
**Impact:** event storm/error path beragam dapat menumbuhkan memori.  
**Fix direction:** bounded channel dengan overflow marker yang memaksa full reconcile; TTL/LRU bounded untuk toast dedupe dan auto-open report cache.

### F14 — Prefix query collection memperlakukan `%` dan `_` sebagai wildcard

**Severity:** Medium  
**Evidence:** `LIKE format!("{prefix}%")` tidak meng-escape wildcard SQL.  
**Impact:** object bernama `A_B` atau `100%` dapat merewrite member object lain.  
**Fix direction:** gunakan normalized component key/range query atau escaped `LIKE ... ESCAPE` lalu verifikasi boundary path di Rust.

### F15 — Detail request UI dapat race saat pindah game/group

**Severity:** Medium  
**Evidence:** promise details tidak memiliki request generation/cancellation; response lama dapat menimpa state group baru. Error hanya toast dan kartu jatuh ke ukuran 0 tanpa status retry.  
**Impact:** user dapat melihat metadata/konfirmasi Trash yang salah atau tidak tahu scan partial/gagal.  
**Fix direction:** ikat dialog ke `{game_id, report_id}`, abaikan response stale, dan render loading/error/retry per candidate.

### F16 — Path update contract bercampur absolute dan relative

**Severity:** Medium  
**Evidence:** reconcile normal menghasilkan relative mod paths, sedangkan conflict resolver menambahkan absolute rewrites; frontend mengandalkan `activeGame` untuk mengubah relative menjadi absolute tetapi resolver memanggil mapper dengan `null`.  
**Impact:** selection/cache rewrite bergantung pada asal result dan mudah drift pada parent/nested path.  
**Fix direction:** tetapkan wire contract tunggal, disarankan semua `DiskReconcilePathUpdate` relative terhadap mods root dengan converter di satu boundary.

### F17 — Direct rename conflict masih memakai state/dialog lama

**Severity:** Low-Medium  
**Evidence:** `WorkspaceRenameConflict` tetap dirender oleh UI lama, bukan dinormalisasi menjadi conflict report/group yang sama.  
**Impact:** validation, collection handling, retry, dan accessibility dapat berbeda antara konflik yang ditemukan command rename dan konflik yang ditemukan watcher.  
**Fix direction:** adapter legacy -> `FolderNameConflictGroup`, satu manager dan satu backend resolution contract.

### F18 — Collection impact merge tidak mendupe path rewrite

**Severity:** Low  
**Evidence:** affected names dan missing paths didedupe, tetapi `rewritten_paths` selalu di-extend.  
**Impact:** duplicate UI rewrite/invalidation dan jumlah kerja berlebih pada batch kompleks.  
**Fix direction:** dedupe berdasarkan normalized `(from,to)` saat merge.

## Keputusan arsitektur yang direkomendasikan

### A. Coherent `DiskSnapshotEnvelope`

Satu full walk menghasilkan:

- object dan terminal-mod entries;
- normalized runtime identity;
- optional filesystem identity;
- snapshot generation/start-end time;
- read warnings/partial flag;
- conflict groups;
- scoped view per changed root.

Writer tidak boleh membaca ulang filesystem untuk mengambil keputusan identity. Bila watcher queue version berubah selama snapshot, pass saat ini boleh selesai read-only tetapi writer harus retry dari snapshot baru sebelum commit.

### B. `GameDiskCoordinator` dan mutation lease

Gunakan satu koordinasi per game untuk mutation + reconcile. Lock order tetap:

```text
OperationLock (untuk operasi destructive lintas game)
  -> GameDiskMutationLease(game_id)
       -> filesystem mutation
       -> reconcile_with_lease(...)
       -> publish result
```

Watcher/manual reconcile hanya mengambil lease game. Trailing reconcile menerima lease yang sudah ada sehingga tidak reentrant. Event yang datang saat lease aktif masuk queue version dan dijalankan setelah mutation selesai.

### C. Durable filesystem identity dengan fallback konservatif

Untuk memenuhi auto-heal rename eksternal setelah restart/event loss, tambahkan migration aditif untuk identity folder. Rekomendasi bentuk data:

- `game_id`, `entity_kind`, `entity_id`;
- `volume_or_device_id`;
- `file_index_or_inode`;
- `creation_discriminator`;
- unique key per game/kind/filesystem identity.

Identity bersifat optional. Pada filesystem yang tidak mendukungnya, stitched event tetap digunakan. Content fingerprint boleh menjadi evidence tambahan, bukan satu-satunya bukti. Bila lebih dari satu old/new candidate cocok, jangan rewrite collection otomatis.

### C2. Startup recovery gate

Sebelum query workspace/collection dianggap current untuk game aktif:

1. load last successful reconcile generation dan filesystem identity ledger;
2. lakukan source availability check;
3. recover journal staging yang belum selesai;
4. ambil satu full coherent snapshot;
5. detect folder conflicts dan ambiguous rename candidates;
6. commit safe deltas atau return blocked/confirmation state;
7. baru tandai game `hydrated` dan izinkan collection apply/toggle/bulk action.

Jika source belum tersedia, pertahankan DB snapshot terakhir dan tampilkan `SourceUnavailable`. Jangan prune. Setelah source kembali, watcher session baru harus memicu full reconcile, bukan scoped reconcile dari event pertama saja.

### D. Explicit conflict-resolution commit

Generic reconcile tetap all-or-nothing dan blocked bila ada conflict group. Command resolver berbeda: setelah revalidasi group di bawah lease, ia boleh commit hanya group yang dipilih dengan before/after identity mapping lengkap. Commit harus mencakup stable ID dan collection dalam satu transaksi; report full berikutnya hanya memperbarui antrean.

### E. Convergence result, bukan log-only

Semua mutation mengembalikan:

- `filesystem_applied`;
- `convergence_status` (`Applied`, `Blocked`, `DeferredRetry`, `Failed`);
- `disk_reconcile_result` bila ada;
- `recovery_warning`/journal id bila rollback tidak sempurna.

Frontend tidak menampilkan sukses penuh bila status bukan `Applied` atau intentional `Blocked` yang disertai conflict manager.

### F. Collection snapshot tidak boleh dimiliki runtime row

Collection adalah durable user snapshot; row `mods`/`objects` hanyalah projection runtime yang boleh hilang saat disk berubah. Karena itu foreign key collection tidak boleh memakai cascade yang menghapus membership ketika runtime object hilang.

Model yang direkomendasikan:

- `collection_objects` memiliki logical `object_ref_key` non-null sebagai identity snapshot dan `object_id` nullable sebagai current runtime binding (`ON DELETE SET NULL`).
- `collection_mods` menyimpan `object_ref_key` non-null, `mod_path_key` non-null, sementara `object_id` dan `mod_id` nullable (`ON DELETE SET NULL`).
- Projected collection state mengelompokkan berdasarkan `object_ref_key`; runtime IDs hanya enrichment.
- Ketika folder kembali dengan logical identity yang sama, reconcile melakukan rebind IDs dan membersihkan warning missing tanpa membuat membership baru.

Alternatif yang ditolak:

1. **Hanya menghitung impact sebelum cascade.** Lebih kecil, tetapi membership tetap hilang; data user tidak recoverable.
2. **Mempertahankan object row sebagai tombstone.** Menghindari migration collection, tetapi mencampur runtime projection dengan archival snapshot dan mudah bocor ke Object List/counts.
3. **Mengandalkan `snapshot_json` saja.** Ada cache snapshot, tetapi active-game projection saat ini dibangun kembali dari member tables; dua sumber kebenaran akan drift.

### G. Rename nested diklasifikasikan dari evidence, bukan depth

Watcher harus mempertahankan raw rename pair untuk semua path di bawah mods root. Rename healer kemudian memakai DB index dan coherent destination snapshot:

1. depth 1 dengan source object yang dikenal -> object-root rename;
2. source exact mod row + destination terminal entry -> terminal mod rename pada depth berapa pun;
3. source/destination bukan terminal tetapi mempunyai descendant mods -> container subtree rename; pasangan child ditentukan dari preserved suffix dan, bila tersedia, filesystem identity;
4. move antar-object memperbarui owning `object_id` dan `object_type` setiap child;
5. bila event hilang, gunakan persisted filesystem identity; bila pasangan tidak unik, return `NeedsRenameConfirmation`.

Melonggarkan check menjadi sekadar `depth >= 2` tidak cukup: rename container dapat salah dianggap mod, sedangkan remove+add berbentuk mirip rename tidak boleh digabung tanpa evidence.

Alternatif rename yang dipertimbangkan:

1. **Evidence-based DB index + coherent snapshot + filesystem identity — direkomendasikan.** Menangani terminal/container/cross-object dan restart secara aman, dengan biaya migration identity serta test matrix lebih besar.
2. **Cukup ubah depth check menjadi `>= 2`.** Patch kecil dan menutup contoh depth 3, tetapi salah mengklasifikasikan container serta tetap gagal setelah watcher event hilang.
3. **Pasangkan remove/add memakai content/name similarity.** Tidak memerlukan migration, tetapi dapat merewrite collection ke folder yang salah ketika mod duplicate atau isi dua folder mirip; hanya layak sebagai suggestion untuk user confirmation, bukan auto-heal.

## Dependency graph

```text
T1 collection game scope
  -> T1A durable collection schema
       -> T1B missing/rebind projection
  -> T2 reconcile callers
  -> T3 mutation callers + parent prune

T4 coherent snapshot
  -> T5 game mutation lease
       -> T6 suppression recovery
       -> T7 watcher fidelity/bounds

T8 filesystem identity storage
  -> T9 offline/nested rename healing

T8 + T11
  -> T11A startup recovery gate

T4 + T5 + T1
  -> T10 atomic selected-group commit
       -> T11 durable staging recovery

T10 + T7
  -> T12 details + UI request state
  -> T13 unified conflict/path contract

T1..T13 -> T14 acceptance matrix dan full verification
```

## Task 1 — Tambahkan collection reference API yang wajib game-scoped

**Description:** Tambahkan API repo/service baru yang selalu menerima `game_id`, filter query melalui `collections.game_id`, escape prefix wildcard, dan dedupe impact. Pertahankan wrapper lama hanya sementara agar migrasi callsite dapat dilakukan bertahap tanpa mematahkan build.

**Acceptance criteria:**

- [ ] Rename/missing path game A tidak membaca atau menulis collection game B dengan relative path identik.
- [ ] Nama object berisi `%`/`_` hanya merewrite component path yang benar.
- [ ] Signature setiap affected collection dihitung tepat sekali dan path rewrites terdedupe.

**Verification:**

- [ ] Rust tests dua game dengan path sama.
- [ ] Rust tests prefix `%`, `_`, slash/backslash, dan prefix boundary.
- [ ] `cargo test collection_service::tests::references_tests`.

**Dependencies:** None  
**Files likely touched:**

- `src-tauri/src/repo/collection_repo/references.rs`
- `src-tauri/src/services/collection_service/references.rs`
- `src-tauri/src/services/collection_service/tests/references_tests.rs`
- `src-tauri/src/domain/collection.rs`

**Estimated scope:** Medium (4 files)

## Task 1A — Pisahkan collection logical identity dari runtime foreign key

**Description:** Rebuild `collection_mods` dan `collection_objects` secara additive-safe agar membership memakai logical reference key yang durable, sementara `mod_id`/`object_id` menjadi nullable bindings dengan `ON DELETE SET NULL`. Migrasi menyalin seluruh data existing dan memvalidasi jumlah/member uniqueness sebelum mengganti tabel lama.

**Acceptance criteria:**

- [ ] Menghapus runtime object/mod tidak menghapus collection membership.
- [ ] Existing collection mempertahankan jumlah member, enabled state, path, dan signature input setelah migration.
- [ ] Logical key non-null dan unik dalam collection; runtime FK boleh null dan dapat direbind.

**Verification:**

- [ ] Migration test dari fixture schema saat ini dengan beberapa collection/member.
- [ ] Foreign-key tests membuktikan object/mod delete menghasilkan `NULL`, bukan cascade delete.
- [ ] Rollback/failure test tidak meninggalkan campuran old/new table.

**Dependencies:** Task 1  
**Files likely touched:**

- `src-tauri/migrations/20260825000001_preserve_collection_members.sql`
- `src-tauri/src/domain/collection.rs`
- `src-tauri/src/repo/collection_repo/members.rs`
- `src-tauri/src/repo/collection_repo/state.rs`

**Estimated scope:** Medium (4 files; tests colocated in repository modules)

## Task 1B — Bangun missing state dan rebind dari logical collection keys

**Description:** Ubah projected-state builder dan collection service agar missing object/mod tetap tampil dari logical snapshot. Saat disk entry kembali, rebind nullable runtime IDs berdasarkan `game_id` dan normalized logical keys dalam transaksi, lalu recompute signature/snapshot sekali.

**Acceptance criteria:**

- [ ] Parent delete mempertahankan object dan semua child sebagai missing roots pada preview collection.
- [ ] Restore/recreate parent dengan identity sama merebind member tanpa duplicate.
- [ ] Apply collection melewati missing roots dengan warning terstruktur dan tidak menghapusnya.

**Verification:**

- [ ] Projected-state tests object binding null, mixed present/missing children, dan rebind.
- [ ] Collection preview/apply integration tests sebelum delete, sesudah delete, dan sesudah restore.
- [ ] Signature stabil untuk membership yang sama dan berubah hanya sesuai present/missing contract.

**Dependencies:** Task 1A  
**Files likely touched:**

- `src-tauri/src/services/projected_state_service/mod.rs`
- `src-tauri/src/services/projected_state_service/tests/projected_state_service_tests.rs`
- `src-tauri/src/services/collection_service/projection.rs`
- `src-tauri/src/services/collection_service/references.rs`
- `src-tauri/src/services/collection_service/tests/references_tests.rs`

**Estimated scope:** Medium (5 files)

## Task 2 — Migrasikan callsite collection pada Disk Reconcile

**Description:** Gunakan API game-scoped baru di mod/object pass, rename healer, dan prune. Hapus akses unscoped dari jalur reconcile.

**Acceptance criteria:**

- [ ] Mod rename, object rename, dan child missing selalu membawa `game_id`.
- [ ] Prefix-only enable/disable tetap tidak mengubah logical collection path.
- [ ] Transaction rollback mengembalikan projection dan collection bersama-sama.

**Verification:**

- [ ] Projection writer tests untuk dua game.
- [ ] Rename healer tests untuk mod/object prefix dan semantic rename.
- [ ] `cargo test services::disk_reconcile`.

**Dependencies:** Tasks 1, 1A, dan 1B  
**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/projection_writer/mods.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/objects.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/prune.rs`
- `src-tauri/src/services/disk_reconcile/rename_healer.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/tests.rs`

**Estimated scope:** Medium (5 files)

## Task 3 — Migrasikan mutation caller dan benahi parent-delete impact

**Description:** Pindahkan rename/move/trash/bulk caller ke API game-scoped. Pada parent prune, capture semua child references sebagai missing, null-kan runtime bindings melalui FK contract, persist projected collection state, lalu hapus runtime rows.

**Acceptance criteria:**

- [ ] Delete object langsung dari disk melaporkan seluruh child collection member sebagai missing.
- [ ] Bulk delete dan trash tidak menyentuh collection game lain.
- [ ] Object/child delete, nullable binding update, collection impact, dan signature update berada dalam satu transaksi atau gagal bersama.

**Verification:**

- [ ] Test direct parent delete dengan dua child di beberapa collection.
- [ ] Test cross-game bulk delete dan object rename.
- [ ] Targeted tests `mods::bulk`, `mods::trash`, dan projection prune.

**Dependencies:** Tasks 1-2, termasuk Tasks 1A dan 1B  
**Files likely touched:**

- `src-tauri/src/services/mods/bulk/delete.rs`
- `src-tauri/src/services/mods/trash/service.rs`
- `src-tauri/src/services/mods/core_ops/rename.rs`
- `src-tauri/src/services/mods/organizer_move.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/prune.rs`

**Estimated scope:** Medium (5 files)

## Checkpoint A — Collection integrity

- [ ] Semua collection reference API production wajib `game_id`.
- [ ] Runtime object/mod deletion tidak dapat menghapus membership snapshot melalui FK cascade.
- [ ] Cross-game dan wildcard tests hijau.
- [ ] Parent delete menghasilkan missing members yang dapat direbind setelah restore.
- [ ] `cargo test` tetap hijau sebelum lanjut ke concurrency.

## Task 4 — Gunakan satu full snapshot koheren per reconcile

**Description:** Ganti dua filesystem walks pada scoped reconcile dengan satu `DiskSnapshotEnvelope`; derive scoped projection dan conflict queue dari envelope yang sama. Snapshot membawa warning/partial state dan tidak boleh ditulis bila walk tidak lengkap.

**Acceptance criteria:**

- [ ] Conflict preflight dan writer melihat snapshot generation yang sama.
- [ ] Scoped reconcile tetap hanya menulis changed roots.
- [ ] Read error di tengah walk tidak berubah menjadi prune massal.

**Verification:**

- [ ] Inject mutation/read error selama snapshot dan assert tidak ada DB write.
- [ ] Test full queue dari scoped event tetap lengkap.
- [ ] Benchmark scoped event storm sebelum/sesudah untuk menjaga latency wajar.

**Dependencies:** Checkpoint A  
**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/disk_snapshot.rs`
- `src-tauri/src/services/disk_reconcile/reconcile.rs`
- `src-tauri/src/services/disk_reconcile/reconcile_tests.rs`
- `src-tauri/src/services/disk_reconcile/types.rs`

**Estimated scope:** Medium (4 files)

## Task 5 — Satukan mutation dan reconcile dalam per-game lease

**Description:** Tambahkan coordinator yang menserialisasi filesystem mutation dan projection commit untuk game yang sama. Sediakan entrypoint reconcile yang menerima guard agar trailing reconcile tidak deadlock.

**Acceptance criteria:**

- [ ] Manual/refocus/watcher reconcile menunggu mutation aktif dan kemudian memakai state final.
- [ ] Toggle/bulk/conflict trailing reconcile berjalan dalam urutan lock yang terdokumentasi.
- [ ] Event yang datang saat mutation tidak hilang; queue version diproses setelah lease.

**Verification:**

- [ ] Deterministic concurrency tests dengan barrier di tengah rename batch.
- [ ] Timeout/deadlock test untuk nested command -> reconcile.
- [ ] Orchestrator queue tests untuk event saat lease aktif.

**Dependencies:** Task 4  
**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/orchestrator/state.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/entry.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/run.rs`
- `src-tauri/src/services/fs_utils/operation_lock.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/tests.rs`

**Estimated scope:** Medium (5 files)

## Task 6 — Tegakkan suppression recovery contract

**Description:** Ubah blanket suppression menjadi lease yang mencatat dirty generation. Migrasikan operasi dengan write-set diketahui ke path-scoped suppression; operasi write-set tak diketahui wajib full reconcile sebelum sukses.

**Acceptance criteria:**

- [ ] Perubahan eksternal di root lain selama bulk delete/import tetap diproses.
- [ ] Drop blanket lease tanpa successful full reconcile meninggalkan `pending_full_reconcile=true`.
- [ ] Kegagalan trailing reconcile muncul pada mutation result, bukan hanya log.

**Verification:**

- [ ] Suppressor unit tests untuk nested guard, dirty generation, dan tail.
- [ ] Bulk delete/import integration test dengan concurrent external change.
- [ ] Failure injection trailing reconcile.

**Dependencies:** Task 5  
**Files likely touched:**

- `src-tauri/src/services/scanner/watcher/suppressor.rs`
- `src-tauri/src/services/mods/bulk/delete.rs`
- `src-tauri/src/commands/mods/mod_import_cmds.rs`
- `src-tauri/src/services/disk_reconcile/emit.rs`
- `src-tauri/src/services/scanner/tests/watcher_tests.rs`

**Estimated scope:** Medium (5 files)

## Task 7 — Perkuat fidelity dan backpressure watcher

**Description:** Pertahankan Removed/Renamed paths tanpa stat-based ambiguity, ganti unbounded channel dengan bounded coalescing, dan ubah overflow menjadi satu marker yang memaksa full reconcile. Batasi cache dedupe frontend.

**Acceptance criteria:**

- [ ] Delete/rename deep folder bertitik selalu menandai owning root.
- [ ] Event storm tidak menumbuhkan channel tanpa batas; overflow menghasilkan full reconcile.
- [ ] Toast identik tetap dedupe 3 detik, error berbeda/TTL lewat tetap tampil, map tetap bounded.

**Verification:**

- [ ] Watcher classification tests per event kind dan dotted directory.
- [ ] Stress test bounded queue/overflow marker.
- [ ] Vitest dedupe TTL, different key, cross-game, dan eviction.

**Dependencies:** Tasks 4-6  
**Files likely touched:**

- `src-tauri/src/services/scanner/watcher/mod.rs`
- `src-tauri/src/services/scanner/watcher/event_filter.rs`
- `src-tauri/src/services/scanner/watcher/lifecycle.rs`
- `src-tauri/src/services/scanner/tests/watcher_tests.rs`
- `src/features/file-watcher/hooks.ts`

**Estimated scope:** Medium (5 files)

## Checkpoint B — Concurrency dan watcher

- [ ] Tidak ada reconcile yang membaca staging app mutation.
- [ ] Setiap blanket suppression mempunyai full-recovery evidence.
- [ ] Event overflow menghasilkan full reconcile dan satu toast per dedupe window.
- [ ] Targeted Rust/TS concurrency tests hijau.

## Task 8 — Simpan identitas filesystem persisten secara optional

**Description:** Tambahkan tabel aditif untuk folder filesystem identity dan capture identity pada snapshot. Identity tidak menggantikan normalized path key; ia hanya menjadi evidence rename lintas event/restart.

**Acceptance criteria:**

- [ ] Existing DB bermigrasi tanpa rewrite data destructive.
- [ ] Supported local filesystem menghasilkan identity stabil setelah rename.
- [ ] Unsupported/zero/duplicate identity disimpan sebagai absent dan tidak menyebabkan auto-merge.

**Verification:**

- [ ] Migration up test dan clean-install schema test.
- [ ] Platform-gated rename identity test Windows; Unix equivalent bila CI mendukung.
- [ ] Duplicate/reused identity safety test.

**Dependencies:** Task 4  
**Files likely touched:**

- `src-tauri/migrations/20260825000000_filesystem_identities.sql`
- `src-tauri/src/repo/filesystem_identity_repo.rs`
- `src-tauri/src/repo/mod.rs`
- `src-tauri/src/services/disk_reconcile/disk_snapshot.rs`
- `src-tauri/src/services/disk_reconcile/types.rs`

**Estimated scope:** Medium (5 files)

## Task 9 — Heal arbitrary nested/offline rename secara konservatif

**Description:** Pertahankan raw watcher rename pair tanpa klasifikasi depth. Di rename healer, klasifikasikan berdasarkan source DB index dan destination snapshot: object root, exact terminal mod, atau container subtree. Resolve berdasarkan urutan evidence: stitched event, persisted filesystem identity, lalu ambiguous confirmation.

**Acceptance criteria:**

- [ ] Rename terminal nested, parent container, dan move antar-object memperbarui seluruh descendant stable ID/path/object link/collection secara atomik tanpa membuat row palsu untuk container.
- [ ] Rename saat aplikasi tertutup auto-heal setelah restart bila filesystem identity unik.
- [ ] Ambiguous/missing identity tidak pernah memilih pasangan otomatis dan menghasilkan status terstruktur.

**Verification:**

- [ ] Tests terminal rename depth 2/3/4, container subtree rename, cross-object move, restart, lost event, dan ambiguous candidates.
- [ ] Collection rewrite/missing assertions untuk setiap kasus.
- [ ] No SQL 1555 pada identity transition.

**Dependencies:** Tasks 1-2, 7-8  
**Files likely touched:**

- `src-tauri/src/services/disk_reconcile/watcher_batch.rs`
- `src-tauri/src/services/disk_reconcile/rename_healer.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/index.rs`
- `src-tauri/src/services/disk_reconcile/projection_writer/mods.rs`
- `src-tauri/src/services/disk_reconcile/tests/rename_healer_tests.rs`

**Estimated scope:** Medium (5 files)

## Task 9A — Gate startup/game activation pada full recovery reconcile

**Description:** Tambahkan recovery state per game agar workspace tidak memakai DB snapshot lama sebagai state current sebelum journal recovery, source check, dan full reconcile selesai. Watcher session baru/resume setelah gap harus menaikkan generation dan meminta full pass.

**Acceptance criteria:**

- [ ] Saat aplikasi dibuka setelah external create/delete/prefix toggle, workspace pertama sudah mencerminkan disk tanpa menunggu event baru.
- [ ] Saat aplikasi dibuka setelah semantic rename/move, collection auto-heal bila filesystem identity unik; ambiguity membuka confirmation dan tidak mengubah collection.
- [ ] Source yang offline saat startup tidak menyebabkan prune; ketika kembali tersedia, satu full reconcile berjalan sebelum scoped watcher events.

**Verification:**

- [ ] Restart-style integration tests: mutate fixture setelah shutdown, lalu initialize game runtime kembali.
- [ ] Matrix parent/child/nested untuk create, delete, prefix toggle, semantic rename, dan cross-object move.
- [ ] Suspend/source-unavailable/reconnect test dengan generation assertion.

**Dependencies:** Tasks 4-5, 8-9, 11  
**Files likely touched:**

- `src-tauri/src/services/recovery_service.rs`
- `src-tauri/src/services/disk_reconcile/orchestrator/state.rs`
- `src-tauri/src/commands/scanner/disk_reconcile_cmds.rs`
- `src/features/file-watcher/hooks.ts`
- `src/features/workspace-runtime/state/workspaceState.ts`

**Estimated scope:** Medium (5 files)

## Task 10 — Commit selected conflict group secara atomik

**Description:** Setelah rename/trash group direvalidasi di bawah game lease, gunakan before/after snapshot mapping untuk memperbarui seluruh row identity dan collection state group itu dalam satu transaction, walau group lain masih blocked.

**Acceptance criteria:**

- [ ] Setelah resolve group A sementara B masih konflik, row A sudah memiliki ID/path/name/status/object link final.
- [ ] Trash kandidat A mempertahankan survivor dan collection reference yang benar saat B masih blocked.
- [ ] DB failure mengembalikan filesystem atau menghasilkan recovery warning/journal; tidak ada partial commit.

**Verification:**

- [ ] Rust command/service test dua group dengan invariant query sesudah aksi pertama.
- [ ] Swap rename, one/two-name rename, parent object group, dan Trash survivor tests.
- [ ] Injected SQL failure dan rollback failure tests.

**Dependencies:** Tasks 1-5, 9  
**Files likely touched:**

- `src-tauri/src/services/mods/core_ops/folder_conflict_resolution.rs`
- `src-tauri/src/commands/mods/conflict_cmds.rs`
- `src-tauri/src/repo/mod_repo/update.rs`
- `src-tauri/src/repo/object_repo/update.rs`
- `src-tauri/src/commands/mods/tests/conflict_cmds_tests.rs`

**Estimated scope:** Medium (5 files)

## Task 11 — Tambahkan journal dan recovery two-phase rename

**Description:** Tulis journal durable sebelum staging, update phase secara idempotent, dan recover pada startup/sebelum reconcile. Recovery memprioritaskan restore old path; collision membutuhkan user action, bukan delete.

**Acceptance criteria:**

- [ ] Crash setelah staging sebagian atau apply sebagian dapat direcover deterministik.
- [ ] Folder stage tidak pernah diam-diam diabaikan selamanya.
- [ ] Recovery collision dilaporkan dengan exact paths dan tidak menghapus data.

**Verification:**

- [ ] Phase-by-phase crash simulation dan restart recovery tests.
- [ ] Existing target/case-only/swap recovery tests.
- [ ] Manual inspection Recycle Bin/Explorer hanya sebagai tambahan, bukan bukti utama.

**Dependencies:** Tasks 5, 10  
**Files likely touched:**

- `src-tauri/src/services/mods/core_ops/folder_conflict_resolution.rs`
- `src-tauri/src/services/mods/core_ops/conflict_rename_journal.rs`
- `src-tauri/src/services/mods/core_ops/mod.rs`
- `src-tauri/src/services/recovery_service.rs`
- `src-tauri/src/services/tests/recovery_service_tests.rs`

**Estimated scope:** Medium (5 files)

## Checkpoint C — Identity dan conflict transaction

- [ ] Rename nested/offline memiliki evidence yang dapat diaudit atau meminta konfirmasi.
- [ ] Resolve satu group tidak meninggalkan partial identity ketika group lain blocked.
- [ ] Crash staging memiliki recovery path idempotent.
- [ ] DB invariant query dan failure-injection suite hijau.

## Task 12 — Jadikan conflict details bounded, cancellable, dan jujur

**Description:** Pindahkan recursive scan ke blocking pool, stream preview maksimal 20 tanpa menyimpan seluruh list, laporkan partial/errors, dan ikat response ke request generation.

**Acceptance criteria:**

- [ ] UI tidak diblokir oleh folder besar dan request lama tidak menimpa group/game baru.
- [ ] Ukuran/file count menggunakan checked atau saturating arithmetic dan menandai partial result.
- [ ] Card memiliki loading, error, dan retry state; Trash confirmation tidak memakai angka 0 sebagai fallback diam-diam.

**Verification:**

- [ ] Rust tests large tree, permission/read error, symlink/reparse, cancellation, dan preview limit.
- [ ] Vitest stale response, switch game, per-card retry, dan thumbnail fallback.
- [ ] Manual responsive/keyboard check.

**Dependencies:** Task 10  
**Files likely touched:**

- `src-tauri/src/commands/mods/conflict_cmds.rs`
- `src-tauri/src/commands/mods/tests/conflict_cmds_tests.rs`
- `src/features/folder-grid/modals/FolderConflictManager.tsx`
- `src/features/folder-grid/modals/FolderConflictCandidateCard.tsx`
- `src/features/folder-grid/modals/FolderConflictManager.test.tsx`

**Estimated scope:** Medium (5 files)

## Task 13 — Satukan report state, direct conflict, dan path rewrite contract

**Description:** Tambahkan `game_id/report_id` pada dialog state, clear pending secara eksplisit saat blocked report sudah diketahui, normalisasi legacy rename conflict ke group model, dan gunakan relative path contract tunggal pada wire.

**Acceptance criteria:**

- [ ] Dialog yang terbuka tidak berpindah diam-diam ke report game lain.
- [ ] Blocked state tidak memicu reconcile loop tetapi banner tetap persisten dan report baru auto-open sekali.
- [ ] Resolver, watcher, dan legacy rename menghasilkan path rewrite selection/cache yang sama untuk parent dan nested mod.

**Verification:**

- [ ] Vitest report per game, close/reopen fingerprint, pending state, dan legacy adapter.
- [ ] Path rewrite tests absolute UI path dari relative wire payload.
- [ ] EN/ID/ZH i18n lint dan keyboard/focus tests.

**Dependencies:** Tasks 7, 10, 12  
**Files likely touched:**

- `src/features/file-watcher/hooks.ts`
- `src/features/file-watcher/reconcileSelection.ts`
- `src/features/workspace-runtime/state/workspaceState.ts`
- `src/features/folder-grid/modals/ConflictResolveDialog.tsx`
- `src/features/folder-grid/modals/FolderConflictManager.test.tsx`

**Estimated scope:** Medium (5 files)

## Task 14 — Acceptance matrix dan full verification

**Description:** Jalankan matriks end-to-end lintas app action, watcher, direct disk mutation, parent/child/nested, bulk/cancel, restart, conflict queue, Trash, dan collection. Audit semua mutation entry point yang memakai suppression dan catat contract-nya.

**Acceptance criteria:**

- [ ] Setiap skenario menghasilkan `Applied`, intentional `Blocked`, `NeedsRenameConfirmation`, atau visible `Failed`; tidak ada silent drift.
- [ ] Tidak ada toast `UNIQUE constraint failed: mods.id` atau command `not allowed`.
- [ ] Collection game lain tidak berubah; stable ID/path/status/object/collection invariant lolos setelah setiap step.

**Verification:**

- [ ] `cargo fmt --check` dan targeted Rust tests.
- [ ] `cargo test`.
- [ ] `pnpm test -- --run`.
- [ ] `pnpm lint`.
- [ ] `pnpm i18n:lint`.
- [ ] `pnpm build`.
- [ ] E2E folder conflict plus new external rename/delete/restart/bulk specs.

**Dependencies:** Tasks 1-13, including Task 9A  
**Files likely touched:**

- `tests/e2e/specs/phase3c-folder-conflicts.e2e.ts`
- `tests/e2e/specs/disk-reconcile-external-mutations.e2e.ts`
- `tests/e2e/specs/disk-reconcile-bulk-concurrency.e2e.ts`
- `tasks/folder-conflict-drift-audit-todo.md`

**Estimated scope:** Medium (4 files)

## Final acceptance matrix

| Origin           | Operation                         | Target                   | Evidence yang diharapkan                                                                                           |
| ---------------- | --------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| App              | Enable/disable                    | child mod                | Path/status/ID converged; no duplicate toast/refresh.                                                              |
| App              | Enable/disable                    | parent object            | Child paths berubah, child own-status tetap, collection logical path tetap.                                        |
| App              | Bulk toggle/cancel                | campuran parent/child    | Setiap success converged; failure eksplisit; unrelated external event tidak hilang.                                |
| App              | Rename/move                       | nested mod/cross-object  | Stable ID transition dan collection rewrite atomik.                                                                |
| Explorer         | Prefix toggle                     | child/parent             | Watcher/reconcile memahami normalized identity tanpa SQL 1555.                                                     |
| Explorer         | Semantic rename                   | nested                   | Stitched event atau filesystem identity melakukan heal; ambiguity meminta konfirmasi.                              |
| Explorer         | Delete                            | child                    | Row dipruning, collection missing + signature update.                                                              |
| Explorer         | Delete                            | parent                   | Semua child impacts dicapture sebelum cascade.                                                                     |
| Explorer         | Rename/delete                     | saat app mati            | Startup full scan memakai filesystem identity atau menghasilkan confirmation, bukan guess.                         |
| Explorer         | Create                            | saat app mati            | Startup full scan menambah row/object link sebelum workspace hydrate.                                              |
| Explorer         | Prefix toggle                     | saat app mati            | Startup full scan mempertahankan identity logical, memperbarui physical path/status, dan tidak merusak collection. |
| Explorer         | Parent delete                     | saat app mati            | Startup full scan capture seluruh child collection missing impact sebelum cascade.                                 |
| OS               | Sleep/source disconnect/reconnect | app hidup                | Tidak prune saat unavailable; watcher session generation baru memaksa full recovery pass.                          |
| Watcher          | Overflow/event storm              | semua                    | Bounded queue, full reconcile, satu toast per TTL.                                                                 |
| Conflict manager | Rename group                      | group lain masih blocked | Group terpilih sudah DB-complete; queue tersisa akurat.                                                            |
| Conflict manager | Trash candidate                   | group lain masih blocked | Survivor dan collection tetap konsisten; Trash dapat direstore.                                                    |
| Crash            | Mid-stage rename                  | 2/3+ candidate           | Journal recovery idempotent, tidak ada hidden orphan.                                                              |

## Security checklist hasil audit

- **SQL injection:** query memakai bind parameters; tidak ditemukan string-interpolated SQL injection. Ada business-logic wildcard bug pada `LIKE`, dicakup F14.
- **Command injection:** Explorer dipanggil dengan `.arg`, bukan shell string. Path tetap harus direvalidasi tepat sebelum spawn/mutation.
- **XSS:** React escaping digunakan; tidak ada raw HTML pada path/name conflict.
- **Authentication/CSRF/session:** tidak relevan untuk local Tauri IPC. Security boundary yang berlaku adalah permission allow-list dan containment pada configured mods root.
- **Authorization/IDOR:** path containment dan stale-group revalidation sudah ada; collection scoping antar-game belum aman (F01).
- **Race/TOCTOU:** mutation-vs-reconcile, stale async details, dan crash staging adalah gap utama (F02/F04/F10/F15). Revalidate canonical source/parent tepat sebelum setiap filesystem rename dan tolak escape/reparse yang berubah.
- **Cryptography:** BLAKE3 dipakai untuk deterministic ID dan UUID untuk temporary stage; tidak ada secret/credential pada flow ini.
- **Information disclosure:** absolute local paths sengaja ditampilkan pada conflict manager; tidak ditemukan credential logging.
- **DoS/resource exhaustion:** unbounded watcher channel, recursive details scan, dan frontend maps dicakup F12/F13.
- **Business logic/state machine:** partial group commit, parent prune, lost suppression, dan offline rename adalah risiko utama.

## Files reviewed

Changed feature files dibaca melalui full diff atau full source view:

- Backend contracts/commands: `src-tauri/permissions/app-commands.toml`, `src-tauri/src/commands/mods/conflict_cmds.rs`, `src-tauri/src/commands/mods/tests/conflict_cmds_tests.rs`, `src-tauri/src/lib.rs`, generated bindings registration.
- Reconcile: `disk_snapshot.rs`, `identity_conflicts.rs`, `reconcile.rs`, `reconcile_tests.rs`, `types.rs`, `emit.rs`, orchestrator `entry.rs`/`run.rs`/`state.rs`, `watcher_batch.rs`, `rename_healer.rs`, projection writer `write.rs`/`objects.rs`/`mods.rs`/`prune.rs`/`index.rs`, dan path updates.
- Mutation/repo: conflict resolution service, mod/object update repo, bulk toggle/delete, trash, operation lock, stable IDs.
- Watcher: `mod.rs`, `event_filter.rs`, `events.rs`, `lifecycle.rs`, `suppressor.rs`, dan watcher tests.
- Collection: repository references, collection service references/tests, collection impact domain.
- Frontend: watcher hooks/tests, reconcile selection, banners, old/new conflict dialogs, candidate card, validation/tests, workspace dialog state, game slice, scanner types, bindings, dan EN/ID/ZH locale additions.
- E2E: `tests/e2e/specs/phase3c-folder-conflicts.e2e.ts`.

Additional surrounding callsites dicari untuk seluruh `SuppressionGuard`, internal reconcile emitter, collection reference handler, stable ID generator, dan path update consumer.

## Limitations audit

- Audit dilakukan terhadap dirty working tree saat ini; tidak ada `master...HEAD` feature commit range yang terpisah, sehingga baseline adalah working diff terhadap `HEAD` dan dependency code yang relevan.
- Tidak ada runtime fault injection atau full test suite yang dijalankan pada fase plan; verification commands di atas wajib dijalankan saat implementasi.
- Perilaku Windows Explorer UI dan Recycle Bin restore tetap membutuhkan manual smoke check, sedangkan command invocation dan filesystem result diuji otomatis.
- Guaranteed rename healing pada filesystem yang tidak memberi stable file identity tetap tidak mungkin tanpa user confirmation; plan sengaja memilih fail-safe daripada menebak.

## Approval gate

Sebelum implementasi, keputusan yang paling material adalah menyetujui migration aditif filesystem identity. Jika migration ditolak, acceptance harus diturunkan menjadi: semantic rename eksternal auto-heal hanya ketika watcher memberi stitched `Renamed`; setelah restart/event loss, aplikasi menandai old path missing dan meminta konfirmasi untuk pasangan new path.
