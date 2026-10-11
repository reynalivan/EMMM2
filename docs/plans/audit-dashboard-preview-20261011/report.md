# Audit Dashboard dan integrasi preview — 11 Oktober 2026

Scope lanjutan: Dashboard, clipboard/native image picker, penyimpanan gambar,
watcher refresh, dan Mod Viewer receipt/review. Ini audit source satu pass;
source produksi tidak diubah. Satu repro hook frontend dijalankan setelah slot
validasi dilepas; tidak ada build/Cargo/native pada pass ini.
[Peta coverage 16 kelompok fitur](coverage.md) memisahkan subflow yang telah
diperiksa dari bagian yang masih terbuka.

## Lima bug terkonfirmasi

### P2 — Image import dapat membuat ulang folder yang sudah dipindah

[mod_thumbnail_cmds.rs:21](../../../src-tauri/src/modules/library/adapters/tauri/mod_thumbnail_cmds.rs#L21)
memvalidasi folder sebelum menunggu operation lock pada 26–28, lalu menulis
path yang ditangkap pada 31. Operasi rename yang sudah memegang lock dapat
selesai sebelum waiter mendapat lease. Readiness menjadi bersih, tetapi folder
asal sudah pindah; [metadata.rs:68](../../../src-tauri/src/modules/library/application/mods/metadata.rs#L68)
menulis `preview_custom.png`, dan
[atomic_file.rs:33](../../../src-tauri/src/platform/fs/atomic_file.rs#L33)
membuat ulang parent yang hilang.

Paste/save memiliki guard folder hilang, tetapi belum membuktikan identitas
folder asal jika folder lain menggantikan path yang sama.
Gunakan capture/revalidate root dan folder `FilesystemIdentityProof` di bawah
lease, mengikuti
[hide-image:150](../../../src-tauri/src/modules/library/adapters/tauri/preview_cmds.rs#L150).
Reproduksi yang diperlukan: rename-before-write waiter dan same-path replacement.
Ini temuan kode yang sudah ada, bukan regresi akibat diff terbaru.

### P2 — Analisis Mod Viewer usang dapat muncul lagi setelah dismiss/relaunch

[modViewerExternalReviewState.ts:151](../../../src/features/mod-runtime/actions/modViewerExternalReviewState.ts#L151)
mengambil receipt, menunggu analisis pada 160, lalu memublikasikan report/review/toast
pada 164–194 tanpa memeriksa receipt atau urutan analisis lagi.
Dismiss/relaunch selama await dapat dibatalkan oleh completion lama.
Dua analisis juga dapat selesai terbalik karena
[watcher:297](../../../src/features/file-watcher/hooks/useFileWatcher.ts#L297)
tidak menunggu bridge tersebut.

Admission watcher sebelum dispatch tidak menjaga completion async yang telah
berjalan. Tambahkan token receipt dan generation analisis; invalidasi saat
dismiss/relaunch dan periksa sebelum seluruh publikasi. Coverage lama memeriksa
dismiss/replacement sinkron, belum deferred completion.

### P2 — Review Dashboard yang sudah dibatalkan tetap membuka wizard

[DashboardIdentitySuggestions.tsx:42](../../../src/pages/dashboard/components/DashboardIdentitySuggestions.tsx#L42)
melanjutkan pagination dan memanggil launcher global tanpa guard ownership.
Not now, pindah halaman, atau pindah game tidak membatalkannya.
Game selector tersedia di Dashboard; subtree Dashboard diganti saat game berubah,
sedangkan host wizard hidup di luar page content.

Batalkan generation saat dismiss/unmount/perubahan konteks dan periksa sebelum
launch. Request masih membawa game asal; temuan ini adalah UI usang, bukan bukti
penulisan ke game baru.

### P2 — Dua peluncuran classification wizard dapat mencampur state

[ObjectClassificationWizardHost.tsx:80](../../../src/features/match-wizard/components/ObjectClassificationWizardHost.tsx#L80)
menyetel request sebelum await, lalu menulis schema/items/catalog/drafts/busy
tanpa guard generation. Dua review Dashboard dari A dan B dapat launch bersamaan.
Hasil A yang terlambat dapat menimpa preview B; error A juga dapat menutup wizard B.

Hanya launch aktif yang boleh mengubah state, callback, error dan busy; kosongkan
preview lama saat request baru masuk. Backend memeriksa game-owned object IDs dan
fingerprint sebelum transaksi, sehingga korupsi lintas game tidak diklaim.

### P2 — Tombol dismiss tidak menghilangkan review yang sedang terlihat

[modViewerExternalReviewState.ts:110](../../../src/features/mod-runtime/actions/modViewerExternalReviewState.ts#L110)
menghapus query, sedangkan UI berlangganan disabled query observer. Repro memakai
hook produksi dan TanStack 5.101.2 asli: cache hilang, tetapi output tetap
`visible` setelah dismiss; expected `none` gagal. Kontrol
`setQueryData(reviewKey, null)` pada observer yang sama lulus.

Caller produksi meneruskan review/dismiss langsung dari PreviewPanel ke
ModHealthSection, tanpa perubahan state lain pada klik dismiss. Publikasikan
`null` ke query aktif agar subscriber diberi tahu, serta pertahankan guard receipt
untuk mencegah analisis lama memulihkan review. Ini berbeda dari completion race
sebelumnya. Repro ada di ignored
`.audit-output/audit-dashboard-preview-20261011/viewer-dismiss.test.tsx`;
[catatan resmi v5](https://github.com/tanstack/query/blob/main/docs/framework/react/guides/migrating-to-v5.md)
juga membedakan removal cache dari pemberitahuan observer.

## Cleanup terverifikasi

| Kandidat                                                                            | Perubahan terkecil                                                                                                                                                 |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [useModInfo:90](../../../src/widgets/mod-preview/hooks/usePreviewData.ts#L90)       | Hanya definition dan test caller; hapus hook/test khususnya. Pertahankan backend `readModInfo` dan query key yang masih dipakai jalur lain.                        |
| [Dashboard fallback:20](../../../src/pages/dashboard/Dashboard.tsx#L20)             | `stats` sudah diakses sebelum fallback; DTO Rust/generated/demo selalu mengisi field wajib. Hapus `EMPTY_STATS` dan fallback `??0/??[]`, gunakan payload langsung. |
| [Pie gradients:90](../../../src/pages/dashboard/components/DashboardCharts.tsx#L90) | Pie memakai warna CSS, bukan gradient. Hapus pemanggilan `ChartGradients` pada pie; pertahankan helper/defs milik bar.                                             |

Legacy IPC
[list_mod_preview_images:114](../../../src-tauri/src/modules/library/adapters/tauri/preview_cmds.rs#L114)
tidak memiliki caller UI produksi, tetapi masih dipakai banyak native assertions
di client-workspace-smoke dan phase4-preview. Kandidat penghapusan ini bersyarat:
pindahkan assertion ke API visibility-aware dan filter `!is_hidden` agar kontrak
visible-only tetap teruji. Jangan sekadar mengganti string endpoint atau menghapus
helper/tests yang masih menjaga perilaku tersebut.

Tidak ada dependency baru yang terverifikasi aman dibuang. Migrasi tersimpan,
journal recovery dan filesystem identity guard tetap diperlukan.

## Tiga kandidat yang belum dikonfirmasi

1. Import/paste context-menu tidak langsung invalidasi gallery. Watcher saat ini
   mengamati authority sebelum suppression dan menjalankan catch-up setelah drop,
   sehingga persistent stale UI belum terbukti. Verifikasi dengan watcher tertunda
   atau coverage gagal; direct gallery invalidation dapat memberi settlement pasti.
2. Clipboard/picker mempertahankan folder lama sementara game berasal dari mutation
   hook. Timing observer/mount belum direproduksi; containment menolak mismatch
   antar root terpisah. Jangan klaim wrong-game write.
3. Launch Mod Viewer belum memeriksa identitas folder asal setelah receipt scan.
   Same-path replacement belum diuji native; game/folder frontend sudah dibekukan.

Dugaan error suggestion hilang tanpa feedback gugur: global unhandled rejection
meneruskan diagnostic ke dialog aktif. Dugaan thumbnail invalidation tanpa game ID
gugur: key itu memang prefix untuk semua game-scoped entries.

## Validasi dan batas

- Pembacaan source, caller search (termasuk source/test untracked), serta review
  adversarial independen selesai untuk temuan yang dilaporkan.
- Lima canonical JSON artifacts lulus schema validation; 30 selected source hashes
  dibandingkan dari capture sebelum review final sampai penyimpanan hasil, zero drift.
  Hash ini menjaga snapshot review, bukan bukti runtime atau full source coverage.
- 11 source Dashboard dibaca penuh; dependency/backend/integration edges dibaca
  sesuai hipotesis. Tiga kandidat tetap pending; seluruh app belum selesai diaudit.
- Satu isolated Vitest run memuat hook produksi dan TanStack asli: regression
  dismiss gagal sesuai bug yang dicari, kontrol publikasi null lulus (dua cases,
  19.01 detik). Ini bukan klaim suite proyek hijau atau native UI certification.
- Tidak menjalankan app build/typecheck/lint/Cargo/native pada pass ini. Repro
  frontend dijalankan setelah handoff original hotkey dan release slot startup;
  giliran compiler kemudian dilepas kembali. Source/config/dist tetap tidak disentuh.
- JSON automation source of truth berada di ignored
  `.bug-hunter/audit-dashboard-preview-20261011/{hunter-findings,skeptic,referee,scan-report,coverage}.json`.
