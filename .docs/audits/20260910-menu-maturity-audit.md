# Audit kematangan menu dan dead code

Tanggal: 10 September 2026. Cakupan: working tree lokal, termasuk perubahan yang belum di-commit. Ada perubahan paralel selama audit; referensi merupakan kondisi saat pemeriksaan. Tidak mengubah implementasi atau data aplikasi. Berkas ini merupakan hasil audit.

## Status perbaikan â€” 11 September 2026

Urutan perbaikan mengikuti permintaan: dead code dan sisa import lama, lalu F4/F9, F5/F6, F7/F8, dan F1/F2/F3.

- D1/D2/D3: berkas adapter Rust serta re-export TypeScript yang tidak terjangkau dihapus setelah migrasi referensi tes. Store dan jalur Browser Import lama, termasuk command `browser_clear_imported`, dibersihkan; status `imported` tetap dibaca untuk kompatibilitas data lama.
- F4/F9: Ignore baru menutup grup duplikat setelah semua pasangan berhasil, seleksi partial dipertahankan, dan progres scan disimpan lintas navigasi.
- F5/F6: Randomizer mengecualikan leaf di dalam parent disabled dan mengisolasi state untuk game/sesi yang aktif.
- F7/F8: watcher Mod Inbox mengikat root yang tepat saat restart; semua aksi Play memakai helper yang sama untuk pending, error, dan auto-close.
- F1/F2/F3: event progres tidak lagi menimpa status terminal; Homepage memakai draft yang dapat diketik; retention sekarang tersimpan dan dibaca dari SQLite melalui command Tauri, dengan migrasi sekali dari preference browser lama.

## Penilaian dan prioritas

1. Browser/Downloads dan Settings → Browser: paling banyak bukti wiring terputus, state salah, dan sisa workflow.
2. Storage Optimizer: kontrak Ignore antarlapis tidak konsisten dan lifecycle scan belum utuh.
3. Randomizer: state lintas game/sesi dan aktivasi mod di parent disabled tidak konsisten.
4. Mod Inbox: watcher tidak mengikuti perubahan lokasi inbox.
5. Dashboard Quick Play: jalur launch berbeda dari tombol utama.

Collections relatif lebih lengkap dalam alur yang diperiksa: preview, missing mods, partial apply, runtime changes, dan recovery sudah terhubung. Ini bukan sertifikasi seluruh Collections bebas bug. Tidak menyimpulkan semua menu atau semua Settings setengah matang.

P1 berarti perlu diprioritaskan karena merusak alur utama; P2 berarti bug fungsi yang terlokalisasi; P3 berarti residu pemeliharaan.

## Temuan fitur

### F1 — P1 — Status unduhan terminal ditimpa cache progres

- Bukti: `src/pages/browser/hooks/useDownloads.ts:20–28`, `:41–42`, `:80`, `:121`.
- Setelah satu event progres, `progressByDownloadId` menyimpan progres tanpa dibersihkan ketika unduhan selesai, gagal, atau dibatalkan. `applyLatestProgress` selalu menimpa status menjadi `in_progress`, termasuk pada hasil refetch backend yang sudah terminal.
- Dampak: daftar kembali menampilkan Downloading, active-count salah, aksi yang bergantung status terminal tidak tersedia sampai cache modul hilang saat reload aplikasi.
- Verifikasi: fungsi aktual diekstrak dan ditranspilasi dengan TypeScript lalu dieksekusi di Node VM tanpa menulis file. Input status `finished`, `failed`, dan `canceled` dengan cache progres semuanya menghasilkan `in_progress`.
- Perbaikan: status backend tetap otoritatif; batasi overlay progres pada status aktif dan bersihkan cache saat terminal/delete. Uji urutan progress → terminal event → refetch, termasuk event terlambat.
- Gap tes: `DownloadManagerPanel.test.tsx` memock seluruh hook sehingga tidak menguji transisi tersebut.

### F2 — P2 — Homepage browser tidak bisa diketik

- Bukti: `src/pages/settings/components/tabs/BrowserTab.tsx:63–68`.
- Input controlled memakai `value={homepageUrl}` dari query dengan `onChange` kosong. Ketikan dikembalikan ke nilai query; blur/Enter kemudian menyimpan nilai yang tidak berubah. Setter backend tersedia.
- Perbaikan: state draft lokal, validasi, dan satu jalur penyimpanan. Uji mengetik, blur/Enter, serta gagal menyimpan.

### F3 — P2 — Pengaturan retention tidak mengatur cleanup backend

- Bukti: `BrowserTab.tsx:111`, `src/entities/browser/model/useBrowserStore.ts:118–128`, `src-tauri/src/modules/browser/application/browser/download_service.rs:193–199`.
- UI menyimpan `retentionDays` ke Zustand/localStorage. Cleanup membaca `browser_settings.retention_days` dari SQLite dengan fallback 30 hari. Tidak ditemukan jalur produksi yang menyimpan nilai UI tersebut ke SQLite.
- Dampak: memilih 1 atau 365 hari tidak menghasilkan kebijakan yang dipilih; riwayat dapat dibersihkan lebih cepat dari ekspektasi pengguna. Temuan ini tentang riwayat download, bukan klaim file unduhan ikut terhapus.
- Perbaikan: satu sumber setting backend dengan getter/setter atau payload cleanup yang eksplisit. Test backend saat ini menulis SQLite langsung sehingga tidak menguji wiring UI.

### F4 — P1 — Ignore grup duplikat tiga anggota atau lebih hanya menyelesaikan pasangan pertama

- Bukti: `src/features/scanner/utils/resolutionRequests.ts:51–58`; `src-tauri/src/modules/duplicates/adapters/tauri/tauri.rs:267`; `src-tauri/src/modules/duplicates/application/dedup/resolver.rs:302`, `:325`, `:342`; `src-tauri/src/modules/duplicates/adapters/sqlite/dedup/mod.rs:79`, `:110`.
- Skenario: satu grup A/B/C, pilih Ignore lalu Apply. Frontend menghasilkan (A,B), (A,C), (B,C) dengan groupId sama. Batch berisi hanya Ignore masuk `resolve_batch`, yang mengotorisasi setiap pasangan sebagai grup pending. Pasangan pertama mengubah grup menjadi ignored; pasangan berikutnya gagal, lalu grup menjadi partial.
- Dampak: hanya satu pasangan masuk whitelist. Grup hilang dari laporan yang memfilter pending; pasangan lain dapat muncul lagi pada scan berikutnya.
- UI `src/features/scanner/components/DuplicateReport.tsx:65–68` juga menghapus seluruh seleksi dan memberi toast sukses tanpa membaca `summary.failed`. Hook memiliki warning partial, tetapi pesan sukses dan hilangnya seleksi tetap menyesatkan.
- Perbaikan: otorisasi dan penyelesaian satu grup secara utuh, ubah status setelah semua pasangan selesai; tampilkan hasil partial dan pertahankan pekerjaan yang gagal.
- Gap tes: frontend menguji pembentukan all-pairs; tes backend Ignore yang ditemukan memakai pasangan tunggal. Skenario di atas ditelusuri secara statis, belum dieksekusi terhadap native backend.

### F5 — P2 — Randomizer menawarkan mod di parent disabled tetapi Apply bisa tidak mengaktifkannya

- Bukti: `src-tauri/src/modules/library/application/mods/metadata.rs:162`; `src/features/randomizer/RandomizerModal.tsx:112–141`; `src-tauri/src/modules/workspace/application/workspace/switch.rs:272`; `src-tauri/src/modules/library/application/mods/core_ops/toggle.rs:45–48`.
- Skenario: `DISABLED Character/Variant`, dengan leaf Variant tidak berprefix disabled. Service memasukkan kandidat karena ancestor disabled. Apply menargetkan leaf, sedangkan toggle memeriksa nama leaf saja dan mengembalikan no-op. Parent tetap disabled; modal dapat tertutup tanpa menjelaskan bahwa mod belum efektif aktif.
- Perbaikan: definisikan aktivasi parent dengan validasi dampaknya, atau keluarkan kandidat ancestor-disabled sampai alur itu didukung. Periksa effective-enabled sebelum menganggap Apply selesai.
- Gap tes: tes kandidat efektif-disabled tersedia; belum mencakup penerapan akhirnya.

### F6 — P2 — Proposal Randomizer terbawa antar-game dan sesi buka

- Bukti: `src/features/randomizer/RandomizerModal.tsx:25`, `:77`, `:114`; pemanggil di `src/widgets/launch-bar/LaunchBar.tsx` memasang modal terus-menerus tanpa key per game.
- Skenario: roll game A → tutup → ganti game B → buka. Proposal tidak direset; guard proposal nonkosong mencegah autoroll. Apply menggabungkan path lama dengan gameId baru.
- Dampak: pratinjau salah; path absolut biasanya ditolak guard. Jika path relatif cocok di game B, target bisa berbeda dari proposal yang dilihat. Tidak mengeksekusi skenario mutasi lintas game saat audit.
- Perbaikan: isolasi state berdasarkan game/sesi, reset proposal/seleksi/error, dan abaikan respons async dari game/sesi lama. Tambah pengujian close/reopen serta pergantian game.

### F7 — P2 — Mengganti lokasi Mod Inbox tidak memindahkan watcher

- Bukti: `src/pages/mod-inbox/ModInboxPage.tsx:79–92`, `:194–208`; `src-tauri/src/modules/ingestion/application/import_batch/mod_inbox_watcher.rs:56–93`.
- Skenario: inbox lama dan baru sama-sama ada, game sama, gunakan Choose Location. Snapshot berganti rootPath tetapi dependency effect hanya gameId, rootState, dan teks prefix error. Effect tidak restart watcher ketika kedua lokasi sama-sama ready.
- Backend watcher terikat ke canonical root pada start; `save_settings` yang aktif tidak melakukan restart watcher inbox.
- Dampak: daftar baru termuat sekali, tetapi file masuk ke lokasi baru tidak otomatis muncul sampai refresh manual atau menu dipasang ulang.
- Perbaikan: sertakan identitas root dalam lifecycle watcher dan lindungi start/stop async dari balapan. Tes yang ada meliputi mount/unmount, stale refresh, dan missing → ready; belum ready-root A → ready-root B.

### F8 — P2 — Quick Play berbeda dari Play utama

- Bukti: `src/pages/dashboard/components/DashboardQuickActions.tsx:35–38` dibanding `src/widgets/launch-bar/LaunchBar.tsx:61–78`, `:122`.
- Quick Play hanya memanggil launchGame dan `catch(console.error)`. Play utama memiliki pending state, menampilkan error, serta menjalankan exitApp ketika auto-close aktif.
- Dampak: Quick Play mengabaikan auto-close, gagal tanpa pesan UI, dan tetap menerima klik selama launch berjalan. Backend launch tidak menyediakan mekanisme auto-close pengganti.
- Perbaikan: gunakan satu aksi launch bersama dengan pending/error/autoclose. Tes QuickActions yang ada hanya memverifikasi navigasi Mod Inbox.

### F9 — P2 — Scan Storage Optimizer tidak dapat dilanjutkan pemantauannya setelah navigasi

- Bukti: `src/features/scanner/StorageOptimizerPage.tsx:26`, `src/widgets/app-shell/AppShell.tsx:61`, `src-tauri/src/modules/duplicates/adapters/tauri/tauri.rs:39`, `:89`, `:148`.
- Scan berjalan dalam task backend, sementara progress berada di state halaman. Keluar menu melepas halaman; masuk lagi menampilkan Start tanpa progress/Stop scan yang masih berjalan. Start baru ditolak karena scan sebelumnya masih berjalan.
- Backend mengirim event Progress setelah proses scan selesai, sehingga adanya progress bar belum menjamin progres selama komputasi panjang.
- Perbaikan: status task yang bisa diambil ulang, subscription yang bisa disambung, serta progres inkremental dari scanner. Temuan statis, tanpa benchmark native.

## Dead code dan sisa refactor

### D1 — P3 — Tujuh berkas backend tidak dideklarasikan sebagai modul

`adapters/tauri/mod.rs` pada tujuh domain berikut hanya mendeklarasikan command files baru. Berkas `adapters/tauri/tauri.rs` lama tetap ada, tetapi tidak dideklarasikan, tidak dimasukkan melalui include/path, dan registry aktif memakai command files penggantinya:

| Domain | Berkas lama | Perkiraan baris |
| --- | --- | ---: |
| automation | `src-tauri/src/modules/automation/adapters/tauri/tauri.rs` | 56 |
| catalog | `src-tauri/src/modules/catalog/adapters/tauri/tauri.rs` | 242 |
| dashboard | `src-tauri/src/modules/dashboard/adapters/tauri/tauri.rs` | 33 |
| games | `src-tauri/src/modules/games/adapters/tauri/tauri.rs` | 301 |
| library | `src-tauri/src/modules/library/adapters/tauri/tauri.rs` | 492 |
| system | `src-tauri/src/modules/system/adapters/tauri/tauri.rs` | 355 |
| workspace | `src-tauri/src/modules/workspace/adapters/tauri/tauri.rs` | 482 |

Total sekitar 1.960 baris termasuk komentar/baris kosong. Bukti dead code adalah reachability modul, bukan anggapan bahwa seluruh fungsi bernama sama tidak terpakai. Berkas command pengganti tetap aktif. Risiko praktis: memperbaiki versi lama tidak mengubah aplikasi; code search juga bisa mengarahkan ke implementasi usang.

### D2 — P3 — Sebelas compatibility re-export tidak terjangkau entrypoint produksi

Graph import/export TypeScript statis dari `src/app/entrypoint/main.tsx` atas 399 source files menghasilkan 14 file tidak terjangkau. Tiga merupakan infrastruktur tes yang sah; sebelas berikut adalah compatibility re-export:

- `src/app/providers/queryClient.ts`
- `src/pages/collections/hooks/collectionReferenceImpact.ts`
- `src/pages/dashboard/hooks/useThumbnail.ts`
- `src/pages/settings/hooks/settingsQuery.ts`
- `src/pages/settings/model/settings.ts`
- `src/shared/ui/components/ui/Toast.tsx`
- `src/widgets/mod-explorer/hooks/folderMutationPayloads.ts`
- `src/widgets/mod-explorer/hooks/useFolderCoreMutations.ts`
- `src/widgets/object-sidebar/modals/MoveToObjectDialog.tsx`
- `src/widgets/object-sidebar/modals/MoveToObjectDialogPanels.tsx`
- `src/widgets/object-sidebar/services/objectService.ts`

Beberapa masih dipakai tes atau menjadi target mock usang. Jangan menghapus semuanya secara mekanis sebelum migrasi referensi tes. Ini residu struktur, bukan sebelas fitur besar yang mati. Pencarian tambahan tidak menemukan `import.meta.glob` yang mengubah kesimpulan entrypoint ini.

### D3 — P2/P3 — Residu workflow import di Browser

- `src/entities/browser/model/useBrowserStore.ts`: selectedDownloadIds dan selection actions hanya dipakai store/tes; autoImport, skipGamePicker, allowedExtensions, downloadsRoot dan setter-nya tidak memiliki konsumen produksi yang ditemukan.
- `src-tauri/src/modules/browser/adapters/sqlite/browser/downloads.rs:228`, `:246`: get_finished_for_import dan mark_imported tidak punya pemanggil produksi yang ditemukan. `import_jobs.rs:20` insert_job juga tidak punya pemanggil.
- Clear Imported masih ditampilkan pada DownloadManagerPanel dan DownloadsPage, tetapi tidak ditemukan workflow produksi pembentuk status imported untuk unduhan baru. Data lama mungkin masih memakai status tersebut.
- Rekomendasi: pastikan arah produk, lalu bersihkan state/DAL/UI usang atau lengkapkan workflow yang memang masih diperlukan. Audit ini tidak merekomendasikan menghidupkan auto-import tanpa keputusan produk.

## Verifikasi dan batas audit

- Lulus: `node node_modules/typescript/bin/tsc --noEmit --incremental false`.
- Berhasil: traversal import/export TypeScript dan pemeriksaan deklarasi modul Rust secara read-only; reproduksi fungsi cache progres F1.
- Tidak berjalan: Knip gagal karena file lokal `zod/v4/core/json-schema.js` hilang. Pemanggilan pnpm sempat meminta sinkronisasi instalasi dan berhenti; tidak memaksa install/purge.
- Tes terarah DashboardQuickActions dan ModInboxPage tidak dieksekusi: worker Vitest gagal memuat `expect-type/dist/branding`. Tidak ada klaim suite lulus; proses kemudian dihentikan.
- Tidak dilakukan: build native Rust, lint penuh, native E2E, operasi pada mod pengguna, pengujian performa. Audit ini berfokus kematangan menu dan wiring, bukan pentest atau sertifikasi keamanan.
- Checklist keamanan skill: injection, XSS, authentication, authorization/IDOR, CSRF, session, cryptography, information disclosure, DoS tidak diverifikasi menyeluruh; tidak ada klaim bersih. Authorization target/path ditelusuri pada temuan Randomizer/Ignore; race/state ditinjau pada proposal, watcher, download, dan lifecycle scan; business logic menghasilkan temuan di atas. Auth/session/CSRF web tidak diasumsikan berlaku sama pada aplikasi desktop lokal.

## Catatan cakupan pembacaan

Berkas berikut dibaca penuh melalui source langsung atau hasil CodeGraph (pengelompokan memakai direktori untuk singkatnya):

- `src/pages/dashboard/`: Dashboard.tsx; hooks/useDashboardStats.ts, useActiveKeybindings.ts; components/DashboardQuickActions.tsx dan tesnya.
- `src/widgets/launch-bar/LaunchBar.tsx`, `src/app/entrypoint/main.tsx`, `src/app/store/appStore/navigationSlice.ts`, `src/widgets/object-sidebar/components/ObjectContextMenu.tsx`; sebelas compatibility re-export pada D2.
- `src/pages/mod-inbox/`: api.ts; ModInboxPage.tsx dibaca seluruh fungsi komponennya, dengan import dan bagian tes diperiksa terpisah. ModInboxPage.test.tsx hanya sebagian, bukan review penuh.
- `src/pages/settings/`: SettingsPage.tsx; components/tabs/BrowserTab.tsx, AITab.tsx, PrivacyTab.tsx, HotkeyTab.tsx.
- `src/pages/browser/`: types.ts, downloadStatusBadge.ts, components/DownloadsPage.tsx, DownloadManagerPanel.tsx dan tesnya, BrowserPage.tsx, hooks/useDownloads.ts, useWebviewSync.ts.
- `src/entities/browser/model/useBrowserStore.ts`.
- `src/features/randomizer/RandomizerModal.tsx` dan tesnya.
- `src/features/scanner/`: StorageOptimizerPage.tsx, hooks/useDedup.ts, components/DedupFeature.tsx, DuplicateReport.tsx, IgnoredPairsModal.tsx; utils/dedupProgress.ts, resolutionRequests.ts dan tesnya.
- `src/features/conflict-report/ConflictModal.tsx`; `src/pages/collections/CollectionsPage.tsx`, components/SaveCollectionModal.tsx, ApplyCollectionModal.tsx, RecoveryDialog.tsx, CollectionContextControls.tsx, hooks/useCollectionRuntime.ts; AppShell.tsx.
- Backend browser: adapters/tauri/tauri.rs, application/browser/download_service.rs, application/browser/browser_service/settings.rs, application/browser/tests/download_service_tests.rs, adapters/sqlite/browser/settings.rs dan mod.rs.
- Backend workspace: domain/workspace/switch.rs, application/workspace/switch.rs, adapters/tauri/workspace_cmds.rs. Backend library: application/mods/metadata.rs, bulk/toggle.rs, core_ops/toggle.rs, adapters/sqlite/mods/listing.rs.
- Backend duplicates: adapters/tauri/tauri.rs, application/dedup/resolver.rs, scanner.rs, adapters/sqlite/dedup/mod.rs. Backend ingestion: application/import_batch/mod_inbox_watcher.rs. Backend dashboard: adapters/sqlite/dashboard/mod.rs.

Generated bindings, App.tsx, backend settings save service/command, games launch command, browser downloads/import_jobs/download_handler, conflict_cmds, dan sejumlah tes lain hanya dibaca bagian yang relevan. Modul lama D1 diperiksa reachability dan deklarasinya; isi seluruh tujuh berkas tidak diaudit perilakunya. Kesimpulan tidak meluas ke kode yang tidak diperiksa.
