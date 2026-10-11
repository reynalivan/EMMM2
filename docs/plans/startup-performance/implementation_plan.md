# Plan performa startup dan penyederhanaan lifecycle

Tanggal plan: 2026-10-09. Disetujui pengguna; implementasi lokal dan validasi
terarah/native restart selesai pada 2026-10-10. Hasil pengukuran dan batas
validasi seluruh workspace dicatat terpisah di `validation.md`.
Pengguna mengizinkan refaktor arsitektur bila membuat sistem lebih sederhana,
stabil, dan mengurangi bug. Refaktor dilakukan pada tanggung jawab yang bermasalah,
tanpa membuat manager, antrean, atau sumber readiness kedua.

## 1. Keputusan utama

Rekomendasi: satu pemilik recovery per binding game/root, watcher yang hidup
selama sesi filesystem valid, dan publikasi aktivasi yang terikat generation.
Perbaiki pemakaian ulang watcher dahulu. Setelah itu, uji cache identitas
direktori melalui kontrak library yang sudah tersedia. Terakhir, buka shell dan
read model yang diberi status recovering tanpa menunggu pekerjaan disk berat.

Tidak ada satu perubahan aman yang otomatis membuat seluruh startup selesai
dalam milidetik. Registrasi native cepat, tetapi validasi perubahan offline,
klasifikasi, projection, dan recovery tetap membutuhkan kerja yang terukur.

### Bukti yang menjadi dasar

| Temuan                   | Bukti                                                                                   | Implikasi                                                                          |
| ------------------------ | --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Cache file ID mahal      | Watcher produksi 26.09–28.90 detik; kontrol NoCache 0.388–0.867 ms                      | Biaya ada sebelum timer indexing; kandidat optimasi terbesar                       |
| Watcher dibangun ulang   | Bootstrap mempromosikan watcher aktif; aktivasi game sama hanya memeriksa slot inactive | Watcher valid dibuang dan full `GameSwitched` scan diminta lagi                    |
| Scan tetap mahal         | Log aplikasi terpasang: `StartupBoot` 16.552 detik, 30,290 direktori                    | Waktu ini bukan total startup dan bukan hasil binary yang sudah diperbaiki         |
| Backfill pernah berulang | Tabel `app_meta` tidak ada; query mengabaikan kegagalan marker                          | Fix di repo menyimpan marker atomik di `app_settings`; binary terpasang masih lama |
| Index dibaca tiga kali   | 444 object/12,729 mod rows untuk 148 object/4,243 mod                                   | Backfill object memakai namespace berbeda; staging memicu reload tambahan          |

Perbaikan marker, boot sekali, dan deduplikasi `initStore` sudah ada di repo.
Audit dan benchmark historis tetap berada di `audit-20261009.md` dan
`validation.md`; tidak dianggap sebagai pengukuran setelah seluruh plan selesai.

## 2. Batas tanggung jawab

| Tanggung jawab                                  | Pemilik                                                  | Kapan berakhir                                                |
| ----------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------- |
| Subscription, cache, dan receiver filesystem    | `WatcherState` + `WatcherSession`                        | Sesi diganti, coverage hilang, channel ditutup, atau shutdown |
| Pilihan game dan hak menerbitkan status/runtime | `DiskReconcileState` + `ActivationAuthority`             | Intent/generation aktivasi disupersede                        |
| Baseline recovery                               | Gate recovery yang ada, terikat game/root yang diamati   | Hasil terminal atau binding owner disupersede                 |
| Dirty evidence dan penerimaan projection        | Authority event generation + reconcile revision yang ada | Observation terselesaikan atau harus direpair                 |
| Runtime/KeyViewer dan prewarm opsional          | Worker latest-wins yang ada                              | Generation diganti atau job selesai                           |

Aktivasi yang lama tidak boleh menghentikan receiver untuk sesi yang masih valid.
Intent aktivasi, generation aktivasi, generation recovery, watcher session, event
generation, dan reconcile revision tetap berbeda; jangan digabung menjadi satu
angka atau disimpulkan hanya dari game ID.

Refaktor batas kode secara bertahap: acceptance/recovery di application,
pemasangan/cache notify dan emisi Tauri di adapters, antarmodul melalui `api.rs`.
Ekstrak bagian yang disentuh berdasarkan tanggung jawab dan uji perilakunya;
tidak ada pemindahan seluruh modul atau pemotongan file berdasarkan jumlah baris.

## 3. Pilihan yang dibandingkan

| Pilihan                                       | Keputusan                                                      | Alasan                                                                                                               |
| --------------------------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Pakai ulang watcher aktif yang terbukti valid | Implementasi pertama                                           | Menghapus kerja berulang tanpa mengganti kontrak event                                                               |
| Cache file ID semua file seperti sekarang     | Baseline pembanding                                            | Kontrak sudah ada, tetapi biaya bootstrap terbukti besar                                                             |
| Cache ID seluruh direktori dalam scope        | Kandidat utama tahap cache                                     | Mempertahankan lineage folder dan mengurangi native-ID reads untuk asset files; perlu gate kompatibilitas file moves |
| Sekadar mengganti dengan `NoCache`            | Tidak dipilih untuk produksi sekarang                          | Pinned Windows backend tidak menyediakan tracker untuk pasangan rename; event akan kehilangan stitching              |
| Memanaskan full cache di background           | Bukan solusi performa utama                                    | Memindahkan beban tetapi tidak menghapusnya; juga menambah race/buffer backlog                                       |
| Native `ReadDirectoryChangesExW`              | Alternatif terpisah bila cache direktori tidak memenuhi target | Bisa membawa identitas event, tetapi menambah backend Win32 dan fallback filesystem/OS                               |
| Cache persisten atau NTFS USN journal         | Ditunda                                                        | Invalidation, kompatibilitas, dan recovery jauh lebih luas daripada masalah yang sudah terbukti                      |

Pemeriksaan memakai source dependency yang benar-benar dipakai: `notify 8.2.0`
dan `notify-debouncer-full 0.7.0`. Source Windows mengirim `From`/`To` tanpa
tracker; debouncer mencocokkan tracker atau file ID. Wiki upstream yang menyebut
tracker tidak dijadikan kontrak untuk versi pinned ini.

Library mendukung implementasi `FileIdCache` sendiri; lookup harus membaca cache,
bukan mencari identitas baru dari filesystem. Ini mendukung adapter kecil tanpa
fork dependency. [Kontrak FileIdCache](https://docs.rs/notify-debouncer-full/0.7.0/notify_debouncer_full/trait.FileIdCache.html)

`ReadDirectoryChangesExW` memiliki batas dukungan Windows/NTFS dan record extended
menyertakan FileId/ParentFileId. Karena implementasi baru harus menangani I/O,
cancellation, buffer parsing dan fallback, alternatif ini belum menjadi pilihan
awal yang paling sederhana. [API Microsoft](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-readdirectorychangesexw),
[record extended](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-file_notify_extended_information)

## 4. Tahapan implementasi

### Tahap 0 — Baseline yang dapat dipercaya

- Bekukan identitas build, dependency, kondisi hardware dan dataset pengukuran.
  Bedakan restart proses dengan OS cold cache; jangan membandingkan debug dengan
  release atau mengosongkan cache/data pengguna untuk memperoleh angka bagus.
- Pisahkan waktu settings/DB recovery, pemasangan OS watcher, pengisian ID cache,
  census, klasifikasi, projection, runtime opsional, shell paint, dan Core Ready.
- Hitung install watcher per binding, full scan per owner recovery, index load
  passes, native-ID reads direktori/file, entries cache, overflow dan repair.
- Gunakan log lokal dengan correlation ID/generation dan counts, tanpa path
  pengguna, nama mod, credentials atau telemetry/network tambahan.
- Baseline utama harus binary dari source yang sudah memiliki fix marker. Aplikasi
  lama tetap bukti historis, bukan pembanding setelah fix. Build/installer disiapkan
  sebelum tindakan instalasi yang memerlukan otorisasi pengguna.

**Gate:** minimal tiga restart yang tidak mengubah library; hasil dapat ditautkan
ke binary yang sama. Scan tambahan akibat perubahan nyata dicatat terpisah.

### Tahap 1 — Reuse watcher dan pisahkan umur receiver dari aktivasi

1. Pertahankan urutan intent, activation guard, pengecekan intent terkini,
   validasi target, generation aktivasi dan persist/ACK settings yang ada.
2. Sebelum reset recovery atau invalidasi watcher, coba reuse hanya untuk game
   yang sama dan watcher aktif yang benar-benar terpasang.
3. Gunakan helper acceptance yang sudah ada:
   `try_clean_activation_recovery_result` → `clean_cached_activation_result` →
   `activation_result_is_accepted`. Pertahankan operation guard nonblocking sampai
   publikasi; revalidasi root fisik, runtime-config coverage, latest applied
   revision, event generation, pending commit dan suppressor repair.
4. Publikasikan melalui `publish_activation_outcome` dengan activation generation
   baru dan recovery generation yang sudah selesai. Pertahankan ACK settings,
   onboarding settlement, runtime enqueue dan prewarm yang sudah terkoordinasi.
5. Jangan memanggil `start_watcher_for_activation` untuk reuse current watcher:
   entrypoint itu masih mengganti watcher dan membuat receiver loop baru.
6. Pada receiver yang dipertahankan, aktivasi lama yang `Superseded` hanya
   melepaskan tugas publikasi/barrier timer miliknya. Loop tetap mengamati sesi
   yang valid; berhenti hanya ketika sesi/coverage/channel memang diganti.
7. Jika proof tidak tersedia, tahap pertama mempertahankan jalur konservatif
   yang ada. Dirty evidence tidak dihapus untuk memaksa Ready.

Isolated/foreign-root journal `NeedsRepair` tidak disamakan dengan ordinary
pending disk commit. Pertahankan isolasi repair yang ada; jangan menambahkan
collection-snapshot barrier ke aktivasi dan membuat root baru terkunci selamanya.

**Gate:** startup library tidak berubah memakai satu watcher selected-game dan
satu baseline full scan; ACK game sama tidak meminta `GameSwitched` scan atau
mengganti session/receiver. Callback yang terlambat tidak mematikan receiver atau
menyatakan generation lama Ready.

### Tahap 2 — Kurangi ID reads melalui cache direktori

Implementasikan satu cache milik watcher melalui `FileIdCache`, menggunakan
factory dan tipe `ModWatcher` yang ada. Hindari framework pilihan backend,
cache global baru, persistence, atau antrean repair kedua.

Kontrak cache:

- Seed identitas **seluruh direktori** yang memang tercakup. Gunakan tipe entry,
  bukan nama, ekstensi atau depth; direktori bertitik tetap direktori.
- Asset files tetap teramati oleh native watcher. Tidak membaca native ID setiap
  asset file bukan berarti memfilter event atau melewatkan validasi INI/metadata.
- Cache lookup tidak melakukan I/O. Jangan mencari identitas source historis
  yang sudah dipindah atau canonicalize path yang sudah hilang.
- Directory rename, moved-in subtree, create/delete/recreate dan perubahan
  ancestor harus memelihara identitas/path descendants dengan proof native.
  Enumeration/identity failure atau race mempertahankan repair evidence.
- Tidak ada eviction untuk identitas direktori yang masih menjadi coverage
  trusted. Batasi lewat lifetime/scope watcher; jangan menurunkan bukti dengan LRU.
- Pertahankan lexical Windows namespace dan physical spelling. Jangan memakai
  normalisasi basename atau alias `DISABLED` sebagai identitas cache.
- Runtime-config parent watch dan `d3dx.ini` tetap tercakup. Link/junction tidak
  boleh memperluas scope atau diam-diam menghilangkan direktori yang diwajibkan.

**Perubahan kontrak yang harus dibuktikan:** file yang tidak dicache dapat
menghasilkan `Removed` + `Created`, bukan satu `Renamed`. Classifier saat ini
menyimpan kedua sisi struktural dan observer berjalan sebelum filter. Namun
stitched cross-root rename sekarang memaksa full scan; dua observasi terpisah
dapat menghasilkan scoped reconcile. Tes harus membuktikan DB/runtime fresh di
kedua root, termasuk counterpart yang terlambat. Bila bukti closure tidak cukup,
gunakan conservative repair; tidak menebak pasangan dari nama atau waktu.

Kenaikan jumlah event harus diuji terhadap buffer 4,096 event dan event yang
tiba selama seed. Overflow/`need_rescan` membatalkan kepercayaan yang relevan dan
menjalankan recovery yang ada. Microsoft juga menetapkan enumeration ulang saat
notification buffer tidak dapat melaporkan seluruh perubahan.
[Semantik buffer Windows](https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-readdirectorychangesw)

Kerja finite yang blocking tidak dijalankan pada executor async. Jika dipindah
ke `spawn_blocking`, hasil berupa resource owned yang hanya dipublikasikan sesudah
intent/session/root diperiksa lagi. Job yang disupersede dibatalkan kooperatif;
`abort` tidak menghentikan pekerjaan blocking yang sudah mulai. Resource yang
tidak diterima ditutup dan tidak pernah menjadi watcher aktif.
[Kontrak Tokio](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)

**Gate:** seluruh kontrak correctness lulus, tidak ada asset-file ID prewarm,
memori mengikuti direktori/scope hidup, dan startup/event throughput membaik
melampaui variasi pengukuran. Target awal watcher setup p95 ≤5 detik pada dataset
referensi adalah target eksperimen, belum hasil atau jaminan. Jika gagal, jangan
ship perubahan cache hanya karena kode sudah dibuat; nilai alternatif ExW secara
terpisah dengan bukti kebutuhan.

### Tahap 3 — Shell cepat dan satu recovery owner untuk pending activation

Tahap ini baru dilakukan setelah lifetime/reuse/cache memiliki coverage tes.

- Jadikan pembacaan settings snapshot cepat melalui revisioned `ConfigService`
  read; jangan menunggu activation guard yang sedang mengerjakan cache/scan.
- Perluas same-root activation untuk **join** owner startup yang masih pending,
  bukan reset gate atau scan lagi. Binding owner harus sesuai config/root dan
  generation terkini; game ID yang sama saja tidak cukup. Waiter publikasi lama
  berakhir pada supersession tanpa mengambil ownership recovery.
- Gunakan receipt/status yang ada untuk pilihan game dan fase Syncing; saved
  selection provisional tidak boleh sendiri menerbitkan Ready atau mengaktifkan
  workspace mutation. Listener dipasang sebelum request, dan status snapshot
  menutup race event yang sudah lewat.
- Shell, settings dan read model yang didukung dapat tampil dengan status
  recovering. Core belum valid tetap menolak operasi disk melalui preflight
  backend. Setelah Core Ready, kontrak disk-first untuk ordinary dirty metadata
  tetap seperti sekarang; jangan mengunci switch pada runtime/projection opsional.
- Completion baseline memicu refresh yang sesuai **meskipun changed flags kosong**.
  Cancel/reject hasil query pre-baseline yang terlambat, lalu refetch untuk owner
  yang masih current. `invalidateQueries` saja tidak cukup untuk cold in-flight
  query yang belum punya data.
- Initial route, selected-root change, recovery failure, source unavailable dan
  retry menggunakan status yang ada. UI error/retry/i18n tetap jelas; tidak ada
  spinner tanpa status terminal atau loop polling tanpa batas.

**Gate:** shell paint p95 ≤2.5 detik pada mesin/dataset referensi dan tetap
responsif selama scan; ini target penerimaan. Core Ready diukur terpisah, setiap
root hanya mempunyai satu recovery owner, dan data lama tidak menang setelah
completion/supersession. Tidak ada klaim Core Ready ≤2.5 detik.

### Tahap 4 — Selesaikan biaya projection yang terbukti

- Samakan object backfill dengan key namespace normal object writer; jangan
  mengubah namespace mod atau bump marker yang memaksa seluruh backfill tanpa
  kebutuhan. Data lama tetap diperbaiki oleh reconcile yang valid.
- Pertahankan index sesudah object staging bila mod staging tidak mengubahnya,
  sehingga tidak ada pembacaan identical index lagi. Tetap reload ketika row
  identity memang berubah; uji native swaps dan collection references.
- Warm full reconcile tanpa perubahan tidak boleh melakukan identity staging
  atau mengubah nilai key yang sudah benar. Hitung query passes dan row writes,
  tidak hanya wall time. Ordinary DAL updates masih dapat mengeluarkan SQL UPDATE;
  conditional no-op updates baru dioptimasi terpisah bila biaya terukur dan
  kebutuhan timestamp/observation tetap terpenuhi.
- Staging GC opsional baru dipindah dari setup bila timing menunjukkan biaya.
  Journal/mutation recovery tetap memenuhi guard sebelum mutation tersedia.

**Gate:** no-change reconcile tidak melakukan identity staging/key-value changes,
normal index load satu pass, dan swap/recovery/collection coherence tidak regresi.
Gate ini tidak menjanjikan nol SQL UPDATE atau melewatkan runtime refresh yang
diwajibkan. NoCache atau persistent classifier cache tidak ditambahkan sebagai
jalan pintas.

## 5. Tes yang menentukan boleh tidaknya perubahan dipakai

| Kelompok            | Skenario minimum                                                                                  | Bukti wajib                                                                           |
| ------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Reuse/lifetime      | Bootstrap→aktivasi sama; loop lama masih initializing/retrying; event sebelum/saat reuse          | Receiver hidup, session tetap, satu scan, status/runtime hanya generation current     |
| Authority           | Pending commit, busy lease, ordinary dirt, suppressor repair, overflow                            | Tidak false Ready; scoped/full repair benar; foreground tidak menunggu lease opsional |
| Root binding        | A→B→A, root replacement pada path sama, runtime path berubah, identity unreadable                 | Tidak menerima proof lama; failed/source-unavailable tetap eksplisit                  |
| Rename cache        | Prefix reverse chain, external rename, ancestor/subtree rename, dotted dirs, deletion/recreation  | Native ownership dan alias/path rewrite benar; tidak ada cache identity palsu         |
| File observations   | INI/asset move antardua object roots, relevance berubah, half missing/late                        | Projection dan runtime freshness kedua root; ambiguity repair bila belum proven       |
| Events saat seed    | Create subtree lalu rename; temp replacement; event burst melebihi buffer                         | Tidak kehilangan observation atau mengakui snapshot yang lebih tua                    |
| Runtime config/link | `d3dx.ini` rename/replacement di luar Mods; junction rebinding                                    | Coverage, containment dan repair konsisten                                            |
| UI hydration        | StrictMode, pending owner join, event completion sebelum listener, no-change baseline, late query | Satu owner, hasil stale ditolak, snapshot/error/retry tidak hilang                    |
| Durability          | Settings persistence gagal, isolated foreign-root repair, crash/restart                           | Restoration/journal evidence dipertahankan; tidak rollback acknowledged disk commit   |

Gunakan test-first untuk regresi yang dapat direproduksi. Assert state dan efek
nyata, bukan hanya bahwa event diterima. Tes native yang mengubah folder memakai
fixture terisolasi dan identifier E2E yang diverifikasi. Library asli hanya untuk
benchmark read-only dengan izin scope yang sudah ada.

## 6. Pengukuran, urutan rilis, dan rollback

- Ukur tiap tahap sendiri pada build flavor/dataset/kondisi yang sama; minimal
  tiga app-cold restarts, serta warm runs terpisah. OS-cold perlu label dan
  prosedur tersendiri. Jangan menjumlahkan sampel watcher dan scan berbeda
  menjadi angka end-to-end yang tidak pernah diukur.
- Simpan median/range serta counts yang deterministik; tambah jumlah sampel
  untuk p95 yang bermakna. Target watcher/shell belum tersertifikasi oleh audit.
- Urutan: baseline → reuse/lifetime → cache experiment → pending owner/shell →
  projection. Tidak mengganti cache dan ownership sekaligus saat mencari regresi.
- Jalankan suite terkait, TypeScript/build, lint/architecture, Rust tests,
  Clippy/formatting dan review. Kegagalan modul lain dilaporkan dan diselesaikan
  sebelum mengklaim seluruh release green; tidak memakai type/lint bypass.
- Tidak ada dependency, persistent readiness cache, schema migration, background
  daemon, updater action atau worktree baru yang diwajibkan plan ini.
- Rollback per tahap ke binary compatible yang masih menyimpan fix marker dan
  membaca journal/schema saat ini. Jangan menghapus DB, journal atau repair
  evidence. Installer/instalasi merupakan langkah eksplisit setelah artifact
  build/validasi tersedia, bukan efek samping sesi perencanaan.

## 7. Area kode yang disentuh saat implementasi

| Area                                        | Perubahan terarah                                                               |
| ------------------------------------------- | ------------------------------------------------------------------------------- |
| Settings activation adapter                 | Admission dan reuse/join sebelum reset; ACK revision tetap otoritatif           |
| Workspace watcher lifecycle/factory/cache   | Lifetime receiver, penerimaan current watcher, adapter cache direktori          |
| Reconciliation state/API/bootstrap          | Ownership pending, binding/generation, phase timing, acceptance/repair yang ada |
| Object backfill/projection writer           | Namespace object dan penggunaan ulang index snapshot yang sesuai                |
| App entrypoint/store/settings queries       | Shell hydration, phase/read-only policy, query completion boundary              |
| Watcher/recovery/UI tests dan audit harness | Regression matrix, counts dan benchmark setiap tahap                            |

Lokasi tetap di vertical slice yang memiliki tanggung jawabnya. Nama/file baru
ditetapkan dari struktur aktual saat implementasi; jangan membuat lapisan global
`services`, event bus atau duplicate readiness store.

## 8. Lanjutan pengurangan pekerjaan rutin (disetujui 2026-10-10)

Pengguna menyetujui prioritas 1–2 dari audit lanjutan:

- Hapus recursive size walk, query path khusus sizing, dan size write dari
  reconcile/startup/watcher. Nilai lama tidak ditimpa; mod baru memakai default
  schema. Storage Optimizer tetap mengukur file secara eksplisit. Duplicate
  waste dashboard menggunakan ukuran report optimizer dengan binding root
  dan pemeriksaan anggota terindeks yang masih hidup.
- Pisahkan cleanup import staging: sebelum IPC menerima import baru, validasi
  referensi DB dan detach garbage milik aplikasi dengan rename atomik tanpa
  overwrite ke `import-staging/.cleanup`. Sesudah setup selesai, event native
  `Ready` menjalankan penghapusan rekursif di blocking pool. Worker hanya melihat
  quarantine sehingga ekstraksi baru yang belum tercatat tidak terhapus.
  Sisa quarantine akibat shutdown dicoba lagi pada peluncuran berikutnya.

Recovery journal/import tetap berjalan sebelum mutation tersedia. Detach masih
memerlukan pembacaan layout UUID dan rename singkat saat setup; penghapusan
rekursif payload yang dipindahkan keluar dari jalur itu. Native `Ready` adalah
batas event loop setelah setup, bukan sertifikasi bahwa frame pertama sudah
tergambar. Prioritas 3–5 belum termasuk implementasi lanjutan ini.

## 9. Lanjutan pekerjaan opsional (prioritas 3 dan 5)

Disetujui pengguna pada 2026-10-10:

- Keybinding Home menunggu activation Ready dari sumber status workspace yang
  sudah ada. Query menggunakan game dan generation aktivasi agar hasil lama
  tidak tampil setelah pergantian atau recovery baru. Skeleton hanya berada di
  widget keybinding; Home lain tetap dapat digunakan. Error/source unavailable
  menghentikan skeleton dan menampilkan pesan. Mode demo hanya membaca fixture.
- Pembersihan task log settled dipindahkan dari setup `block_on` ke worker
  startup yang sudah ada, setelah baseline game aktif selesai atau gagal.
  Retention tujuh hari dan perlindungan task Pending/Running tidak berubah.
  Recovery task/import/journal tetap dilakukan sebelum mutation tersedia.

Tidak ada daemon, readiness manager, IPC tambahan atau perubahan schema.
Sesuai arahan terakhir pengguna, verifikasi akhir memakai E2E aplikasi native;
server demo browser yang sempat dipakai dihentikan. Prioritas 4 belum diubah.

## 10. Cleanup arsitektur dan risiko thumbnail (disetujui 2026-10-10)

- Satukan predicate path efektif aktif di domain normalizer Workspace melalui
  facade `api.rs`; pertahankan semantik separator, prefix legacy dan status DB.
- Perjelas metrik initial index dan metadata yang termasuk mod pass; tidak
  menambah guard SQL, cache atau instrumentation framework.
- Perjelas entry point bootstrap guard untuk identitas native E2E dan audit
  read-only tanpa memecahnya menjadi service/state baru.
- Reproduksi invalidation thumbnail antar-folder dengan fixture deterministik.
  Perbaikan hanya jika dapat mempertahankan reset/hide/same-folder safety dan
  batas memory/retry. Verifikasi targeted, strict Clippy dan native Windows
  dengan build baru; library GIMI tidak digunakan atau diubah.

Selesai: 1.635 tes Rust lulus, 21 ignored, strict all-target Clippy/formatting
lulus, serta lima kasus native pada dua spec dengan build baru lulus. Detail
regresi junction, batas verifikasi dan artifact tercatat di
[`history cleanup`](../../history/202610100012-simplify-startup-and-thumbnail-invalidation.md).

## 11. Onboarding dan first indexing (disetujui 2026-10-10)

- Tolak aktivasi terminal Failed segera, tetap izinkan SourceUnavailable yang
  punya worker recovery; readiness tetap membutuhkan revision disk sah.
- Lindungi sesi foreground yang masih dimiliki dari TTL orphan dengan ownership
  scoped dan cleanup pada success/error/drop; explicit cancel tetap berlaku.
- Bind progress onboarding ke reason dan run yang aktif, cegah event terlambat,
  phase/count mundur, serta callback setelah cleanup/retry. Listener harus siap
  sebelum IPC indexing dimulai dan failure listener menjadi error yang jelas.
- Laporkan folder classified/root berjalan selama root besar diproses, memakai
  throttle yang ada. Tetap bedakan completed roots dan unknown total work.
- Pindahkan watcher-before-scan ke blocking preparation yang sama. Ukur wall
  registration/discovery/preparation/apply/handoff; bedakan summed root work.
- Benchmark batch 1/2/4/8 pada fixture balanced dan skewed dengan parity output,
  pilih perubahan hanya jika manfaat konsisten. Jangan memakai data GIMI.
- Native E2E harus melalui onboarding/Finish Setup dengan folder nonempty dan
  picker Windows asli, kemudian memeriksa progress, kesiapan Home, background
  game, restart dan cleanup. Tidak bypass lewat save+refresh atau mock dialog.

Tidak ada daemon, cache persisten, dependency baru, perubahan schema, instalasi
atau modifikasi config/library production.

Selesai: 1.674 tes Rust dan 58 tes frontend terkait lulus; strict all-target
Clippy, format/type/lint checks lulus. Dua kasus native dengan picker Windows
asli memverifikasi 6.000 mod, game background 80 mod dan restart; concurrency
tetap 8 berdasarkan 40 trial dengan parity. Batas pengukuran dan artifact:
[`onboarding first index`](onboarding-first-index-20261010.md).

## 12. Efisiensi indexing dan finishing (diminta 2026-10-10)

Permintaan pengguna mencakup audit, benchmark dan implementasi perbaikan lokal.

- Ukur first-index dan sync ulang pada fixture yang sama; bedakan discovery,
  metadata, SQL projection, read-model, handoff dan pekerjaan runtime background.
- Uji batching INSERT mod baru dengan identitas langsung dalam transaction
  yang sama. Pertahankan existing-row identity, koleksi, metadata, safety,
  normalisasi Unicode, rollback dan conflict protection. Hindari guard SQL
  unchanged yang sebelumnya tidak menunjukkan manfaat.
- Pangkas traversal importer pada configured Mods sebelum descent, dengan
  containment canonical; konfigurasi runtime/Core dan invalidasi tetap diperiksa.
- Gunakan progress berdasarkan unit aktual setiap tahap: root preparation,
  mod projection dan finishing tanpa persen/ETA palsu. Hapus helper weighted
  size-work yang sudah tidak digunakan product setelah mengonfirmasi callers.
- Pertahankan perubahan hanya bila pengukuran repeat menunjukkan manfaat;
  verifikasi regression, lint/type/Clippy serta native Windows pada build baru.

Tidak menambah dependency, daemon, cache persisten, schema atau concurrency.
Data nyata GIMI dan profil production tetap di luar pengujian.

Native pertama membuktikan Home 7,36 s masih didominasi discovery/classification
4,61 s pada satu root berisi 6.000 mod; projection sudah 0,45 s. Uji lanjutan
mendistribusikan budget Rayon onboarding yang sama (maksimal 8) ke cabang
independen saat jumlah root sedikit. Jangan menambah pool/concurrency; lanjut
hanya dengan parity boundary/census/identity, cancellation/error safety dan
pengukuran control width 1 melawan bounded width 2/4/8.

Selesai: controlled single-root discovery 603,6 → 279,1 ms (53,8%); INSERT
berpasangan 224,6 → 30,8 ms (86,3%). Native final 6.000 mod mencapai Home
3,92 s vs observasi sebelumnya 7,36 s; discovery 1,21 s vs 4,61 s. Batas
single debug observation/cache tercatat di
[`laporan efisiensi`](indexing-efficiency-20261010.md). Full Rust 1.723 lulus,
27 ignored; FE 53 lulus; strict Clippy/format/type/lint dan 3 kasus native lulus.
Profil QA terisolasi dan fixture dibersihkan sesudah owned shutdown; GIMI tidak
diakses. Normal sync classification dan lock publikasi KeyViewer menjadi kandidat
audit berikutnya, dengan correctness guards tetap dipertahankan.
