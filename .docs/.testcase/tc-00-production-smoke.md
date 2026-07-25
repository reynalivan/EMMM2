# TC-00 — Production Smoke Checklist (Data-Safety Paths)

> Jalankan terhadap **folder mod sandbox** (copy kecil dari library asli) memakai build release (`pnpm tauri build`), SEBELUM dogfooding di library asli. Semua item harus lulus. Detail langkah per fitur ada di tc-NN terkait.

| # | Skenario | Verifikasi | Ref | Status |
|---|----------|-----------|-----|--------|
| 1 | Toggle mod on/off | Prefix `DISABLED ` benar di disk, tidak ada file hilang, UI sinkron setelah reconcile | tc-20 | ☐ |
| 2 | Delete mod → restore | Folder pindah ke `./app_data/trash/`, restore mengembalikan utuh, tidak ada hard delete | tc-22 | ☐ |
| 3 | Import .zip/.7z (termasuk ber-password) | Hasil ekstraksi benar, collision dialog muncul saat nama bentrok | tc-23, tc-37 | ☐ |
| 4 | Apply collection dengan sebagian mod hilang | Transaksional, partial failure ditampilkan ke user, tidak ada state setengah jadi | tc-31 | ☐ |
| 5 | Rename/toggle saat file dikunci proses lain | Retry dialog muncul, operasi bisa diulang, tidak ada folder korup | tc-13, tc-21 | ☐ |
| 6 | Kill app paksa di tengah bulk toggle | Setelah restart + reconcile, disk state konsisten (filesystem is truth) | tc-14 | ☐ |

| 7 | Force enable + centang "jangan ingatkan lagi" | Duplicate warning tidak muncul lagi untuk kombinasi mod yang sama; entri tampil di IgnoreManagementModal dan bisa di-revoke | tc-29 | ☐ |
