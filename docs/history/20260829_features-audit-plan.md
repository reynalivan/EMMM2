# Features Architecture Deep-Dive & Cleanup Plan

Setelah melakukan audit struktural berlapis sedalam mungkin pada `src/features/`, saya menemukan bahwa secara keseluruhan konvensi *kebab-case* untuk nama folder dan *PascalCase*/*camelCase* untuk penamaan file sudah sangat baik. Namun, ada beberapa **inkonsistensi** dalam hal penempatan file (terutama di level *root feature*) dan pengelompokan subfolder yang sedikit melenceng dari standar murni *Feature-Sliced Design (FSD)*.

Berikut adalah temuan dan **saran plan** untuk merapikan struktur internal setiap fitur:

## 1. Merapikan Root Feature (Entrypoint Rule)
> **Prinsip FSD**: Root dari sebuah direktori fitur (e.g. `src/features/nama-fitur/`) idealnya **hanya** berisi *entrypoint component* (halaman utama) atau `index.ts` (Public API). Sub-komponen, hook, dan utilitas harus masuk ke dalam subfolder yang sesuai (`components/`, `hooks/`, `utils/`, dll).

**Saran Relokasi:**
*   **`match-wizard`**: 
    *   Sub-komponen (`ImportBatchWizardItemRow.tsx`, `ObjectClassificationWizardHost.tsx`) pindahkan ke `components/`.
    *   Logic (`importBatchDecision.ts`) pindahkan ke `utils/`.
*   **`onboarding`**: 
    *   Sub-komponen (`AutoDetectResult.tsx`, `ManualSetupForm.tsx`) pindahkan ke `components/`.
    *   Hook/state (`indexingProgress.ts`, `useOnboardingDiskProgress.ts`) pindahkan ke `hooks/` atau `utils/`.
*   **`preview`**: 
    *   Helper logic (`keybindingValidator.ts`, `previewPanelUtils.ts`) pindahkan ke `utils/`.
*   **`scanner`**: 
    *   Jika `StorageOptimizerPage.tsx` adalah entrypoint utama, maka `DedupFeature.tsx` sebaiknya masuk ke `components/`.
    *   Logic (`dedupProgress.ts`) masuk ke `utils/` atau `hooks/`.
*   **`file-watcher`**: 
    *   Semua file *reconcile* (`pathUtils.ts`, `reconcileProgress.ts`, `reconcileRefresh.ts`, dll) bertebaran di root. Sebaiknya kelompokkan ke dalam `utils/`.
    *   File `hooks.ts` namanya kurang deskriptif. Ubah menjadi `hooks/useFileWatcher.ts`.

## 2. Standarisasi Nama Subfolder (Layering)
> **Prinsip FSD**: Di dalam folder feature, kita membagi berdasarkan layer teknis (`components`, `hooks`, `utils`, `services`, `modals`), BUKAN berdasarkan sub-domain bisnis lagi.

**Temuan Inkonsistensi & Saran:**
*   **`settings`**: Memiliki folder `tabs/` dan `theme/`.
    *   Dalam standar ketat, komponen-komponen tab seperti `GamesTab.tsx` dan `DynamicThemeInjector.tsx` seharusnya berada di dalam `components/` (bisa di dalam `components/tabs/` jika sangat banyak).
    *   File `settings/tabs/hotkeyConflicts.ts` (yang merupakan murni fungsi utility) terselip di folder UI. Harus dipindah ke `settings/utils/hotkeyConflicts.ts`.
    *   Hook `useCustomThemes.ts` di folder `theme/` harus dipindah ke `settings/hooks/useCustomThemes.ts`.
*   **`onboarding/welcome/`**: 
    *   Memiliki struktur `scenes/` dan komponen langsung. Karena ini adalah komponen internal dari fitur onboarding, lebih sesuai jika dibungkus ke dalam `src/features/onboarding/components/welcome/`.

## 3. Penamaan File (Naming Conventions)
*   **Komponen UI**: Sudah konsisten menggunakan `PascalCase` (e.g. `StorageOptimizerPage.tsx`, `AnimatedLogo.tsx`).
*   **Hooks**: Sudah konsisten menggunakan format `useCamelCase` (e.g. `usePreviewActions.ts`), kecuali untuk file `file-watcher/hooks.ts` yang perlu di-*rename*.
*   **Logic / Utils**: Sudah konsisten menggunakan `camelCase` (e.g. `importBatchDecision.ts`).

## Rangkuman Eksekusi Batch (Bila Disetujui)
Jika Anda setuju dengan plan ini, eksekusi batch yang bisa saya lakukan adalah:
1. Memindahkan file-file dari root ke `components/`, `hooks/`, dan `utils/` pada 5 folder *features* yang berantakan (`match-wizard`, `onboarding`, `preview`, `scanner`, `file-watcher`).
2. Menghilangkan struktur folder _domain-in-domain_ (`tabs`, `theme` pada `settings`) dan meleburnya ke struktur teknis FSD (`components`, `hooks`, `utils`).
3. Menjalankan skrip pencarian-dan-perbaikan `import` (seperti yang dilakukan pada langkah refactor besar sebelumnya) agar semua *relative paths* tetap valid dan tidak ada path yang mati.

*Status: Read-only plan disajikan.*
