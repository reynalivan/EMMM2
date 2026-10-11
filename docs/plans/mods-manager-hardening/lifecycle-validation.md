# Mods lifecycle: hasil akhir

Sembilan temuan dalam scope Windows sudah diperbaiki dan diverifikasi.
Satu keterbatasan Linux tetap **manual review**; laporan ini tidak menyatakan
repository bebas dari semua bug atau memberi jaminan source-identity lintas platform.

Working tree bersama dipertahankan pada `main`, base commit
`2079d4ab944a7b51914973c1135f83003be03873`. Tidak ada stash, pergantian branch,
worktree, commit/push, pemasangan aplikasi produksi, atau mutasi library pengguna.
Migrasi SQLite additif telah disetujui pengguna dan diuji pada database fixture.

[Laporan mesin yang lolos schema](E:/code/projects/EMMM2/.bug-hunter/mods-lifecycle-fix/fix-report.json)
dan [snapshot 25 file](E:/code/projects/EMMM2/.bug-hunter/mods-lifecycle-fix/validation-complete-source.json)
menyimpan scope serta provenance. Hash scope tetap sama sesudah native, full Rust,
dan strict Clippy. Setiap follow-up memakai baseline/manifest baru; bukti awal
tidak ditimpa. Batas batch produksi tidak terlampaui; verifikasi dilakukan terpisah.

## Fix yang selesai

| ID       | Perubahan dan bukti                                                                                                                                                                     |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MM-L2    | Target organizer terikat namespace Mods; junction luar root ditolak dan tidak ditampilkan sebagai target. Unit serta native IPC lulus.                                                  |
| MM-L3    | Root asli dicapture sebelum await dan disimpan dalam journal. Root pengganti/legacy ambigu tidak di-rollback otomatis.                                                                  |
| MM-L4    | Navigasi hanya mengakses item dengan indeks valid. Regresi grid kosong/halaman terakhir serta native keyboard lulus.                                                                    |
| MM-L5    | Receipt operation/game/root masuk dalam transaksi SQL finalisasi collection/task. Child process keluar mendadak sebelum marker journal; reopen mempertahankan asset dan task Completed. |
| MM-L2-R1 | Folder tujuan yang belum ada dibuat per komponen melalui parent pin, tanpa mengadopsi directory/junction yang muncul setelah prepare. Retained pin dipakai saat publication.            |
| MM-L3-R1 | Retry, rollback, object/mod publication, trusted projection dan finalisasi mempertahankan identitas awal. Destination direbase setelah object rename; pin dilepas sebelum compensation. |
| MM-L3-R2 | Participant no-op untuk memperbaiki spelling stale ikut dipin selama projection. Regresi swap leaf RED menjadi GREEN.                                                                   |
| MM-L2-R2 | `only-enable` ke container tetap men-disable sibling di parent sibling sendiri. Identitas/bytes dan restart lulus unit/native.                                                          |
| MM-L6    | Windows toggle memakai source handle yang sudah diverifikasi, bukan lookup path kedua. Race replacement direproduksi secara deterministik; replacement dan collision tetap utuh.        |

Review independen terakhir tidak menemukan actionable finding dalam perubahan
Windows ini. Tiga regresi sementara pada koreksi—self-lock object, lookup parent
Windows biasa/verbatim, dan routing sibling `only-enable`—ditutup sebelum gate final.

## Verifikasi yang dijalankan

| Gate                                                                 | Hasil final                                                                                               |
| -------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `rtk cargo test --lib`                                               | **1.744 passed, 0 failed, 29 ignored**; ignored tetap terlihat, tidak dihapus/di-exclude sebagai solusi.  |
| Enam suite frontend navigation/grid/runtime/selection/watcher/paging | **98 passed**.                                                                                            |
| Native Windows WDIO, `mods-lifecycle-hardening.e2e.ts`               | **5/5 passed**, WebView2 154.0.4258.62, session `6ffe64648026e155b481bfd0fcd34f5f`, 2026-10-10 20:31 WIB. |
| App/E2E typecheck dan production frontend build                      | Passed; native membangun binary baru dengan identifier `com.reynalivan.emmm.e2e`.                         |
| `rtk cargo clippy --all-targets -- -D warnings`                      | Passed pada source final.                                                                                 |
| Scoped ESLint, rustfmt, Prettier, whitespace                         | Passed.                                                                                                   |
| Schema fix-report dan source integrity                               | Passed; 25 hash stabil.                                                                                   |

Jumlah 1.847 dalam JSON adalah 1.744 Rust + 98 frontend + 5 native, bukan penjumlahan
ulang seluruh run/child worker. Baseline 14 adalah suite navigation awal yang
lulus dua kali, bukan baseline full repository. Tidak ada flaky-test exclusion.
Run global sebelumnya tetap tercatat: RED KeyViewer milik task startup kemudian
ditutup pemiliknya; failure replacement toggle terbukti race produksi dan difix.
Native pertama 4/5 terhenti pada search yang masih terlipat; kontrol dibuka lewat
UI sebenarnya, tanpa bypass atau menghapus kasus gagal.

Native final mencakup paging 100/100/5, cursor query-bound/eviction, navigasi search
kosong, penolakan junction, `only-enable` dengan sibling di parent lain, restart,
dan apply collection. Bytes asset sumber/sibling dipertahankan. Driver milik
run dihentikan sebelum fixture sementara dibersihkan; fixture dapat dibuat ulang
dari spec. Uji crash receipt memakai SQLite/journal nyata dan abrupt child exit,
bukan simulasi browser atau klaim full-app crash test.

## Watcher: bukti koordinasi, bukan hitungan native di atas

Pemilik chat “Cek alur Team Inbox hingga Move” menutup gap idle/suppressor melalui
loop Tauri nyata: last-owner release, echo expiry, dan abort; revision 6/7/8
terikat generation 3/4/5 dengan artifact Published. Parent membaca
[history](E:/code/projects/EMMM2/docs/history/202610100020-close-watcher-convergence-gap.md)
dan [report native](E:/code/projects/EMMM2/.audit-output/watcher-gap-verification/native-report.json).
Ini bukti tambahan dari task pemilik; tidak ditambahkan ke 5 kasus/1.847 pass milik
laporan ini. Reload game nyata tidak diklaim oleh fixture tersebut.

## Semua item yang masih perlu manual review

1. **MM-L6-LINUX — celah source binding belum ditutup di Linux.** Adapter
   non-Windows masih menerbitkan source berdasarkan pathname. Parent anchoring
   dan `RENAME_NOREPLACE` melindungi destination, bukan atomic identity source.
   Perlu kontrak/design khusus Linux dan pengujian Linux sebelum mengklaim aman
   lintas platform. macOS, network filesystem dan ownership-bound recovery di
   platform lain juga tidak disertifikasi oleh run Windows ini.
2. **Recovery legacy/ambigu.** Collection journal tanpa root epoch atau bukti
   completion, root/participant yang diganti, receipt dengan binding berbeda,
   journal malformed/foreign-root, serta staging berisi asset tambahan tetap
   meminta repair manual. Jangan menghapus journal/asset untuk memaksa recovery.
   Jika cleanup receipt gagal, bukti dipertahankan; orphan terminal boleh
   diinspeksi saat maintenance, tetapi tidak mengautorisasi operasi lain.
3. **Kompatibilitas upgrade/downgrade.** Writer sekarang format journal/progress
   4; reader baru membaca 1–3. Jangan downgrade ke reader format 3 ketika evidence
   format 4/receipt masih ada. Migrasi additif tidak menghapus data lama; review
   packaging/backup dan kompatibilitas recovery sebelum distribusi.
4. **Crash Safe Mode deferred completion.** Abrupt-exit test membuktikan jalur
   collection/task Completed. Full-app crash pada jalur
   `defer_task_completion=true` belum diuji; belum ada bug baru yang terkonfirmasi
   pada jalur tersebut.
5. **Batas native UI yang belum dijalankan di continuation ini.** Virtual-scroll
   besar, symbolic bulk selection lintas halaman, kombinasi multi-select
   `only-enable`, positive recycle-bin delete, external editor/Mod Viewer,
   clipboard/native picker, dan full CRUD/conflict-overlay matrix perlu QA
   terpisah. Unit mencakup paging/selection/structural recovery, tetapi tidak
   menggantikan seluruh matrix native. Preview hide/restore dan MM-5/MM-6 sudah
   memiliki bukti native terdahulu, bukan rerun pada laporan ini:
   [completion sebelumnya](E:/code/projects/EMMM2/docs/history/202610100011-complete-mods-preview-and-folder-recovery.md).
6. **Game/runtime nyata dan benchmark.** Launch loader/game, global hotkey,
   focused-game reload, serta library besar nyata tidak disentuh. Game state
   Running/Unknown masih dapat sah menghasilkan manual-reload notice. Benchmark
   dan ignored crash-worker helpers bukan otomatis bug: sebagian worker diuji
   melalui parent; prasyarat/performance real-library tetap berada di task terkait.

## File implementasi yang terdampak

Daftar tepat 25 file terdapat pada snapshot di atas. Kelompoknya:

```text
src/widgets/mod-explorer/hooks/useFolderNavigation.{ts,test.ts}
src-tauri/src/modules/library/application/mods/{organizer_move.rs,organizer_targets.rs,tests/organizer_move_tests.rs,core_ops/toggle.rs}
src-tauri/src/modules/collections/application/apply/{apply_pipeline.rs,apply_pipeline_tests.rs,steps/batch_rename.rs,steps/rollback.rs}
src-tauri/src/modules/mutation/{mod.rs,api.rs,coordinator.rs,journal.rs,recovery.rs,tests.rs,application/workspace_mutation/engine.rs,adapters/mod.rs,adapters/sqlite/mod.rs,adapters/sqlite/projection_receipts.rs}
src-tauri/src/modules/system/application/app/bootstrap.rs
src-tauri/src/platform/fs/file_utils.rs
src-tauri/migrations/20261010000000_mutation_projection_receipts.sql
src-tauri/AGENT.md
tests/e2e/specs/mods-lifecycle-hardening.e2e.ts
```

Dokumentasi/provenance tambahan: `lifecycle_implementation_plan.md`, laporan ini,
history continuation, serta `.bug-hunter/mods-lifecycle-fix/` berisi manifest,
baseline, evidence dan laporan. Perubahan lain di checkout tidak direvert.
