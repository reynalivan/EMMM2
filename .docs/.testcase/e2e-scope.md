# E2E Test Scope — Phased Plan (WDIO + tauri-driver)

> Sumber kebenaran skenario: file `tc-NN-*.md` di folder ini. Dokumen ini **hanya menyusun urutan & cakupan E2E**, bukan menulis ulang test case.
> Harness: `wdio.conf.ts` → `tauri-driver` + `msedgedriver`, `maxInstances: 1` (serial), butuh build debug `src-tauri/target/debug/emmm.exe`. Jalankan: `pnpm test:e2e`.
> Prinsip: **Disk = source of truth, DB = index**. Setiap fase yang menyentuh disk WAJIB pakai sandbox folder (copy kecil), bukan library asli.

---

## Status implementasi

**Fase 0–9 sudah diimplementasi** (IPC-driven + two-sided disk/DB assert). Gate: `tsc` + `eslint` hijau. Belum dieksekusi runtime (butuh build debug + driver Windows).

| Spec file | Fase | Epic (tc-NN) |
|---|---|---|
| `app.e2e.ts` | 0 | boot + severe-error gate |
| `app-bootstrap.e2e.ts` | 0 | 01 |
| `smoke-data-safety.e2e.ts` | 0 | 00 (toggle/trash/reconcile) |
| `onboarding.e2e.ts` | 1 | 03 |
| `game-management.e2e.ts` | 1 | 02 |
| `phase1-gates.e2e.ts` | 1 | 04, 02 (multi-game/switch) |
| `phase2-navigation.e2e.ts` | 2 | 07, 08, 09, 11 |
| `phase3-mod-ops.e2e.ts` | 3 | 10, 13, 14, 20, 21, 22, 40 |
| `phase4-preview.e2e.ts` | 4 | 16, 17, 18, 19, 41, 43 |
| `phase5-import.e2e.ts` | 5 | 23, 37, 38, 39 |
| `phase6-scan-sync.e2e.ts` | 6 | 25, 26, 27, 28, 32 |
| `phase7-collections.e2e.ts` | 7 | 29, 30, 31 |
| `phase8-runtime.e2e.ts` | 8 | 33, 34*, 35, 36, 42* |
| `phase9-security-browser.e2e.ts` | 9 | PIN(read-only)*, themes, browser |

Support bersama: `test/support/{fixtures,ipc,app,data}.ts`. Fixture arsip: `test/fixtures/sample-mod.zip`. `*` = sebagian [manual-smoke], lihat di bawah.

### Catatan runtime (WAJIB dibaca sebelum `pnpm test:e2e`)

1. **DB tidak terisolasi antar-run.** Spec menulis ke `app_data` DB asli (games/objects/collections/trash menumpuk). Jalankan terhadap build **debug** dengan `app_data` sekali-pakai, atau panggil `reset_database` di global `before` bila mau bersih. Folder mod fixture di temp sudah auto-cleanup.
2. **Channel IPC** (`invokeWithChannel` untuk scanner/dedup/extract) mengandalkan `window.__TAURI__.core.Channel` — valid karena `withGlobalTauri: true` di `tauri.conf.json`.
3. **Urutan serial alfabetis** (`maxInstances: 1`): `app` → `app-bootstrap` → `game-management` → `onboarding` → `phase1..9` → `smoke`. Tiap spec seed game sendiri + nama objek unik, jadi minim bocor state.
4. **[manual-smoke]** (tak diotomatiskan; harus diverifikasi manual): `launch_game` (spawn loader asli), updater real (`tc-34` cuma cek command settle), hotkey in-game (`tc-42` cuma cek registrasi), ekstraksi arsip **ber-password/7z** (`tc-37` cuma zip biasa), dan **set/verify/reset PIN** (`tc-45` — sengaja read-only: PIN basi bisa mengunci app saat boot).

---

## Aturan harness (berlaku semua fase)

1. **Isolasi per-spec.** Tiap spec bikin mock game folder sendiri di `os.tmpdir()` (pola sudah ada di `game-management.e2e.ts` / `onboarding.e2e.ts`), seed via `invoke('add_game_manual', …)`, `after()` hapus folder. Jangan mutasi DB/library user.
2. **IPC-bypass untuk yang tak bisa diklik.** Native file picker, launch game asli, overlay in-game, password prompt arsip → panggil command Rust langsung lewat `window.__TAURI__.core.invoke`, jangan simulasi dialog OS.
3. **Selector kontrak.** Utamakan `[data-testid=…]` / `id`. Kalau elemen target belum punya testid, tambahkan testid di komponen (bagian dari pekerjaan fase itu), jangan andalkan teks yang bisa berubah i18n.
4. **Assert dua sisi.** Untuk operasi disk: verifikasi **disk** (prefix `DISABLED `, lokasi trash) **dan** UI/DB setelah reconcile. Lihat `.docs/flow.md`.
5. **Serial + idempoten.** Karena `maxInstances: 1`, urutan spec dalam satu fase harus tidak saling bocor state. Kill-app test (tc-00 #6) dijalankan paling akhir di fase-nya.
6. **Batas jujur.** Yang tak bisa 100% otomatis (updater real, hotkey in-game, launch game asli) ditandai **[manual-smoke]** — jangan diklaim tercakup.

---

## Fase 0 — Fondasi harness & smoke data-safety
**Tujuan:** infra fixture + jaring pengaman sebelum fitur. **Prasyarat:** build debug ada.
**Deliverable:** helper fixture bersama + smoke release checklist otomatis sebisanya.

| Item | Ref | Catatan E2E |
|---|---|---|
| Extract fixture helper bersama (mock game factory, IPC-seed, cleanup) | — | Refactor dari 2 spec existing → `test/support/`. DRY, dipakai semua fase. |
| Production smoke path (data-safety) | tc-00 | 6 skenario jadi 1 spec "smoke" yang menyentuh toggle/trash/import/apply minimal end-to-end. Sisanya diperdalam di fase spesifik. |
| Boot + no-severe-error gate | tc-01, `app` | Naikkan `app.e2e.ts` jadi hard-fail kalau ada React error `SEVERE`. |

**Exit:** fixture helper dipakai ulang; `pnpm test:e2e` hijau serial.

---

## Fase 1 — Gerbang masuk (Onboarding · Game · Settings)
**Tujuan:** semua jalur yang menggerbangi dashboard. **Prasyarat:** Fase 0.

| Epic | Ref | Fokus flow |
|---|---|---|
| Onboarding / FTUE | tc-03 ✅perluas | Auto-detect gagal → manual add → transisi dashboard; validasi core files (d3dx.ini/d3d11.dll/loader). |
| Game Management | tc-02 ✅perluas | Add/edit/remove game, switch active game, Quick Play launch (**[manual-smoke]** untuk proses game asli — E2E cukup assert command ter-invoke). |
| Settings | tc-04 | Ganti theme, language (i18n runtime), path settings, persist ke store & reload. |

**Exit:** dari fresh state bisa sampai dashboard 3 jalur; ganti setting persist lintas restart.

---

## Fase 2 — Navigasi & permukaan baca (Workspace · ObjectList · Folder listing)
**Tujuan:** projeksi read-only; risiko rendah, cakupan navigasi tinggi. **Prasyarat:** Fase 1 (butuh game + mods terseed).

| Epic | Ref |
|---|---|
| Workspace layout & navigation | tc-05 |
| ObjectList navigation & resizing | tc-06 |
| Object List | tc-07 |
| Smart filters & sorting | tc-08 |
| Object schema & master DB | tc-09 |
| Folder listing & classification | tc-11 |
| Folder Grid UI | tc-12 |

**Seed:** mock game dengan objek + mod berlapis (Object → Skin → Variants) sesuai hierarki `.docs/flow.md`.
**Exit:** navigasi objek→folder, filter/sort, resize panel — semua tanpa mutasi disk.

---

## Fase 3 — Operasi mod inti ⚠️ DATA-SAFETY (risiko tertinggi)
**Tujuan:** semua mutasi disk destruktif. **Prasyarat:** Fase 2. Wajib assert disk + DB.
**Peta ke tc-00:** #1 toggle, #2 trash, #5 file-locked, #6 kill-mid-bulk.

| Epic | Ref | Verifikasi kritis |
|---|---|---|
| Core mod ops (toggle/rename/delete) | tc-13 | Root-cause path bersama. |
| Mod Toggle | tc-20 | Prefix `DISABLED ` benar, tak ada file hilang, UI sinkron pasca reconcile. |
| Mod Rename | tc-21 | Rename saat file dikunci → retry dialog, folder tidak korup. |
| Trash Safety | tc-22 | Pindah ke `app_data/trash/`, restore utuh, **no hard delete**. |
| Object CRUD | tc-10 | Buat/rename/hapus objek konsisten disk↔DB. |
| Metadata actions (pin/favorite/move) | tc-40 | Move antar objek tidak menghilangkan mod. |
| Bulk operations & selection | tc-14 | Kill app paksa di tengah bulk → restart+reconcile konsisten (jalankan terakhir). |

**Exit:** semua operasi reversible/aman; reconcile pasca-crash konsisten.

---

## Fase 4 — Preview & editor
**Tujuan:** panel kanan & editor konten. **Prasyarat:** Fase 2 (baca) — sebagian nulis file INI.

| Epic | Ref | Catatan |
|---|---|---|
| Preview panel layout & state | tc-16 |  |
| Metadata editor | tc-17 | Tulis metadata → persist. |
| INI viewer & editor | tc-18 | Edit → save file INI (assert disk). |
| Image gallery | tc-19 |  |
| Thumbnail cache | tc-41 | Cache hit/miss, invalidasi. |
| Dynamic KeyViewer overlay | tc-43 | Parse key dari INI → render. |

**Exit:** edit metadata/INI persist ke disk; galeri & thumbnail render.

---

## Fase 5 — Import & organisasi ⚠️ DATA-SAFETY
**Tujuan:** pipeline masuknya mod baru + kolisi. **Prasyarat:** Fase 3 (trash/rename matang). **Peta ke tc-00:** #3 import, #6.

| Epic | Ref | Verifikasi kritis |
|---|---|---|
| Mod import pipeline | tc-23 | Import .zip/.7z, hasil ekstraksi benar. |
| Archive extraction | tc-37 | Termasuk arsip ber-password (**[IPC-bypass]** untuk input password). |
| Folder collision resolution | tc-39 | Collision dialog muncul saat nama bentrok, pilihan user dihormati. |
| Auto-organizer | tc-38 | Penempatan otomatis ke objek yang cocok. |
| Explorer interactions (drag/drop) | tc-15 | Drag-drop import (**[IPC-bypass]** jika drop OS tak bisa disimulasi). |

**Exit:** import → ekstrak → resolve kolisi → mod muncul di grid, tanpa file tertimpa diam-diam.

---

## Fase 6 — Mesin scan / sync / match (backend-heavy)
**Tujuan:** command-level E2E (invoke → assert DB/disk), toleran long-running. **Prasyarat:** Fase 2.

| Epic | Ref |
|---|---|
| Scan engine | tc-25 |
| Deep matcher & auto-kategorisasi | tc-26 |
| Sync database (disk reconcile) | tc-27 |
| File watcher (drift eksternal) | tc-28 |
| Smart duplicate scanner | tc-32 |

**Catatan:** file watcher → ubah disk di luar app (fs langsung), assert UI/DB ikut update. Scan besar → naikkan timeout mocha per-spec.
**Exit:** scan mengisi DB benar; drift disk eksternal terdeteksi & terekonsiliasi.

---

## Fase 7 — Collections & konflik ⚠️ DATA-SAFETY (transaksional)
**Tujuan:** apply transaksional & privasi. **Prasyarat:** Fase 3 + 6. **Peta ke tc-00:** #4 partial apply, #7 ignore duplicate.

| Epic | Ref | Verifikasi kritis |
|---|---|---|
| Virtual collections | tc-31 | Apply dengan sebagian mod hilang → transaksional, partial failure ditampilkan, **tak ada state setengah jadi**. Recovery resume pasca-crash. |
| Conflict detection & resolution | tc-29 | Force enable + "jangan ingatkan lagi" → tak muncul lagi, entri di IgnoreManagementModal & bisa revoke. |
| Privacy & Safe Mode | tc-30 | Switch safe-mode: preview runtime parity, konten sensitif tersembunyi. |

**Exit:** apply collection selalu all-or-nothing dengan laporan; safe-mode konsisten disk↔runtime.

---

## Fase 8 — Runtime periferal (Randomizer · Dashboard · Updater · Toast)
**Tujuan:** fitur lintas-potong & yang sebagian manual. **Prasyarat:** Fase 3/7.

| Epic | Ref | Catatan |
|---|---|---|
| Smart randomizer + integrated launcher | tc-35 | Randomize pilih mod valid; launch **[manual-smoke]**. |
| In-game hotkeys & live controls | tc-42 | **[manual-smoke]** overlay in-game; E2E cukup assert registrasi hotkey. |
| Dashboard & analytics | tc-33 | Agregat count benar dari seed. |
| App updater & asset sync | tc-34 | **[manual-smoke]** update real; E2E mock check-update command. |
| Toast & error handling pipeline | tc-36 | Trigger error terkontrol → toast muncul, tidak silent-fail. |

**Exit:** jalur periferal tercakup; item manual terdokumentasi jelas.

---

## Matriks cakupan (semua area terpetakan)

| Fase | Epic (tc-NN) |
|---|---|
| 0 | 00, 01, (app smoke) |
| 1 | 02, 03, 04 |
| 2 | 05, 06, 07, 08, 09, 11, 12 |
| 3 | 10, 13, 14, 20, 21, 22, 40 |
| 4 | 16, 17, 18, 19, 41, 43 |
| 5 | 15, 23, 37, 38, 39 |
| 6 | 25, 26, 27, 28, 32 |
| 7 | 29, 30, 31 |
| 8 | 33, 34, 35, 36, 42 |

*(tc-24 tidak ada di repo — bukan gap.)*

**Urutan eksekusi disarankan:** 0 → 1 → 2 → 3 → 4/6 (paralel secara kerja, serial saat run) → 5 → 7 → 8.
Fase ⚠️ (3, 5, 7) adalah gerbang rilis: tidak boleh dogfooding di library asli sebelum ketiganya hijau di sandbox.

---

## Audit cakupan skenario (2026-07-21)

Dari **277 skenario terdokumentasi** (`TC-NN-XX` di semua tc-doc), **48 `it()` terotomasi** (~17%). Angka rendah karena mayoritas skenario doc adalah interaksi-UI murni & varian kecil. Kualitatif: **41/43 area punya ≥1 happy-path E2E**; kedalaman edge/failure/UI-nya belum.

### Tier A — Automatable via IPC (nilai tinggi)
Sebagian **sudah ditutup** di `phase3b-failure-paths.e2e.ts` (rename reserved-char reject, duplicate object reject, bulk partial-failure, empty_trash). Sisa yang layak ditambah:
- **tc-02 (22 doc, 2 auto):** edit game, remove game, auto-detect, validasi path/core-file gagal.
- **tc-04 (23 doc, 2 auto):** persist tiap tab (maintenance, keyviewer, hotkey config, AI), reset.
- **tc-01 (18 doc, 1 auto):** recovery-task resume, config-state branching, splash close.
- **tc-10/07/08/11/18/25:** varian CRUD, filter/sort, klasifikasi folder, INI malformed, scan besar.
- **tc-13 sisa:** locked-folder rollback (#TC-13-003, butuh lock handle), rapid-toggle idempotency (#TC-13-004).

### Tier B — Butuh `data-testid` + otomasi klik/drag (ditunda; ubah source komponen)
Hampir seluruh **zero-auto areas**: `tc-05` (12), `tc-06` (11), `tc-12` (15), `tc-15` (12), `tc-16` (11) — resize divider + persist, context-menu, breadcrumb, mobile <768px, drag-rect select, panel collapse. Perlu tambah testid di komponen nyata dulu (sekarang cuma ~11 testid di prod DOM), lalu spec klik/drag WDIO.

### Tier C — Lebih tepat unit test (bukan E2E)
`tc-26` (18 skenario matcher: alias/token/hash overlap, tiebreak, AI, GameBanana) & sebagian `tc-25` scoring — akurasi algoritma, idealnya Rust unit test. E2E cukup buktikan pipa jalan (sudah: TC-25-01).

### Tier D — Manual-smoke (sudah ditandai)
`launch_game`, updater real, hotkey in-game, arsip password/7z, set/verify/reset PIN.

**Rekomendasi urutan berikutnya:** tutup Tier A (murni nambah spec IPC, aman) → putuskan Tier B (butuh sentuh source komponen) → Tier C pindah ke unit test.

---

## Progress penutupan gap (2026-07-21, lanjutan)

### Tier A — SELESAI (spec IPC baru, tanpa sentuh source app)
- `phase3b-failure-paths.e2e.ts` — rename reserved-char/empty reject, duplicate object reject, bulk partial-failure, empty_trash (tc-13/10/14/22 edges).
- `phase0b-bootstrap.e2e.ts` — config-status `HasConfig`, startup recovery check (tc-01).
- `phase1b-game-edit.e2e.ts` — edit/remove game via save_settings, auto-detect, auto-close-launcher persist (tc-02).
- `phase1c-settings-depth.e2e.ts` — maintenance, clear thumbnails, hotkey config, keyviewer persist (tc-04).
- `phase2b-navigation-edges.e2e.ts` — master-DB search, enabled/disabled folder classification (tc-08/11).
- `phase4b-editor-edges.e2e.ts` — tags add/remove round-trip, multi-INI listing (tc-17/18).

### Tier B — click-driven UI SELESAI (landmarks + selection + view toggle)
`data-testid` aditif (aman — 169 + 56 unit test tetap hijau) di: TopBar nav (`nav-<id>`), `ResizableWorkspace` (`workspace-desktop/left/main/right`, `resize-handle-left/right`), `ObjectList` (`object-list-panel`), `FolderGrid` (`folder-grid`), `FolderGridToolbar` (`view-grid`/`view-list`). Object row pakai `data-object-id` yang sudah ada.
- `phaseUI-workspace.e2e.ts` — nav App Menu → mods, assert 3-pane landmark mount + switch dashboard↔mods.
- `phaseUI-folder-grid.e2e.ts` — klik object row → FolderGrid mount (single-click → focusObject → selectedObjectFolderPath), toggle grid/list.

**Tier B sisa (sengaja tetap manual/flaky):** pixel-drag resize + persist (tc-05-01/tc-06-01/tc-16-006), drag-rect marquee select (tc-15-005), mobile <768px viewport (tc-06-07..09). Gesture pixel/drag inheren flaky di E2E → manual/visual QA. Testid landmark (`resize-handle-*`) sudah dipasang kalau mau lanjut suatu saat.

### Tier C — tc-26 matcher: BUKAN gap. Sudah 82 unit test Rust lulus (`deep_matcher/tests/**` — alias/token/hash overlap, tiebreak, evidence gate, negative penalty, gamebanana, AI rerank, malformed INI). Tidak perlu spec E2E baru.
### Tier D — manual-smoke tetap: launch_game, updater real, hotkey in-game, arsip password/7z, set/verify/reset PIN.
