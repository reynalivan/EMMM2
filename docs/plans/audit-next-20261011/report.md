# Audit lanjutan Settings dan Active Conflict Review

11 Oktober 2026. Scan-only; tidak ada perubahan source produksi, dependency,
branch, worktree atau commit. Peta fitur/historical evidence ada di
[coverage](coverage.md). Scope sesi ini sebagian repo: 13/20 source Settings,
seluruh lima source conflict-report, dan kontrak langsung settings/updater/theme.
Catalog, Games, Hotkey, GameForm, Trust dan catalog-query internals tidak diaudit
ulang secara mendalam pada sesi ini.

## Temuan terverifikasi

Semua temuan melewati challenge reviewer terpisah. Enam P2 dan satu P3; bukan
klaim seluruh aplikasi bersih atau seluruh native suite telah lulus.

### 1. P2 — Homepage game sebelumnya bisa disimpan ke game baru

`src/pages/settings/components/tabs/BrowserTab.tsx:66-70,113-116`.
Draft A bertahan saat query homepage B belum selesai. Blur menyimpan draft itu
memakai activeGameId B. Repro actual component + real TanStack Query merekam
`browserSetHomepage('game-b', 'https://fixture.invalid/a')`.

Koreksi dari verifikasi native: TopBar menyembunyikan GameSelector di Settings;
alur biasa berpindah lewat Mods lalu memasang ulang BrowserTab. Audit awal terlalu
luas menyatakan jalur UI dapat berpindah langsung ketika BrowserTab tetap mounted.
Ownership konteks tertunda dibuktikan oleh tes komponen; guard initial/pending read
juga mencegah URL default tersimpan sebelum konfigurasi game tersedia. Draft dan
mutation kini terikat game asal; cache memakai game dari variables, bukan render
terbaru. [Hasil perbaikan](../audit-next-fixes-20261011/validation.md).

Ketujuh temuan dan tiga cleanup dalam laporan ini sudah ditangani. Validasi
perbaikan mencakup 89 tes frontend, 26 Rust dan enam native cases terarah; laporan
scan awal di bawah tetap merekam bukti sebelum perbaikan.

### 2. P2 — Native cancel bisa menyembunyikan Conflict dialog yang sibuk

`src/features/conflict-report/ConflictModal.tsx:96-104,198`.
Dialog hanya memasang onClose. Escape dapat mengirim cancel lalu menutup dialog;
handler close justru return ketika isBusy, sehingga LaunchBar tetap menyimpan
open=true. useDialogSync bergantung hanya pada ref/open yang tidak berubah;
settlement maupun klik Open berikutnya tidak membuka ulang dialog.

Cancelable-event repro mendapati defaultPrevented=false saat pending. Aturan
cancel → close berasal dari
[HTML Standard](https://html.spec.whatwg.org/multipage/interactive-elements.html#the-dialog-element).
Perbaikan terkecil adalah mencegah default cancel saat sibuk dan menguji controlled
close-state recovery. Sesi ini belum menjalankan actual Escape di WebView2.

### 3. P2 — Pindah tab kehilangan updater progress dan memungkinkan download ganda

`src/pages/settings/hooks/useAppUpdater.ts:12-18,34-59`;
`src-tauri/src/modules/updates/adapters/tauri/update_cmds.rs:57-95`.
Job IPC tetap berjalan saat General dilepas, tetapi local isInstalling/progress
hilang. General yang dipasang kembali bisa check/install lagi. Deferred IPC repro
mereproduksi dua install-command calls; backend dan pinned updater 2.10.0 tidak
memiliki shared admission guard.

Job/progress perlu dimiliki di luar lifespan tab dan backend menerima hanya satu
job aktif. Dampak terverifikasi dibatasi pada concurrent check/download dan
hilangnya progress. Implementasi Windows memanggil installer lalu exit; tidak ada
klaim pasti dua installer berjalan atau instalasi korup. Tidak ada download/install
updater nyata dijalankan dalam audit.

### 4. P2 — Toggle AI membuang endpoint yang belum disimpan

`src/pages/settings/components/tabs/AITab.tsx:23-27`.
Effect menyinkronkan baseUrl pada setiap settings snapshot. Enabled-only save
memublikasikan snapshot yang masih berisi endpoint lama, menimpa draft pengguna.
Repro menggunakan useSettings dan TanStack Query asli. Lindungi dirty draft dan
sinkronkan perubahan field relevan, bukan seluruh settings object.

### 5. P2 — Completion save key lama membuang draft key baru

`src/pages/settings/components/tabs/AITab.tsx:51-58`.
Input masih editable selama IPC credential berjalan. Completion save sebelumnya
selalu mengosongkan input/dirty flag, termasuk key baru yang diketik selama wait.
Deferred fixture-key repro mengamati draft kedua hilang tanpa disimpan.

Kosongkan hanya draft yang dikirim jika masih current, atau kunci editing selama
keseluruhan save chain. Tidak ada credential asli digunakan atau ditampilkan.

### 6. P2 — Custom theme menerima ID yang bertabrakan dengan bawaan

`src-tauri/src/modules/system/adapters/tauri/theme_cmds.rs:266-285,426-429`.
ID onyx/light/system lolos slug validation dan tersimpan. General menambahkan
option dengan value yang sama seperti bawaan, sementara DynamicThemeInjector
melewati custom loading untuk semua ID itu. Config warna/background yang baru
diimpor tidak dapat diterapkan sebagai custom theme.

Tolak reserved IDs pada backend import/save validation; pertahankan ID custom
lain. Repro frontend onyx mendapati query dinonaktifkan; penerimaan backend
divalidasi melalui actual validator/call path, bukan native picker.

### 7. P3 — Default/legacy dark tidak sesuai kontrak tema frontend

`src-tauri/src/modules/settings/application/config/models.rs:125`,
`persistence.rs:44` dan `src/shared/lib/themeOptions.ts:13-20`.
Backend masih mengembalikan dark, sedangkan pilihan bawaan hanya
system/onyx/light. Frontend meneruskannya sebagai tema custom; select dengan value
tak dikenal menampilkan pilihan System dan injector mencoba memuat dark.json.

Warna Onyx tetap tersedia lewat default root CSS; ini bukan laporan UI tanpa
style. Samakan fresh defaults ke onyx, lalu tangani nilai historis dark dengan
memperhitungkan custom theme ber-ID dark yang mungkin sudah valid. Jangan
menghapus compatibility untuk data tersimpan hanya karena runtime baru memakai
nama berbeda.

## Cleanup dan simplifikasi yang terverifikasi

| Kandidat                                                  | Bukti                                                                                                                                | Perubahan terkecil saat diimplementasikan                                                                                                                                                                             |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| save_custom_theme/delete_custom_theme/export_custom_theme | Tidak ada caller runtime frontend dalam src/tests/scripts; tersisa command declaration, registry, generated bindings dan permissions | Hapus ketiga endpoint beserta exclusive delete/export wrappers, lalu sinkronkan registry/allowlist/bindings/test. Pertahankan import, load, list, export template dan save/read/serialize helpers yang dipakai import |
| chooseConflictWinner                                      | Hanya dipakai conflictResolution.test.ts; jalur UI memakai chooseConflictModSetWinner                                                | Pindahkan winner/overlap coverage ke API aktif, hapus wrapper test-only dan gabungkan helper private jika tinggal satu caller                                                                                         |
| Alias CSS DaisyUI lama --p/--s/--a/--n/--b1               | Generator sudah menghasilkan --color-*; src dan installed DaisyUI/quick-liquid tidak membaca alias pendek tersebut                   | Hapus lima cabang compatibility; gunakan BUILTIN_THEMES bersama untuk injector/runtime/options agar daftar bawaan tidak drift                                                                                         |

Tidak ada dependency yang terverifikasi aman dibuang. Migrasi retention browser,
revision CAS, recovery journal, validation dan guard identity/filesystem tetap
diperlukan. Pengurangan state/paths untuk updater dan draft dilakukan bersamaan
dengan perbaikan ownership di atas, bukan refactor luas yang terpisah.

## Validasi yang dijalankan

- Existing ConflictModal/conflictResolution/LaunchBar: **27/27 passed**.
- Isolated audit repro suite: **9 assertions**, **7 expected failures** yang
  sesuai temuan, **2 positive controls passed**. Ini bukti kondisi sebelum fix;
  bukan suite GREEN atau bukti native Escape/credential/update integration.
- Awal harness diperbaiki setelah selector i18n salah, refetch fixture masih
  mengembalikan snapshot lama, dan async mutation belum di-flush. Report final
  hanya memakai repro setelah ketiga masalah harness diperbaiki.
- Source hash tujuh file utama tidak berubah sejak pembacaan awal audit.
- Bug Hunter findings/skeptic/referee JSON valid menurut canonical schemas.
- Tidak menjalankan build, Cargo atau native app karena source produksi tidak
  diubah dan audit tidak memerlukan installer/OS side effects.

Evidence lokal yang diabaikan Git:
`.audit-output/audit-next-settings-20261011/{repros.json,conflict-baseline.json,*.test.tsx,vitest.config.ts}`.
Canonical findings/challenges/verdicts:
`.bug-hunter/audit-next-settings-20261011/{hunter-findings.json,skeptic.json,referee.json,recon.json}`.

Audit ini belum meliputi seluruh source Settings atau seluruh repository.
Kelanjutan terarah ada dalam coverage.md; complete queued coverage memerlukan
pass tambahan pada file/subflow yang disebutkan di sana.
