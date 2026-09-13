# Demo mode frontend-only yang strictly development-only

## Tujuan

Menyediakan aplikasi EMMM yang dapat dibuka di browser Vite dengan data dummy deterministik untuk audit UI seluruh surface, tanpa Tauri, filesystem, database, download native, atau data mod pengguna.

## Keputusan desain

- Mode aktif hanya melalui `pnpm dev:demo`, yang menjalankan Vite dengan mode `demo`.
- Guard sumber tunggal: `isDemoMode = import.meta.env.DEV && import.meta.env.VITE_APP_MODE === 'demo'`.
- Build Tauri dan `pnpm build` selalu memakai mode production. Jika `VITE_APP_MODE=demo` muncul saat build, konfigurasi Vite harus menghentikan build dengan error.
- Fixture tidak boleh di-static-import dari entry production. Demo bootstrap melakukan dynamic import setelah guard, dan konfigurasi build meng-alias modul demo ke stub yang gagal tertutup.
- Tidak ada mock global untuk `window.__TAURI__` atau `commands`. UI memakai API feature-local; adapter Tauri tetap memanggil bindings saat aplikasi normal, adapter demo memakai state in-memory.
- Data demo diberi badge non-produksi yang selalu terlihat: `Demo data`. Badge tidak dirender di mode normal maupun produksi.

## Struktur target

```
src/demo/
  runtime/appMode.ts                 # guard DEV + VITE_APP_MODE
  runtime/DemoProvider.tsx           # scenario state dan reset on refresh
  runtime/demoDisabled.ts             # stub production yang throw fail-closed
  scenarios/manifest.ts               # daftar route, scenario, dan metadata
  fixtures/{dashboard,mods,...}.ts    # data tetap, tidak memakai Date.now/random
  adapters/{dashboard,mods,...}.ts    # API in-memory per feature
  ui/DemoModeBadge.tsx
```

API yang dipakai component ditempatkan berdekatan dengan feature, misalnya `pages/dashboard/api/dashboardGateway.ts`. Kontraknya berisi operasi UI yang benar-benar dibutuhkan, bukan seluruh surface `commands` Tauri.

## Skenario yang wajib tersedia

Setiap scenario mempunyai ID stabil, deep link, dan reset state saat refresh.

| Surface | Scenario minimum |
|---|---|
| Global shell, sidebar, topbar | default, nama game panjang, no active game |
| Dashboard | full, loading, error, empty activity |
| Mods Manager | dense, conflicts, unsafe, filtered-empty, mutation-pending |
| Collections | populated, large, missing-mod confirmation, empty |
| Mod Inbox | incoming, validation error, empty |
| Storage Optimizer | scan-result, no duplicates, scan error |
| Discover dan Downloads | tabs, bookmarks/history, queue, failed download |
| Settings | configured, empty games, destructive confirmation |
| Onboarding | welcome, manual-invalid, auto-detect result |
| Modal dan wizard | open state per dialog, keyboard close, validation and pending state |

Scenario URL memakai hash route agar BrowserRouter tidak memerlukan server fallback khusus, misalnya `/#/demo/mods?scenario=conflicts`. Query parameter hanya memilih scenario yang terdaftar dalam manifest; parameter tak dikenal kembali ke default dan menampilkan warning development.

## Tahapan implementasi

1. **Guard dan bootstrapping**
   - Tambahkan `dev:demo` serta Vite mode `demo`.
   - Tambahkan type untuk `VITE_APP_MODE`, fail-closed resolver, badge, dan production build guard.
   - Ubah `AppRouter` agar demo melewati `appStartupCheck`, `checkConfigStatus`, logger Tauri, metadata sync, serta recovery check.
   - Tambahkan unit test untuk guard: mode normal, demo dev, dan demo yang ditolak di production.

2. **Shell dan navigation demo**
   - Tambahkan scenario manifest, scenario picker khusus development, deep link, reset action, dan state language/theme.
   - Pastikan picker tidak memakai literal production-facing copy dan tidak ikut production bundle.
   - Verifikasi semua workspace view dapat dirender tanpa bridge Tauri.

3. **Adapter per vertical slice**
   - Ekstrak kontrak kecil dari command direct-import, mulai Dashboard dan Mods Manager.
   - Tambahkan adapter demo in-memory untuk query dan mutation yang memengaruhi UI: toggle, filter, select, save, clear, retry, dan confirm.
   - Ulangi untuk Collections, Mod Inbox, Storage, Discover/Downloads, Settings, dan Onboarding. Jangan mengubah adapter Tauri selain memindahkan pemanggilan existing ke gateway.

4. **Fixture berkualitas audit**
   - Gunakan ID, timestamp UTC, path, dan urutan item yang tetap.
   - Sertakan nama panjang, Windows paths, Unicode/ID/ZH, data kosong, loading, error, dan 60+ row untuk list virtualized.
   - Tidak ada statistik, user activity, testimoni, atau data yang bisa terbaca sebagai klaim produk nyata. Semua data bersifat operasional dan badge demo menjelaskan konteksnya.

5. **Modal, wizard, dan aksesibilitas**
   - Daftarkan modal yang dapat dibuka melalui scenario, bukan melalui global flag tersembunyi.
   - Uji focus awal, Tab confinement native dialog, Escape untuk dialog non-blocking, dan label/description tiap dialog.
   - Modal recovery tetap bukan scenario yang dapat melakukan action permanen; tampilkan simulasi read-only bila perlu untuk audit visual.

6. **Browser QA dan regression**
   - Buat smoke suite Vitest untuk manifest, resolver scenario, dan mutation reducer.
   - Tambahkan screenshot/browser checklist pada 375x812, 768x1024, dan 1440x900 untuk Light/Onyx serta ID/EN/ZH.
   - Simpan test native yang ada di WDIO untuk IPC, disk, dialog native, dan persistence. Mode demo tidak menggantikannya.

## Kriteria penerimaan

- `pnpm dev:demo` membuka seluruh surface di browser tanpa `__TAURI__`, request filesystem, atau write database.
- Mutation demo hanya mengubah memory lokal dan kembali ke fixture ketika halaman direfresh.
- `pnpm build` dan build Tauri gagal jika mode demo dipaksakan, tidak merender badge demo, dan tidak memuat fixture/chunk demo.
- Semua scenario manifest dapat dibuka melalui deep link dan memuat state yang deterministik.
- Browser QA dapat mengaudit seluruh daftar surface, modal, state loading/empty/error, responsive layout, i18n, dan keyboard.
- E2E WDIO yang memakai app-data terisolasi tetap hijau dan tetap menjadi bukti integrasi native.

## Non-goal

- Tidak membuat backend mock global, network service palsu, atau file game palsu di runtime browser.
- Tidak menjalankan download, scan disk, automation, launcher, native dialog, atau persistence asli dari browser-demo.
- Tidak memasukkan kontrol demo, fixture, atau route demo ke release build.

## Validasi implementasi nanti

1. Unit test guard, scenario manifest, fixture reducer, dan gateway adapter.
2. `pnpm lint`, `pnpm build`, lalu assertion artifact bahwa tidak ada `DemoModeBadge`, fixture marker, atau `VITE_APP_MODE=demo` di `dist`.
3. Browser visual matrix dan keyboard smoke test untuk setiap scenario.
4. Targeted WDIO E2E existing untuk membuktikan native path tidak berubah.
