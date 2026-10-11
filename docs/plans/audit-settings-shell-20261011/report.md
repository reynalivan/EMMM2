# Audit Settings, app shell dan telemetry

11 Oktober 2026. Audit source/caller, empat repro unit, SDK docs/source dan review
independen menemukan lima defect. Tidak ada implementasi perbaikan pada pass ini.
Daftar fitur dan gap lengkap tersedia di [coverage](coverage.md).

## Temuan terkonfirmasi

1. **P2 — Telemetry otomatis melewati sanitasi.**
   [telemetry.ts:26](E:/code/projects/EMMM2/src/shared/lib/telemetry.ts:26)
   mengembalikan semua payload non-EVENT tanpa redaksi. `getWebInstrumentations`
   Faro 2.11.0 memasang error handlers yang mengirim pesan/stack asli; jalur
   `reportFrontendError` yang hanya mengirim kode tidak melindungi jalur tersebut.
   Trigger memerlukan endpoint terkonfigurasi, consent aktif dan sampled session.
   Ini bertentangan dengan pengecualian pesan/path/nama/URL pada informasi privasi.
   Perubahan terkecil: daftar instrumentation/payload eksplisit, menolak atau
   menyanitasi automatic exception sebelum transport. SDK terpasang dan
   [source resmi error instrumentation](https://github.com/grafana/faro-web-sdk/blob/main/packages/web-sdk/src/instrumentations/errors/instrumentation.ts)
   telah diperiksa; tidak ada outbound reproduction.

2. **P2 — Queue diagnostics bisa terisi kembali setelah opt-out.**
   [settings_cmds.rs:509](E:/code/projects/EMMM2/src-tauri/src/modules/settings/adapters/tauri/settings_cmds.rs:509)
   melakukan purge lalu menyimpan consent=false. Bulk/onboarding yang sudah
   dimulai menyimpan consent/sink lama; completion masih dapat enqueue dan worker
   menulis rollup tanpa consent fence. Export tetap diblokir saat disabled, tetapi
   data ini bisa terpilih setelah opt-in berikutnya. Perubahan terkecil: fence
   producer/persistence dan serialize revocation/purge terhadap writer, dengan
   tes completion tertahan melewati opt-out. Dikonfirmasi lewat source, belum
   melalui tes concurrency Rust/native.

3. **P2 — Aksi compact Play hilang setelah Dashboard → Mods.**
   [LaunchBar.tsx:38](E:/code/projects/EMMM2/src/widgets/launch-bar/LaunchBar.tsx:38)
   mengambil target portal sekali, sedangkan target hanya dibuat di Mods. Komponen
   LaunchBar tetap mounted antar kedua view. Target menjadi null atau detached;
   pada lebar compact, aksi desktop tersembunyi dan menu tidak berisi Play,
   Randomizer maupun conflict review. Repro memakai komponen aktual dan gagal
   karena target ada tetapi child Play null. Ikat target ke lifecycle node yang
   sebenarnya; cek kedua arah navigasi tanpa DOM polling.

4. **P2 — Completion save game lama menutup draft baru.**
   [GameFormModal.tsx:154](E:/code/projects/EMMM2/src/pages/settings/modals/GameFormModal.tsx:154)
   selalu memanggil onClose setelah await. Dialog tidak memblok cancel saat
   pending. Save A → Escape → edit B → completion A menutup draft B. Dua repro
   menunjukkan cancel tidak dicegah dan completion lama memanggil onClose setelah
   B dibuka; tidak membuktikan korupsi data backend. Block cancel saat pending dan
   fence completion/error ke sesi asal. Perilaku default close sesuai
   [kontrak dialog HTML](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-dialog-element);
   Escape native belum diuji pada pass ini.

5. **P3 — Resize meninggalkan style global saat unmount.**
   [ResizableWorkspace.tsx:93](E:/code/projects/EMMM2/src/widgets/app-shell/ResizableWorkspace.tsx:93)
   melepaskan mouseup listener tanpa memulihkan cursor/userSelect. Bila runtime
   loading atau navigasi mengganti Mods saat drag, seluruh halaman tetap memakai
   col-resize dan text selection nonaktif. Repro unmount mengamati kedua nilai
   tertinggal. Pulihkan style sebelumnya pada akhir/cancel/unmount gesture.

## Cleanup yang terverifikasi dari caller

| Kandidat                                                                                                          | Bukti dan perubahan terkecil                                                                                                                                        |
| ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [is_conflict_ignored](E:/code/projects/EMMM2/src-tauri/src/modules/workspace/adapters/sqlite/conflict/mod.rs:167) | Hanya definisi, tidak ada caller source/test. Hapus fungsi lookup; tabel, migrasi, list/set/ignore/revoke yang aktif tetap diperlukan.                              |
| [classify_folder_strict](E:/code/projects/EMMM2/src-tauri/src/modules/workspace/domain/classifier.rs:424)         | Hanya satu caller tes parity. Runtime memakai strict scan + classify_folder_strict_from_scan. Pindahkan assertion ke kontrak jalur aktif sebelum menghapus wrapper. |
| [TopBarActionsPortal shim](E:/code/projects/EMMM2/src/widgets/top-bar/TopBarActionsPortal.tsx:1)                  | Satu baris re-export; barrel dapat langsung mengekspor shared component tanpa mengubah API konsumen.                                                                |
| [useFolderMutations shim](E:/code/projects/EMMM2/src/widgets/mod-explorer/hooks/useFolderMutations.ts:1)          | Satu baris re-export useActiveConflicts. Pindahkan satu runtime import dan tiga mock ke API feature, lalu hapus shim.                                               |

Shim masih dipakai; ini simplifikasi wiring, bukan bukti komponen di dalamnya
unused. Dependency telemetry masih memiliki caller aktif, sehingga belum aman
dihapus langsung. Daftar warning build juga bukan daftar unused universal:
fail_interrupted_downloads dan dua helper staging dipakai jalur bootstrap pada
konfigurasi build yang relevan.

## Kandidat yang memerlukan bukti tambahan

- Thumbnail age/cleanup: accessed() dapat lebih tua daripada modified(), dan
  resolver L2 tidak mencatat akses eksplisit. Cleanup tidak menggunakan generation
  resolver. Perlu fixture timestamp/concurrency untuk membuktikan missing-path
  publication; belum ditetapkan sebagai defect runtime.
- Reset: code menghapus app_data/thumbnails, sementara cache aktif berada di
  app_data/cache/thumbnails. Ada cleanup tidak efektif, tetapi tidak ditemukan
  runtime failure. Retensi browser history/settings membutuhkan keputusan scope
  reset; tidak boleh diubah menjadi penghapusan data luas secara otomatis.
- get_logs limit/count belum dibatasi, tetapi caller UI menggunakan 300 dan logfile
  dirotasi. Tidak ada external reachability/business-impact repro untuk dugaan
  allocation abort, sehingga bukan temuan security terkonfirmasi.

## Verifikasi dan penyangkalan

- Isolated Vitest: empat tes, empat assertion RED, 1,51 detik. Satu run awal gagal
  karena relative alias audit salah; diperbaiki sebelum assertions berjalan.
  Run awal bukan bukti defect. Tidak ada source produksi yang dipatch untuk repro.
- Source/Skeptic/Referee mengonfirmasi kelima finding; canonical schemas lulus.
  Snapshot primary 24 input zero drift; backend post-read 32 input cocok.
- Revision guard mencegah whole-settings overwrite; backend mempertahankan
  runtime Safe Mode walau PrivacyTab mengirim safety.keywords saja.
- Log path emmm.log cocok dengan plugin 2.9.1; tidak terjadi .log.log. Maintenance
  menjaga root milik app dan menolak symlink/directory; reset menyimpan backup
  WAL-consistent sebelum SQL clear dan mengubah memory setelah DB berhasil.
- Tidak ada Cargo/build/native/driver, actual telemetry send, installed-app/library
  mutation, dependency change, commit atau push dari audit ini. Root tetap source
  freeze untuk capture task performance. Ini satu pass terarah dengan gap terbuka,
  bukan audit lengkap seluruh repo atau sertifikasi UI native.
