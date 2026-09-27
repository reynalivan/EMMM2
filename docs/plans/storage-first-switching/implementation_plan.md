# Audit dan rencana refactor enable/disable: storage dahulu

Tanggal: 2026-09-27. Basis audit: `main`, commit `745d842`.

Status (2026-09-27): bagian 1-8 adalah audit dan rencana pada commit dasar `745d842`, sehingga rujukan baris dan deskripsi "jalur sekarang" di bawah adalah historis. Perubahan implementasi dan batas validasinya dicatat di bagian 9. Tidak ada klaim bahwa error Windows pada folder mod pengguna telah direproduksi atau target latensi native telah diukur.

## 1. Hasil yang dituju

Urutan prioritas:

1. Klik langsung mengubah tampilan switch secara optimistis dan tetap menerima klik berikutnya.
2. Backend memprioritaskan perubahan nama folder fisik. Keberhasilan rename, setelah dicatat secara durable, menjadi acknowledgement keberhasilan switch.
3. Database, status ancestor, ringkasan object, collection runtime, KeyViewer, dan tampilan lain menyusul secara asynchronous.

Filesystem tetap sumber kebenaran untuk keberadaan, lokasi, dan status folder. Optimistic UI adalah intent pengguna, bukan bukti rename sudah terjadi. Database tetap projection dan penyimpan metadata/preset, bukan sumber perintah untuk membalikkan rename yang sudah diakui berhasil.

Asynchronous berarti ada interval projection tertinggal. Target correctness adalah lag yang terukur, hasil lama tidak mengalahkan hasil baru, dan semua consumer konvergen ke disk setelah pekerjaan selesai. Tidak menjanjikan semua consumer berubah atomik pada saat klik.

## 2. Audit existing system

### 2.1 Jalur sekarang

```text
Klik grid / preview / object / bulk
  -> optimistic state lokal hook (sebagian entrypoint)
  -> antrean FIFO frontend per game
  -> game lock + initial recovery gate
  -> prepare target: lookup DB, ancestor, duplicate advisory
  -> regional preflight; dapat eskalasi ke scan lebih luas
  -> prepare ulang + validasi identity
  -> operation lock + durable journal
  -> rename folder
  -> AWAIT disk discovery + DB projection + object summaries
  -> journal DB committed / complete
  -> enqueue runtime / KeyViewer
  -> balikan command
  -> patch path frontend + berbagai query refresh
```

KeyViewer sudah asynchronous. Yang belum dipisahkan adalah keberhasilan storage dari keberhasilan projection database.

### 2.2 Temuan dan bukti

| Temuan                                                                                                                                                         | Bukti kode                                                                                                                                                                                                          | Implikasi                                                                                                                                                                                    |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Latest intent dan optimistic override disimpan per instance hook. `toggleNode`, `setNodeEnabled`, dan `setFolderPathEnabled` juga mempunyai lifecycle berbeda. | `src/features/workspace-runtime/actions/useWorkspaceSwitchActions.ts:69,80,282,316,420`                                                                                                                             | Konsistensi satu hook belum mencakup grid, preview, sidebar, dialog, dan bulk secara bersama.                                                                                                |
| Semua native switch dalam satu game masuk FIFO frontend.                                                                                                       | `src/features/workspace-runtime/actions/workspaceSwitchOps.ts:54,137,193`                                                                                                                                           | Permintaan berikutnya menunggu seluruh command sebelumnya, termasuk projection. Intent yang sudah masuk FIFO tidak bisa diganti oleh intent terbaru dari instance lain.                      |
| Switch melewati prepare, preflight, prepare ulang sebelum rename.                                                                                              | `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs:364,392,423,438`                                                                                                                                  | Pekerjaan non-storage berada sebelum operasi yang paling diprioritaskan pengguna.                                                                                                            |
| Rename berhasil masih menunggu trusted/full reconcile di bawah mutation lease.                                                                                 | `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs:568,618`                                                                                                                                          | Latensi command dan lock mencakup scan/SQL/summary. Menunda runtime saja belum menyelesaikan ini.                                                                                            |
| Kegagalan projection sesudah rename memicu rollback fisik.                                                                                                     | `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs:699`                                                                                                                                              | Saat ini DB projection masih menentukan apakah perubahan disk dipertahankan. Ini berlawanan dengan target storage-first yang baru.                                                           |
| Journal selesai hanya setelah DB committed; recovery memakai status DB untuk menentukan roll-forward/rollback.                                                 | `src-tauri/src/modules/mutation/journal.rs:523`; `src-tauri/src/modules/mutation/recovery.rs:50,123`                                                                                                                | Menghapus `await reconcile` tanpa mengubah journal/recovery berisiko membatalkan switch yang sudah diakui berhasil saat restart.                                                             |
| Duplicate notice mencari mod aktif pada `object_id` yang sama, mengecualikan satu `mod_id`.                                                                    | `src-tauri/src/modules/workspace/application/scanner/conflict/duplicates.rs:75`; `src-tauri/src/modules/library/adapters/sqlite/mods/listing.rs:539`                                                                | Ini advisory target/object yang sama, bukan hasil pembuktian benturan resource hash. Perlu dibedakan dari collision nama folder.                                                             |
| `PathBusy` berasal dari semua `PermissionDenied` tanpa hasil deteksi proses.                                                                                   | `src-tauri/src/modules/library/application/mods/core_ops/toggle.rs:137`                                                                                                                                             | Belum membedakan sharing violation sementara, permission/ACL, atau keterbatasan deteksi proses. Bukan bukti dua operasi aplikasi melakukan rename bersamaan.                                 |
| Lookup spelling stale dapat memilih sibling normalized pertama; rollback toggle tidak mengulang identity/destination check seperti apply.                      | `src-tauri/src/modules/library/application/mods/core_ops/runtime_path.rs:24`; `src-tauri/src/modules/library/application/mods/core_ops/toggle.rs:66`                                                                | Stale request/compensation perlu ownership fisik yang eksplisit agar tidak memindahkan folder pengganti. Ini risiko struktural, bukan reproduksi error screenshot.                           |
| Refresh switch mencakup workspace, folder, object, collection runtime, dashboard, keybindings, preview, conflict.                                              | `src-tauri/src/modules/workspace/application/workspace/switch.rs:1075`; `src/shared/lib/queryRefresh.ts:48,118`                                                                                                     | Sudah ada dedup dalam satu microtask, tetapi cakupan broad invalidation tetap besar dan belum terikat disk revision.                                                                         |
| Runtime mempunyai latest-generation queue, scoped cache, cancellation, dan publication guard.                                                                  | `src-tauri/src/modules/reconciliation/adapters/tauri/runtime_sync.rs:252,316`; `src-tauri/src/modules/system/application/app/post_apply.rs:1018`                                                                    | Reuse mekanisme ini; tidak perlu membangun runtime queue kedua.                                                                                                                              |
| Revision reconcile diberikan sesudah pass selesai. Runtime publication baru diinvalidate ketika job di-enqueue setelah projection.                             | `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/state.rs:963`; `src-tauri/src/modules/reconciliation/adapters/tauri/runtime_sync.rs:252`                                              | Belum ada kontrak disk revision -> projected revision -> published revision. Menunda DB akan memperbesar gap ini bila tidak diperbaiki.                                                      |
| Grid bulk adalah best-effort per item; workspace batch punya compensation jika satu langkah gagal.                                                             | `src-tauri/src/modules/library/application/mods/bulk/toggle.rs:276`; `src-tauri/src/modules/library/adapters/tauri/mod_bulk_cmds.rs:291,552`; `src-tauri/src/modules/workspace/application/workspace/switch.rs:332` | Jangan menyamakan semua bulk dengan transaksi filesystem atomik. Semantik sukses, gagal, cancel harus eksplisit.                                                                             |
| Bulk guard menolak submit selama operasi berjalan tanpa membandingkan desired state baru.                                                                      | `src/widgets/mod-explorer/hooks/useFolderGridBulk.ts:158,274`; `src/widgets/object-sidebar/hooks/useObjectBulkActions.ts:144`                                                                                       | Enable kemudian Disable saat in-flight dapat kehilangan intent Disable. Guard sekarang mencegah double-submit, belum latest-wins. Tes existing hanya memeriksa submit dengan arah yang sama. |

Yang sudah baik dan dipertahankan: identity checks, collision checks, durable journal, watcher echo tracking, scoped reconciliation, prefix-normalized stable IDs, bounded runtime workers, explorer listing revisions, dan thumbnail keys yang tidak berubah karena toggle.

### 2.3 Batas bukti audit

- Screenshot menampilkan target `AetherRedesignMerge-Toggles7-8-9` dan advisory `TravelerBoy`. Nama berbeda belum membuktikan apakah dua identity fisik berbeda, row stale, atau hubungan pack/child. Audit tidak menyimpulkan akar kasus tersebut tanpa identity/path/revision trace.
- Query duplicate sudah mengecualikan ID target. Menambah filter nama saja bukan solusi; mod bernama sama di path berbeda tetap valid.
- Tidak ditemukan bukti bahwa watcher atau cache KeyViewer sengaja menahan handle yang melarang rename. Error asli Windows harus direkam sebelum dipetakan menjadi `PathBusy`.
- Belum ada baseline native yang memisahkan waktu klik, storage commit, projection, dan runtime pada library pengguna. Angka 3–4 detik adalah laporan pengguna, belum atribusi terukur ke satu tahap.
- Sebanyak 54 tes existing pada empat file switch/bulk frontend lulus dalam dua targeted runs (44 + 10). Backend pada tes tersebut di-mock; hasil ini tidak mencakup Windows locking, crash recovery, ataupun seluruh entrypoint bersamaan.

## 3. Impact map

| Area                                                       | Fakta yang harus tersedia segera                                         | Pekerjaan setelah disk committed                                | Aturan agar tidak drift                                                                                    |
| ---------------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Grid, list, preview, context menu, sidebar                 | Intent terbaru bersama; setelah ACK, path dan state storage aktual       | Metadata dan query authoritative                                | Overlay berdasarkan identity + revision; response lama tidak menghapus intent baru.                        |
| Mod/object/pack/group folder                               | Rename parent/leaf yang benar dan identity tetap                         | Rewrite descendant paths dan summaries                          | Parent rename tidak memerlukan rename semua descendant.                                                    |
| Ancestor                                                   | Status fisik ancestor saat validasi dan local/effective state yang benar | Derive ancestor-disabled indicators dari path/hierarchy terbaru | Child local state tidak ikut dipaksa disabled hanya karena parent disabled.                                |
| DB `mods`, `objects`, filesystem identities                | Tidak wajib untuk menahan ACK normal toggle                              | Satu projection transaction, metadata tetap terjaga             | Satu writer; tidak membuat duplicate ID/row untuk dua spelling folder yang sama.                           |
| Object counts / dashboard                                  | Feedback optimistis sesuai definisi count yang ada                       | Recompute affected summaries                                    | Local enabled count dan effective-active count tidak ditukar maknanya.                                     |
| Collection runtime / Clean-Modified-Unsaved                | Dapat menampilkan data terakhir dengan status sync                       | Derive current effective set/signature                          | Prefix toggle tidak menimpa saved preset/snapshot/member intent.                                           |
| Save current / apply / restore / Safe Mode / preset hotkey | Boundary snapshot/operation yang jelas                                   | Capture/finalize berdasarkan projection konsisten               | Barrier snapshot berlaku pada operasi ini, bukan semua switch.                                             |
| Randomizer / Enable Only This                              | Set mutation yang eksplisit                                              | Projection + runtime mengikuti committed changes                | Batch compensation sebelum commit; setelah disk commit tidak dibalik karena kegagalan projection.          |
| KeyViewer / overlay / active keybindings / reload          | Older publisher segera kehilangan authority saat disk berubah            | Scoped harvest/cache update, artifact publication, reload       | Publication wajib cocok source epoch dan disk/projected revision terbaru.                                  |
| Duplicate/hash/runtime-key advisory                        | Tidak berada pada critical rename path                                   | Hitung dari snapshot coherent yang terbaru                      | Self identity dikecualikan; hasil stale dibuang; advisory tidak mematikan switch.                          |
| Folder collision dialog                                    | Target destination dan kedua physical identities                         | Refresh report scope terkait                                    | Nama sama beda parent sah; collision hanya di namespace destination terkait.                               |
| Watcher / external edit / manual repair                    | Internal rename evidence + dirty external scope                          | Coalesced targeted discovery/projection                         | Hanya exact matching echo yang diakui; perubahan eksternal nyata tidak disembunyikan oleh debounce global. |
| Explorer cursor / selection / breadcrumb                   | Committed path rewrites untuk node yang terlihat                         | Refresh affected listing snapshot                               | Selection terikat snapshot; dilarang memperluas bulk selection diam-diam.                                  |
| Thumbnail / mod health / viewer review                     | Thumbnail stable selama prefix toggle                                    | Invalidasi health/review yang terpengaruh                       | Tidak regenerate semua thumbnail atau menghapus metadata karena switch.                                    |
| Startup / crash / source-root change                       | Recovery storage journal + root identity                                 | Projection/runtime replay/rebuild                               | Tidak mengembalikan rename yang sudah acknowledged hanya karena DB belum menyusul.                         |
| Move / delete / import / rename / metadata writers         | Menghormati target ownership dan source identity                         | Memakai invalidation/projection protocol yang sama              | Fast switch tidak hidup berdampingan dengan writer lama yang tidak memahami revision.                      |

## 4. Keputusan arsitektur

### 4.1 Satu intent controller untuk semua surface

Pindahkan lifecycle intent dari local hook ke shared workspace store/service yang sudah ada. Hook menjadi adapter render dan action.

- Key intent: source/game scope + stable target identity. `mod_id`/`object_id` tetap dipakai bila tersedia, dilengkapi expected filesystem identity/path hint. Folder provisional memakai opaque target reference dari listing. Nama tampilan atau basename tidak menjadi identity.
- API dasar adalah `set desired_enabled`, bukan instruksi toggle yang harus dihitung lagi dari DB stale. Aksi click menegasikan desired state terbaru pada store bersama.
- Per target hanya ada operasi yang sudah berjalan dan satu desired state terbaru. Klik berikutnya mengganti desired state pending. Input A tidak boleh menimpa input B.
- Aturan coalescing boolean ini untuk `SetEnabled` biasa. `EnableOnlyThis`, parent activation, dan collection apply mempunyai side effect/batch policy; target yang sudah enabled belum berarti policy tersebut selesai. No-op hanya boleh diputuskan setelah seluruh policy terpenuhi atau plan memang kosong.
- Dalam satu UI session, `client_seq` dialokasikan saat klik pada shared controller, sebelum IPC/resolve selection. Backend menyimpan sequence tertinggi per producer + target; request yang datang terlambat dengan sequence lebih kecil tidak boleh menjadi pemenang. `intent_id` membuat retry idempotent. Antar-producer independen (misalnya UI dan native hotkey), coordinator menetapkan acceptance order; tidak memakai wall-clock timestamp.
- Operasi rename yang sudah memasuki syscall tidak dibatalkan di tengah. Setelah hasilnya jelas, jalankan desired state terbaru hanya jika berbeda dengan disk.
- Semua jalur masuk ikut: grid/list/preview, object, context menu, parent dialog, force/enable-only, bulk, collection/hotkey/randomizer melalui coordinator backend.
- ACK dan error membawa identity + intent + revision. Error operasi lama tidak rollback tampilan intent yang lebih baru. Event/dialog lama tidak boleh membuka ulang modal setelah state sudah berubah.

Ini menggantikan FIFO satu-command-per-click. Serialisasi singkat perubahan fisik yang saling berkaitan tetap diperlukan. Last wins berlaku per physical target, bukan satu last request global. Untuk target yang tidak mempunyai kegagalan storage nyata, setelah quiescence harus berlaku `disk_state(target) == latest_accepted_desired(target)`.

### 4.2 Storage commit boundary

```text
Click -> shared optimistic intent -> submit -> admission receipt
             |
             v
  storage coordinator: latest desired per identity
             |
  root/path/identity check + destination collision check
             |
  durable prepared journal -> guarded same-parent rename
             |
  durable DiskCommitted receipt + disk_revision
             |
             +----> storage ACK event + actual path/state to UI
             +----> release storage ownership / next intent
             +----> pending projection scope
```

Normal single-folder prefix toggle tidak menunggu duplicate advisory, recursive classification, INI parsing, collection diff, DB summary update, KeyViewer, atau full library scan.

Admission receipt hanya berarti intent tercatat dalam scheduler, bukan rename berhasil. Command submission tidak menunggu storage sehingga intent baru dapat mengganti pending target saat operasi sebelumnya masih berjalan. Storage ACK dikirim ketika durable storage boundary tercapai; event-gap/remount dapat mengambil snapshot receipt backend.

Pekerjaan wajib sebelum ACK tetap ada:

1. Validate configured Mods root/source identity, containment, symlink/reparse policy, expected folder identity, dan operasi recovery yang menyentuh target itu.
2. Resolve current physical folder memakai identity dan committed rewrite overlay. Prefix-normalized path dapat membantu lookup, tetapi tidak boleh menyatukan dua folder fisik yang kebetulan mempunyai base name sama.
3. Collision check lokal dan rename dengan semantics no-replace. Periksa lagi identity/destination sebelum retry; external process tidak menghormati lock aplikasi.
4. Durable journal intent sebelum rename, durable commit evidence sesudahnya. Reuse compact active journal; pindahkan history compaction dari jalur foreground bila measurement membuktikan perlu.

Toggle selalu same-parent rename. Tidak memakai recursive copy/delete fallback untuk operasi ini. Pekerjaan filesystem blocking dijalankan pada worker terbatas agar penerimaan intent baru tidak tertahan thread async.

Disabling/enabling sebuah parent mengubah nama parent. Semua descendant path di DB menyusul. Enabling child yang masih berada di bawah disabled ancestor tetap mengikuti konfirmasi impact existing; approved activation chain berjalan dari outer parent ke child. Konfirmasi luas ini merupakan jalur eksplisit tersendiri, bukan alasan menscan seluruh library untuk leaf toggle biasa.

### 4.3 Journal/recovery diperbaiki sebelum early ACK

Tambahkan state durable `DiskCommitted` beserta source identity, commit revision, operation/intent ID, applied step identities, dan rewrite scope. Status projection terpisah dari status disk. Gunakan journal existing; jangan menambah queue/outbox durable kedua.

- `Prepared` tanpa rename: validate lalu abandon/recover sesuai kebijakan operation.
- Crash setelah rename sebelum marker commit: inspect filesystem identities. Untuk single toggle versi baru, observed destination milik identity yang benar dapat dikonvergensikan; jangan mengirim rename balik secara buta. Atomic multi-step batch harus memastikan seluruh final identity arrangement atau compensate partial steps yang masih dimilikinya.
- `DiskCommitted`, DB pending/failed: roll forward projection dari disk terbaru, bukan rollback folder.
- Projection transaction selesai tetapi acknowledgement journal belum tersimpan: checkpoint DB membuat replay idempotent.
- Rangkaian `A -> DISABLED A -> A`: record pertama tidak dianggap rusak hanya karena destination lamanya sudah diganti oleh commit berikutnya. Recovery membaca urutan/revision dan physical identity terbaru.
- Journal pending tidak boleh dipangkas sebelum durable projection checkpoint meliputnya. Runtime dapat dibangun ulang dari DB/disk; histori setiap klik tidak perlu disimpan selamanya.
- Kegagalan persist sesudah rename dibedakan dari rename gagal. Return observed storage outcome/repair state; jangan menampilkan seolah folder pasti kembali ke state lama.
- Compensation memeriksa ulang expected physical identity dan destination vacancy; tidak memindahkan object pengganti atau overwrite folder yang dibuat proses eksternal. Failed compensation mempertahankan repair record dan hanya melindungi scope terkait.

Format journal harus versioned. Entry lama mempertahankan recovery semantics lama; jangan menafsirkan `Applied` legacy sebagai `DiskCommitted` baru. Rollback executable ke versi yang tidak memahami journal baru dilarang sampai pending journal diselesaikan atau format kompatibel sudah diverifikasi.

### 4.4 Lock dan scheduling

Pisahkan ownership mutation storage dari ownership projection/runtime. Mulai dengan satu short foreground storage writer memakai coordinator existing. Tidak perlu paralel rename pada folder yang sama untuk memperoleh respons cepat.

- Lock storage tidak mencakup disk scan rekursif, SQL writer wait, collection composition, atau runtime harvest.
- Background scan/compute bekerja di luar storage lock. Final projection commit mengambil gate singkat, mengecek version/identity, lalu menerbitkan snapshot atomik.
- Foreground intent mendapat prioritas. Background work mempunyai checkpoint/yield dan scope coalescing, sehingga pekerjaan lama tidak menahan rename berikutnya.
- Parent/child mutations diserialkan dan child path di-rebase dari committed parent rewrites. Ini berlaku juga untuk move/delete/collection apply; bukan hanya command switch.
- Physical root aliases/overlap memakai ownership yang sama meskipun game ID berbeda. Paralelisme antar-root hanya dibuka jika root benar-benar disjoint dan measurement menunjukkan manfaat; tidak perlu langsung membuat lock manager per-file.
- Initial recovery gate dipisahkan dari background indexing readiness: unresolved storage journal/source identity boleh melindungi scope terkait. Refresh runtime/DB biasa tidak membuat semua switch disabled.
- Tidak cukup mengembalikan response awal sambil task background tetap memegang lease lama. Itu mempercepat response pertama tetapi tetap menunda rename berikutnya.

Lock order implementation harus didokumentasikan: coordinator state mutex tidak ditahan ketika menunggu storage ownership, SQL, syscall, atau callback/event. Storage ownership hanya boleh mengambil state mutex secara singkat. Projection menunggu SQL writer dan mengerjakan row updates di luar storage gate, kemudian mengambil gate untuk version-check + commit; storage writer tidak boleh menunggu SQL writer sambil memegang gate. Semua legacy caller dimigrasi agar tidak menciptakan urutan lock terbalik.

### 4.5 Satu projector, revisi disk sebagai boundary

Per source epoch/root gunakan:

```text
runtime_revision <= projected_revision <= disk_revision
```

`disk_revision` adalah urutan durable storage commit, bukan counter reconcile selesai. Simpan projected watermark dalam SQLite pada transaction yang sama dengan row projection; gunakan checkpoint per game/source, additive schema bila diperlukan. Journal menyimpan commit revision agar urutan bisa dipulihkan saat restart. Epoch/root identity mencegah response dari Mods root lama dipakai untuk root baru.

- Extend existing per-game reconcile owner untuk menerima committed changes dan menggabungkan dirty scopes. Tidak membuat projector independen untuk mods, objects, collection, dan ancestor.
- Untuk trusted prefix toggle, apply delta path/status yang diketahui ke affected rows melalui writer existing. Hindari scan ulang/INI parsing untuk mempelajari rename yang baru dilakukan sendiri.
- Parent rewrite diproyeksikan berdasarkan subtree/path boundaries dan physical identities; child local enabled status tetap. Metadata, favorites, safety classification, dan saved collection intent tetap terjaga.
- Urutan intermediate rewrites tetap diketahui sampai checkpoint melewatinya. Scope perubahan A dan B digabung; latest-wins bukan berarti membuang scope A ketika job B datang.
- Projection result membawa revision yang benar-benar tercakup, bukan otomatis revision terakhir. Delta yang arrival sesudah work snapshot harus tetap pending.
- External edits/unknown identity memicu scoped discovery. Scan merekam source epoch + internal disk revision + external-change generation. Jika generation berubah sebelum commit, discard/rebase/retry; jangan commit scan lama setelah rename baru.
- Satu SQLite transaction mencakup mods/objects/path bindings/reference healing/summary/checkpoint. SQLite writer serialization saja tidak membuat filesystem snapshot lama menjadi valid.
- Urutan transaction harus konkret: tunggu `BEGIN IMMEDIATE` dan kerjakan row updates dalam transaction di luar storage gate; ambil gate hanya untuk final epoch/revision check + `COMMIT`. Jika disk/external generation sudah maju, rollback transaction dan reschedule scope. Jangan memegang storage gate sepanjang ribuan SQL updates pada parent/bulk projection. Ukur juga durasi COMMIT/fsync karena bagian final itu tetap punya biaya storage nyata.
- Jika source/watcher authority tidak dapat dipercaya, gunakan targeted validation atau recovery yang terukur. Scope lain tidak otomatis diblokir oleh satu ambiguous folder.
- Projection gagal: simpan pending durable scope, bounded retry, status sync yang dapat ditindaklanjuti. Storage committed tetap committed. Setelah storage/journal sendiri tidak bisa memastikan durability, hanya scope yang unsafe ditahan.
- Pantau ukuran/umur durable backlog. Compact histori hanya setelah checkpoint aman; jangan membuang pending commits karena batas history. Jika storage untuk mencatat intent habis, tolak sebelum rename dengan error durability yang jelas, bukan lanjut tanpa recovery evidence.

### 4.6 Collection dan runtime consumers

Prefix toggle tidak merombak saved collection snapshot. Current runtime signature berubah setelah projection; Clean/Modified/Unsaved dihitung ulang terhadap baseline yang disimpan.

Compact descriptor sudah memakai satu SQLite read transaction. Full collection runtime snapshot harus diberi snapshot/checkpoint yang sama agar beberapa query tidak mencampur revision.

Save current, capture last changes, apply/restore preset, dan Safe Mode memerlukan projection barrier untuk state yang dibaca. Mereka menunggu sampai dapat mengambil coherent snapshot pada revision yang mencakup disk state yang diperlukan. Capture dilakukan di boundary singkat; bila disk maju selama persiapan, refresh/revalidate sebelum mengklaim saved/current. Toggle lain tetap diterima dan diprioritaskan. Bila toggle terus berlangsung, operasi capture dapat tetap pending dengan status jelas, bukan menyimpan stale snapshot.

Collection apply/randomizer/batch tetap mempunyai kontrak storage mereka sendiri. Baseline/snapshot finalization tidak boleh menandai Clean terhadap revision yang sudah superseded.

Runtime/KeyViewer:

1. Invalidate older publication authority pada storage commit, bukan menunggu projection selesai.
2. Setelah projection checkpoint tersedia, enqueue queue existing dengan required disk/projected revision, affected mod IDs/root scopes.
3. Reuse scoped harvest, warm cache, cooperative cancellation, bounded workers dan retry.
4. Build artifact sementara tanpa memegang storage gate. Final publication/reload wajib memeriksa epoch dan revision secara atomik terhadap rename admission/commit. Gap check-then-publish tidak boleh membolehkan job lama menulis setelah disk baru committed.
5. Audit global publication mutex yang saat ini mencakup filesystem writes; pertahankan atomicity dengan gate sekecil mungkin. Ukur contention sebelum memperkenalkan mekanisme per-game baru.

### 4.7 UI receipt dan refresh

Perubahan kontrak typed, memakai event transport Tauri existing dan satu submission API bersama:

- Input: stable target reference, desired state, intent ID, expected source epoch/identity, policy/confirmation existing.
- Admission response: accepted/duplicate/superseded dengan accepted sequence; bukan success toast dan bukan bukti disk berubah.
- Storage receipt: operation ID, resolved intent, source epoch, disk revision, actual path/local state, compact parent rewrite bila ada, outcome applied/noop/conflict/failed/repair-needed.
- Progress terpisah: projected/runtime revision dan sync status/error. Success switch tidak menunggu progress ini selesai.
- Store merender latest intent di atas latest observed disk receipt di atas coherent query snapshot. Setiap query snapshot membawa source epoch dan observed projection checkpoint. Pada setiap surface, receipt tetap berlaku sampai snapshot yang benar-benar dirender sudah mencakup receipt itu; global event `projection completed` saja tidak cukup.
- Refresh tertunda/gagal tidak menghapus receipt overlay. Cleanup bersama hanya setelah cached baselines terkait sudah advanced atau stale snapshots di-invalidasi/evict sehingga remount tidak menampilkan state lama tanpa overlay. Satu surface yang sudah segar tidak boleh menyebabkan surface lain kembali stale.
- Optimistic rollback hanya untuk intent yang gagal dan masih current; reconcile ke observed disk state. Jangan rollback berdasarkan response lama.
- Response inactive game tetap dicatat pada game yang benar, tanpa membuka dialog atau mengubah game aktif.
- Storage ACK hanya patch affected visible nodes/path aliases. Query refresh setelah projection grouped/scoped; runtime completed hanya invalidates consumers runtime. Tidak membangunkan seluruh consumer pada setiap klik.
- Spinner tidak menggantikan switch. `aria-busy` dapat menyampaikan native operation pending; sync turunan mempunyai indikator non-blocking terpisah.
- Event gap/remount mengambil checkpoint/receipt snapshot backend. Kebenaran tidak bergantung pada listener sempat menangkap setiap event.

### 4.8 Konflik dan PathBusy

Pisahkan tiga kelas:

1. **Physical destination collision**: dua directory identities pada namespace parent/destination yang sama. Blok rename dan tampilkan conflict dialog. `X/mymods` dan `Y/mymods` sah. `X/mymods` dan `X/DISABLED mymods` tidak boleh disatukan melalui normalized identity key.
2. **Advisory semantic overlap**: target object sama, resource hash sama, atau potential runtime-key overlap. Hitung setelah storage commit pada coherent revision, exclude target physical identity dan representasi pack/child yang memang satu activation unit. Sibling berbeda yang benar-benar aktif tetap dilaporkan. Dialog informasional tidak menghalangi switch; jangan munculkan ulang hasil superseded.
3. **Rename I/O failure**: tangani OS error aslinya. Bedakan sharing/lock violation, permission denied, source missing/replaced, dan target exists. Retry hanya transient sharing/lock errors, bounded (usulan budget 200 ms), revalidate identity/collision setiap attempt, dan hentikan retry intent superseded sebelum attempt baru. Persistent failure tetap terlihat jelas.

Deteksi proses yang memakai folder bersifat best-effort. `platform/fs/locking.rs:35` mendaftarkan directory ke Restart Manager dan belum mengecek return code pemanggilan pertama `RmGetList`. Dokumentasi Microsoft menyebut registered directory dapat menghasilkan `ERROR_ACCESS_DENIED`; empty process list bukan bukti tidak ada lock. Perbaiki diagnosis ini dan simpan raw OS error (sharing violation 32, lock violation 33, access denied 5) sebelum pemetaan UI. Sumber: [RmGetList](https://learn.microsoft.com/en-us/windows/win32/api/restartmanager/nf-restartmanager-rmgetlist), [Windows error codes](https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-).

Memaksa close/kill aplikasi lain atau retry ACL error tanpa batas bukan bagian solusi. Tidak ada bukti untuk menonaktifkan watcher sebagai solusi PathBusy.

### 4.9 Bulk dan fairness

- Grid bulk mempertahankan successful/failed/skipped/cancelled per item. Selection tetap snapshot-bound; expired snapshot tidak diam-diam di-resolve ulang ke set berbeda.
- Jalankan sebagai worklist identity stabil dengan chunk storage terbatas dan durable per-item/batch receipts; UI menghitung progress dari item yang benar-benar selesai.
- Klik individual terbaru dapat supersede pending intent untuk identity yang belum dieksekusi. Worklist mencatat superseded secara eksplisit. Identity yang sudah committed mengikuti intent terbaru sebagai operasi berikutnya.
- Repeated bulk Enable/Disable tidak dibuang karena boolean in-flight. Desired state terakhir diterapkan per identity pada frozen selection: dua bulk dengan selection overlap menggabungkan target set dan supersede hanya identity yang overlap. Repeated direction yang sama idempotent. Refactor ini khusus switch; duplicate-submit protection untuk delete/move/import tetap mengikuti kontraknya.
- Fairness mengizinkan small foreground operations di antara chunk best-effort; panjang chunk ditentukan benchmark. Runtime/projection scope dari seluruh chunk tetap digabung.
- Atomic workspace/collection batches tidak boleh diam-diam diubah menjadi partial success. Setelah transaksi storage dimulai, overlapping writes menunggu boundary batch; unrelated controls tetap responsif. Compensate sebelum disk commit jika storage step gagal. Multi-file rename bukan transaksi atomik OS.
- Dokumentasi bulk saat ini perlu diselaraskan: axiom transaksi umum pada `AGENT.md` dan API best-effort tidak menjelaskan kontrak yang sama. Definisikan `best_effort` versus `atomic_batch` pada service boundary dan tests.

## 5. Tahapan implementasi

| Tahap                                 | Pekerjaan konkret                                                                                                                                                              | Kriteria keluar                                                                                                                                             |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T0: baseline dan characterization     | Trace click/intent, queue wait, lock wait, prepare, journal, rename, projection, runtime; capture Windows raw errors; harness memakai temp Mods fixtures dan DB isolated.      | Reproducer toggle cepat/native error yang relevan; latency per tahap; tidak mengandalkan mock-only sebagai bukti bug native.                                |
| T1: shared intent + typed receipt     | Shared store/controller dan backend acceptance protocol; semua surfaces menggunakan explicit desired state; command adapter tetap tipis.                                       | Cross-surface rapid toggle mengikuti intent terakhir; stale response/dialog tidak menang; satu pending intent per identity.                                 |
| T2: durable storage commit + recovery | Extend journal/checkpoint/version decoder; implement same-parent no-replace rename; raw error classification + bounded retry.                                                  | Crash/fault injection setiap boundary aman; acknowledged disk tidak rollback karena DB; replay idempotent; nama sama beda path aman.                        |
| T3: split projector ownership         | Extend reconcile owner dengan committed deltas/revisions; short storage gate; scan/SQL work di luar foreground lease; migrate semua filesystem writer yang bertabrakan.        | DB sengaja diblok/diperlambat dan beberapa toggle berikutnya tetap rename; stale scan tidak bisa commit; tidak ada legacy writer yang bypass gate.          |
| T4: consumers dan barriers            | DB ancestor/object semantics; transactional full collection snapshot; Save/Apply/Capture barriers; immediate runtime publication invalidation + revision-bound existing queue. | Semua read model konvergen; saved preset tidak ditimpa; stale KeyViewer artifact tidak publish; collection save saat toggle tidak menangkap campuran state. |
| T5: bulk + UI refresh                 | Best-effort chunk fairness, atomic-batch boundaries, revisioned optimistic overlays, scoped refresh/dedup advisories, event-gap hydration.                                     | Bulk besar tidak menghasilkan FIFO ribuan klik/refetch storm; per-item results jujur; grid/preview/sidebar konsisten.                                       |
| T6: native verification dan rollout   | Integration/stress/crash tests, Windows locking fixtures, benchmarks, typed bindings/permissions, targeted lint/typecheck/build, then package acceptance.                      | Correctness gates dan target latency terpenuhi pada fixture yang sama; dokumentasi arsitektur/req-20 diperbarui; baru siap rilis.                           |

Dependency utama: T2 dan T3 harus lengkap sebelum enable early storage ACK. T4 publication fencing dan writer migration wajib sebelum storage-first aktif. T1 saja atau task background yang tetap memegang lease lama bukan hasil akhir yang memenuhi tujuan.

Area kepemilikan implementasi:

- Frontend: `features/workspace-runtime/actions`, workspace store/optimistic effects, mod/object actions, bulk hooks, switch control.
- Storage: workspace switch application/coordinator, `library/application/mods/core_ops`, mutation journal/recovery/lock, storage error contracts.
- Projection: reconciliation orchestrator/writer/checkpoint DAL; satu owner untuk mod/object/path/summary.
- Consumers: collections runtime/capture/apply, runtime sync/post-apply, watcher event routing dan query refresh.
- Tauri boundary: domain DTO, command registration, permission allowlist, Specta bindings, command-registry tests.

Tidak perlu rewrite seluruh scanner, dependency baru, polling daemon, general-purpose saga engine, atau persistent filesystem cache kedua. Refactor memisahkan lifecycle yang saat ini berada dalam satu command panjang, sambil reuse primitives existing.

### 5.1 Perubahan teknis per modul dan goal

| File / modul                                                                                              | Perubahan yang direncanakan                                                                                                                                                                                                                       | Goal yang dapat diuji                                                                                                    |
| --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `features/workspace-runtime/actions/useWorkspaceSwitchActions.ts`                                         | Hapus ownership local `latestNodeToggleIntents`, `pendingNodeOverrides`, dan waiter-per-click. Semua `toggleNode`, `setNodeEnabled`, `setFolderPathEnabled`, parent/duplicate continuation mengirim ke shared controller.                         | Grid/preview/sidebar/remount membaca desired state sama; tidak ada request yang hilang karena hook instance berbeda.     |
| `features/workspace-runtime/state` + controller action baru yang kecil                                    | Tambahkan target-scoped intent state, admission pump, receipt reducer, snapshot checkpoint selector; reuse Zustand/store bridge.                                                                                                                  | Satu latest pending per target; update checkbox pada frame berikutnya; tidak menunggu native storage atau query refresh. |
| `actions/workspaceSwitchOps.ts`                                                                           | Switch keluar dari FIFO Promise `workspaceMutationQueues`; adapter submission typed dan event receipt menggantikan await-command-complete. Queue legacy tidak dihapus untuk operasi lain sebelum migrasinya selesai.                              | Intent Disable dapat diterima saat Enable masih in-flight dan memengaruhi next storage operation.                        |
| `widgets/mod-explorer/hooks/useFolderGridBulk.ts`; `widgets/object-sidebar/hooks/useObjectBulkActions.ts` | Route enable/disable ke shared intent controller; bypass duplicate-submit guard hanya untuk switch intents; freeze selection revision, resolve membership sekali, reuse target worklist.                                                          | Repeated bulk opposite direction menjadi last wins, bukan silent return; destructive actions tidak ikut berubah kontrak. |
| Switch control, `ObjectRowItem.tsx`, grid/preview policies                                                | Render desired/observed state dari shared selector; hilangkan loading replacement/spinner yang mengesankan semua switch terkunci; pisahkan sync indicator.                                                                                        | Checkbox selalu dapat menerima intent selama source/scope aman; seluruh surface memperlihatkan intent terbaru.           |
| `workspace/domain/workspace/switch.rs`                                                                    | Tambahkan admission/receipt types, typed target reference, producer sequence, source stamp; keep parent/resolution policies typed.                                                                                                                | Kontrak membedakan request diterima, disk berubah, projection selesai, dan runtime selesai.                              |
| `workspace/adapters/tauri/workspace_cmds.rs`                                                              | Tambahkan `submit_workspace_switch_intent` dan bounded read `get_workspace_switch_snapshot`; command memvalidasi payload lalu delegate. Existing switch/bulk callers dimigrasi ke coordinator yang sama, bukan menjalankan executor paralel lama. | IPC admission cepat; semua mutation sources berbagi scheduling/storage ownership.                                        |
| `workspace/application/workspace/switch.rs`                                                               | Pecah pure storage planning/execution dari advisory dan projection. `prepare_switch` tidak memanggil semantic duplicate classifier pada leaf fast path; parent/exclusive/batch tetap policy eksplisit.                                            | Normal toggle tidak melakukan scan/SQL/advisory work sebelum rename.                                                     |
| `library/application/mods/core_ops/{toggle,runtime_path,naming}.rs`                                       | Resolve expected filesystem identity; reject ambiguous sibling variant; same-parent no-replace primitive; bounded retry raw sharing/lock errors; identity-safe compensation.                                                                      | Tidak self-collision karena casing/alias; tidak salah folder atau overwrite; persistent errors jelas.                    |
| `platform/fs/{operation_lock,locking}.rs` dan mutation coordinator                                        | Pendekkan storage lock ownership; pindahkan background work keluar; report raw OS code; perbaiki Restart Manager return-code handling.                                                                                                            | DB/runtime lambat tidak menahan disk; `PathBusy` tidak menyamarkan semua permission failure.                             |
| `modules/mutation/{journal,recovery,coordinator}.rs`                                                      | Add versioned `DiskCommitted`, durable source revision highwater, detached projection completion, safe pending retention/replay.                                                                                                                  | Crash setelah storage ACK tidak membalik state; A->B->A recovery tetap valid.                                            |
| Reconciliation `emit`, `orchestrator`, `projection_writer`                                                | Terima ordered committed deltas; single projector; use existing scope merger; reject stale scans; update rows + checkpoint dalam transaction.                                                                                                     | DB adalah follower storage dengan eventual convergence dan tanpa lost update.                                            |
| SQLite migration + reconciliation checkpoint DAL                                                          | Add `workspace_projection_checkpoints(game_id, source_epoch, projected_revision)` dengan key `(game_id, source_epoch)`, revision nonnegative. Durable disk highwater berada di journal source metadata, bukan counter completion reconcile.       | Restart/pruning tidak mereset revision; projection replay dapat dibuktikan idempotent.                                   |
| Collections `application/runtime`, `current_state`, capture/apply boundaries                              | Full snapshot memakai satu read transaction/checkpoint; add projection barrier pada capture/apply preparation; finalization memakai captured stamp.                                                                                               | Preset tidak menyimpan campuran state dan tidak mengklaim Clean untuk revision yang sudah berubah.                       |
| `runtime_sync.rs`, `post_apply.rs`, KeyViewer publication                                                 | Invalidate publisher pada disk commit; carry required projected stamp; compose outside short gate; atomic check+publish/reload; reuse workers/cache.                                                                                              | Job lama tidak menulis artifact setelah disk revision baru; runtime lag tidak memperlambat switch.                       |
| File-watcher handler, workspace query DTOs, `queryRefresh.ts`                                             | Carry source stamp pada snapshot/event; apply receipts sebelum invalidation; per-surface overlay retention; scoped batched refresh.                                                                                                               | Out-of-order events atau refresh gagal tidak membuat switch/path kembali ke state lama.                                  |
| Generated bindings/permissions/registry tests + requirements/history                                      | Regenerate Specta, register new IPC commands/events, update req-20 dan bulk/recovery docs.                                                                                                                                                        | Tidak ada command yang hilang permission atau caller yang masih bergantung pada old completion semantics.                |

Nama tabel/API pada tabel di atas adalah proposal awal; lihat bagian 9 untuk pilihan implementasi aktual dan perbedaannya.

### 5.2 Kontrak data dan state machine

Skema konseptual yang akan diterjemahkan menjadi Rust DTO + generated TypeScript, tanpa handwritten duplicate types:

```text
SwitchIntent
  intent_id
  producer_id + client_seq
  game_id + source_epoch
  target_ref | frozen_selection_ref
  desired_enabled
  resolution + parent_confirmation + origin_surface

SwitchAdmission
  intent_id + accepted_order
  disposition: accepted | duplicate | superseded

StorageReceipt
  operation_id + source_epoch + disk_revision
  target identity + applied_intent/accepted_order
  actual_path + local_enabled
  committed_rewrites
  outcome: applied | noop | conflict | failed | repair_needed

ProjectionStamp
  game_id + source_epoch + projected_revision

RuntimeStamp
  game_id + source_epoch + required_projected_revision
  existing_runtime_generation + publication_revision
```

`target_ref` memuat stable logical ID jika tersedia dan expected physical identity; path hanya hint, tetap divalidasi backend. Native bulk selection memakai existing query/listing revision/snapshot contract. Semua payload mempunyai limit size yang eksplisit.

Success, conflict, failure, dan uncertain outcome diimplementasikan sebagai discriminated unions, bukan satu struct dengan kombinasi flag yang dapat bertentangan. `actual_path/local_enabled` hanya boleh disebut authoritative setelah diamati; error akibat source hilang tidak mengarang path/state. No-op memakai existing disk revision, tidak menaikkan revision dengan rename fiktif. Payload intent ID/sequence yang diulang dengan isi berbeda ditolak sebagai validation error.

State target:

```text
Settled -> Desired(seq=N) -> StorageApplying(N)
                               |
                     new Desired(seq=N+1) accepted
                               |
                     DiskCommitted(N), observed state updated
                               |
              desired(N+1) == observed ? acknowledge Noop : apply N+1

DiskCommitted(R) -> ProjectionPending(R) -> Projected(R) -> RuntimeSynced(R)
                         |                     |
                      Retry                Retry / Superseded
```

Pending projection/runtime bukan alasan `StorageApplying` berikutnya menunggu. Error tidak otomatis menghapus latest desired; reducer menentukan apakah failed intent masih current dan memakai actual observed disk state untuk feedback/rollback. User-visible success hanya mengikuti storage receipt yang sesuai, bukan admission.

### 5.3 Algoritme last wins dan bounded scheduling

```text
onUserSetState(target, desired):
  seq = next shared producer sequence
  sharedStore[target].desired = (seq, desired)
  render immediately
  flush latest submission without waiting for storage ACK

onBackendSubmit(request):
  validate producer/source/target or resolve frozen selection
  under coordinator state mutex, for each target:
    if request.client_seq < highest sequence for this producer + target:
      mark this target superseded
    else if same intent/sequence was already registered:
      return previous admission/outcome without a new acceptance order
    else:
      register newest desired + backend acceptance order
      if no running/queued owner: enqueue one target token
  return admission, not storage success

storageWorker(target):
  read latest desired immediately before planning
  acquire short ownership; resolve and validate current physical identity
  recheck newest intent before starting syscall / between retry attempts
  if already in desired state AND all requested policy is satisfied:
    produce revisioned Noop outcome through the same completion path
  else:
    persist prepared evidence; rename; persist DiskCommitted
    update observed identity/path/revision and invalidate old publication
  release storage ownership; schedule projection
  under coordinator state mutex, atomically:
    record outcome without regressing observed revision; retire running slot
    compare latest accepted intent with the finished intent
    if latest is newer and fully satisfied by observed state/policy:
      record Noop receipt for that latest intent
    else if latest is newer and still needs execution:
      enqueue exactly one target token
    collect receipts and wakeups to publish after unlocking
  publish receipts / wake worker
```

Worker tidak menahan mutex penerimaan intent selama filesystem syscall. Input/acceptance pump tetap hidup saat storage worker menunggu I/O. Backend menggunakan bounded blocking executor; jumlah jobs bukan jumlah klik. Completion/reschedule dan submission memakai mutex state yang sama: tidak boleh ada celah ketika submission melihat running=true tetapi worker sudah selesai memeriksa latest intent dan segera keluar. Ready token disimpan sebelum wakeup, sehingga intent tidak hilang karena timing notification.

Failed intent yang masih current settle sebagai failure setelah bounded retry; tidak otomatis dijadwalkan selamanya hanya karena desired berbeda dari disk. Newer accepted intent tetap dievaluasi terhadap physical identity/observed outcome terbaru. Projection hanya dijadwalkan untuk disk changes atau repair scope yang nyata; no-op tidak membuat full refresh fiktif.

Untuk request dari producer sama yang tiba terbalik, sequence klik menang atas arrival order. Sequence tinggi pada mod B tidak menggugurkan sequence lebih rendah pada mod A. Untuk producer independen, accepted order backend adalah tie-breaker yang terdokumentasi. No-op tetap mempunyai receipt dengan source/disk stamp aktual sehingga waiter/overlay dapat settle tanpa fake rename.

Shared frontend admission pump membatasi outstanding submit dan menggabungkan burst terbaru; tidak menyimpan array Promise/waiter untuk setiap klik. Backend state dibatasi oleh target unik dan worklist bulk yang sedang aktif. Receipt snapshot menyimpan latest outcome per identity dan bounded recent operation status; durable disk recovery tetap milik journal.

Bulk best-effort memakai round-robin/chunk yang bounded dan meninjau current desired sebelum tiap item. Atomic batch yang sudah dimulai menyelesaikan storage boundary-nya; latest intents untuk participating identities tidak hilang dan dieksekusi sesudah boundary tersebut. Tidak pernah membatalkan filesystem syscall atau separuh atomic batch lalu mengklaim success.

Pending atomic plans tetap mempunyai ownership/ordering atas seluruh participant set. Coordinator tidak boleh menjalankan plan lama setelah newer per-target operation lalu menganggap latest intent sudah selesai; setiap atomic completion menjalankan handoff/recheck untuk seluruh participant identities. Coalescing identical not-started atomic requests harus mempertahankan policy dan membership, bukan menghapus beberapa steps dari batch lalu tetap menyebutnya atomic.

### 5.4 Goal implementasi yang tidak boleh tertukar

1. **Respons input:** latest desired langsung terlihat dan dapat diganti lagi, walaupun storage/runtime belum selesai.
2. **Respons storage:** setelah mandatory safety/journal work, rename menjadi pekerjaan prioritas; delayed projection bukan bagian latency rename berikutnya.
3. **Last wins:** tidak drop opposite bulk request; tidak memproses FIFO setiap klik; tidak memakai stale frontend/DB path untuk menentukan folder yang dipindah.
4. **Storage truth:** receipt berasal dari disk outcome; DB/runtime failure tidak membalikkan acknowledged switch.
5. **No permanent drift:** semua reader/publisher mengonsumsi revision coherent dan akhirnya mengejar disk; temporary pending state boleh, silent stale overwrite tidak boleh.
6. **Bounded scale:** pending work O(unique targets + current bulk worklist), bukan O(click count); leaf fast path tidak O(total library size).
7. **Honest errors:** physical collision/permission/persistent external lock tetap dapat gagal; latest-wins bukan izin overwrite atau mengklaim disk berubah saat rename gagal.

## 6. Verification matrix

Minimal test yang harus lulus sebelum rollout:

1. 100–1.000 intent cepat pada target sama; alternating states; final disk == latest intent; memori pending tidak bertambah per klik.
2. Target sama dari grid dan preview/context menu bersamaan; termasuk remount/game switch; explicit set-state bercampur toggle.
3. Banyak target berbeda; coalescing tidak menghilangkan changed scope target lain; fairness saat bulk berjalan.
4. Parent disable/enable dengan campuran child enabled/disabled; parent-child intents bertumpuk; child local status terjaga.
5. Dua folder bernama sama pada path berbeda; same-parent enabled/disabled collision; casing, UNC, canonical aliases, source replacement dan ambiguous variant lookup.
6. Duplicate advisory diri sendiri, pack/child identity, true sibling overlap, stale enable advisory setelah disable, dan no-op enable.
7. Windows handle fixture sharing violation sementara/persisten, ACL denied, directory enumeration, source disappeared; retry bounded dan error classification benar.
8. Projection sengaja ditahan atau SQLite writer lock sibuk: ACK dan rename berikutnya tetap berjalan; retry tidak membalik disk.
9. Projection lama selesai setelah disk revision baru; reverse rename chain; replay/duplicate event; game/root generation berubah.
10. Crash sebelum/selama/sesudah rename, sebelum/sesudah DiskCommitted, sebelum/sesudah projection checkpoint, dan sebelum journal cleanup; legacy journal kompatibel.
11. Runtime job lama menyelesaikan harvest/publication sesudah rename baru; stale artifact/reload ditolak, scopes/cache tetap lengkap.
12. Save current/apply/restore/Safe Mode/preset hotkey/randomizer selama projection pending; coherent snapshot; Clean/Modified sesuai revision.
13. Native best-effort bulk cancellation, superseded item, partial failure; atomic workspace/collection batch compensation dan failed compensation recovery.
14. Import/move/delete/rename eksternal/watch overflow selama pending projection; tidak ada silent event loss atau unsafe path rebase.
15. Reopen aplikasi setelah successful switch; filesystem truth, DB, collection runtime, keyviewer dan active keybindings akhirnya cocok.
16. Projection-complete event tiba sebelum query refresh; response tertahan/gagal atau dua surface memakai snapshot berbeda. Receipt overlay tidak hilang sebelum rendered snapshot masing-masing mengejar checkpoint.
17. Inject submission tepat saat worker selesai: tidak ada accepted intent yang tertinggal tanpa worker. Enable(N) -> Disable(N+1) -> Enable(N+2) wajib menghasilkan settlement/Noop untuk N+2 meskipun disk sudah enabled oleh N.

Gunakan model-based/property tests untuk urutan intent/commit/projection/event; tambahkan native Windows integration untuk handle dan crash boundaries yang tidak bisa dibuktikan mock React.

### 6.1 Brute click dan bulky action wajib diuji khusus

| Skenario                                 | Beban fixture                                                                                          | Assertion utama                                                                                                           |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| Same-target burst dalam satu React batch | 100 dan 1.000 klik tanpa jeda                                                                          | Paritas toggle/explicit desired terakhir benar; closure React tidak kehilangan klik; satu running + satu latest pending.  |
| Sustained rapid click                    | 30 klik/detik selama 30 detik, beberapa deterministic seeds                                            | Tidak ada backlog per klik, unhandled error, self-conflict stale, atau switch yang berubah menjadi loading control.       |
| Cross-surface dan banyak target          | Grid + preview + sidebar; 100 target, 2.000 intent                                                     | State konsisten per physical identity; target A tidak membatalkan scope target B.                                         |
| Bulk Enable lalu Disable cepat           | 100 submit bergantian pada selection yang sama                                                         | Desired terakhir tidak dibuang oleh in-flight guard; jumlah native work bounded terhadap target, bukan jumlah submit.     |
| Bulk selection overlap                   | Batch A `{a,b,c}`, B `{b,c,d}`, klik individual `b`                                                    | Final state per identity mengikuti acceptance order; `a` dan `d` tetap diproses; semantik atomic batch tidak dilanggar.   |
| Large bulk + foreground clicks           | Bulk 100 / 1.000 / 10.000 selected targets dalam library hingga 100.000                                | Time-to-first-commit, per-item outcome, fairness, cancel latency, peak memory dan single-toggle p95/p99 tercatat.         |
| Bulk limit                               | 10.001 selected targets                                                                                | Batas backend existing 10.000 tetap ditolak secara eksplisit; ukuran library 100.000 bukan izin satu batch 100.000.       |
| Slow derived state                       | Tahan projector 4 detik, runtime 10 detik; inject SQLite contention                                    | Leaf storage commits tetap berjalan; query stale tidak mengubah desired terbaru; setelah release seluruh scope konvergen. |
| Native filesystem adversity              | Temporary sharing lock, real collision satu item, source replacement, external rename, cancel, restart | Tidak ada overwrite/salah folder; retry bounded; kegagalan nyata dilaporkan per kontrak; receipt/journal cocok disk.      |

Untuk brute load, hitung semua accepted/superseded/applied/noop/failed/cancelled intents, bukan hanya state akhir. Assertion final menggunakan pembacaan direktori fisik pada fixture, kemudian DB checkpoint, collection descriptor dan runtime output. Harness tidak menggunakan folder mod pengguna. Audit ini belum menjalankan stress matrix native tersebut; matrix adalah gate implementasi yang direncanakan.

## 7. Performance acceptance yang diusulkan

Ini target implementasi, bukan angka hasil audit:

| Metrik                                                     | Target pada warm local SSD tanpa external persistent lock                                                                                                                                   |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Klik -> visual optimistic                                  | Frame berikutnya, sasaran <=16 ms pada fixture normal; track p95/p99 dan long tasks.                                                                                                        |
| Intent terakhir -> storage ACK leaf toggle                 | p95 <=100 ms, p99 <=300 ms; ukur terpisah idle, saat projection lambat, dan saat bulk.                                                                                                      |
| Background projection 3–4 detik                            | Tidak menambahkan 3–4 detik ke rename foreground berikutnya. Uji deterministik dengan worker yang ditahan.                                                                                  |
| Rapid toggles satu target                                  | Satu running + satu latest pending; pekerjaan stale tidak membesar mengikuti jumlah klik.                                                                                                   |
| Leaf toggle library 100 / 1.000 / 10.000 / 100.000 entries | Tidak ada full-library traversal/INI parse/global count rebuild pada storage path; catat sibling/path depth cost yang nyata.                                                                |
| Bulk                                                       | Laporkan time-to-first-commit, throughput, p95 single-switch latency ketika bulk berjalan, peak pending/journal memory; jangan hanya total durasi.                                          |
| Convergence                                                | Setelah input berhenti dan dependency sehat, checkpoint semua consumer mencapai disk revision terakhir; no dropped scope. Durasi projection/runtime dilaporkan terpisah per ukuran fixture. |

Parent confirmation, atomic multi-folder batch, network/external disk, dan external lock diukur sebagai kategori berbeda; jangan mencampurnya untuk mengklaim semua rename selesai dalam angka leaf-toggle tersebut.

## 8. Rollout dan keputusan untuk review

Rekomendasi: setujui pemisahan storage commit dari projection sebagai perubahan kontrak utama. Ini mengubah kebijakan lama yang me-rollback disk ketika DB projection gagal.

Implementasi dapat dibuat bertahap di balik compatibility gate, tetapi jangan mengaktifkan jalur baru pada user data sebelum recovery, all-writer coordination, consumer barriers, dan publication fencing selesai. Gate hanya mengubah admission untuk operasi baru; pending receipts harus tetap diproyeksikan dan direcover dengan semantics formatnya.

Paragraf ini mencatat keputusan pada tahap audit. Status implementasi dan gate yang benar-benar dijalankan setelahnya ada di bagian 9.

Validasi audit yang dijalankan:

```text
node node_modules/vitest/vitest.mjs run
  src/features/workspace-runtime/actions/useWorkspaceSwitchActions.test.tsx
  src/features/workspace-runtime/actions/workspaceSwitchOps.test.ts
  src/widgets/mod-explorer/hooks/useFolderGridBulk.test.ts

Run 1: 3 test files passed; 44 tests passed.

node node_modules/vitest/vitest.mjs run
  src/widgets/object-sidebar/hooks/useObjectBulkActions.test.ts

Run 2: 1 test file passed; 10 tests passed.
```

Vitest lokal dijalankan langsung karena pnpm host 11.19.0 tidak sesuai pin repo 10.24.0. Tidak dilakukan upgrade dependency. Tidak menjalankan aplikasi native terhadap folder mod pengguna, stress test filesystem, atau crash injection pada tahap audit.

## 9. Status implementasi setelah audit (2026-09-27)

Implementasi memakai jalur command dan journal yang sudah ada, bukan mengganti seluruh switch engine dengan protokol receipt generik yang diusulkan pada T1. `admit_workspace_switch_intent` hanya meneruskan high-water intent agar pekerjaan lama dapat dibatalkan sebelum rename; ACK storage tetap berasal dari command switch/bulk setelah hasil disk diverifikasi. Dengan demikian, tabel tahapan dan kontrak konseptual di atas tidak boleh dibaca sebagai daftar API yang semuanya sudah dibuat.

| Area | Implementasi aktual | Bukti/gate yang masih diperlukan |
| --- | --- | --- |
| Input dan last-wins | Overlay optimistis tetap menerima klik pada switch yang sama; override target diteruskan ke backend lebih awal; grid dan object bulk menyimpan intent terakhir dan melanjutkan memakai path hasil rename. | Tes hook untuk rapid same-target, opposite bulk, dan stale path; stress native lintas surface belum dijalankan. |
| Storage | Jalur leaf normal mempersiapkan switch dari konfigurasi dan disk tanpa menunggu projection DB. Rename memeriksa identitas, konflik nama pada parent yang sama, hasil fisik akhir, dan retry Windows yang bounded. Hash/runtime overlap tidak menjadi alasan menolak rename. | Tes unit untuk collision, identity, retry, dan storage outcome; fixture Windows dengan handle eksternal/ACL belum dijalankan. |
| Durabilitas | Journal menyimpan ACK disk sebelum projection; recovery mempertahankan rename yang telah commit. Reconcile awal dan worker mencatat checkpoint revision per source epoch sebelum menyelesaikan journal; kegagalan checkpoint mempertahankan pending work. | Tes journal/recovery/checkpoint ada; fault injection proses-mati pada semua boundary belum dijalankan. |
| Projection dan konsumen | Disk reconcile berjalan menyusul, dengan checkpoint dan event; UI dapat membaca snapshot saat event terlewat. Collection capture/apply menunggu projection pending, runtime dan KeyViewer tetap di jalur asynchronous. Overlay tidak dibersihkan bila refresh gagal. | Konvergensi end-to-end pada library besar, contentions DB, dan runtime lambat belum diukur secara native. |
| Integrasi | Migration SQLite, command registration, permission, dan binding TypeScript diselaraskan; hook indexing background dipindah ke layer fitur agar aturan impor tetap valid. | Jalankan kembali seluruh lint, typecheck, test Rust/Frontend, dan production build pada diff final. |

Yang **belum terbukti** dari matriks bagian 6-7: p95/p99 klik-ke-rename pada SSD, burst 1.000 klik, bulk 10.000 item pada library 100.000, crash injection, serta Windows handle eksternal. Ini bukan hasil hijau yang dapat diklaim dari unit test atau build. Target-target itu tetap gate pengukuran native sebelum menyebut performa/ketahanan produksi terverifikasi; installer tidak termasuk permintaan implementasi ini.

Validasi kode pada diff implementasi: Vitest penuh 198 file/1.059 tes lulus (1 skipped); `cargo test --lib` lulus; ESLint, aturan arsitektur, TypeScript, Rustfmt, Clippy `-D warnings`, dan build frontend lulus. Review read-only menemukan dua celah yang kemudian diperbaiki: checkpoint pemulihan awal dan path lama pada kelanjutan bulk. Validasi ini tidak menggantikan fixture/native gate di atas.
