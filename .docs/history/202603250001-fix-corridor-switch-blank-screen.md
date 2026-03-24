# Fix Corridor Switch Blank Screen

## Context
Saat transisi antara Safe ↔ Unsafe corridor, aplikasi menunjukkan layar kosong (blank) karena mencoba menggunakan data lama (stale) dari corridor sebelumnya untuk me-render UI di corridor baru sebelum data baru selesai di-fetch.

## Changes
- **Query Keys**: Menambahkan `isSafe` ke query key `collectionKeys.list` untuk memisahkan cache per corridor.
- **Hooks (`useCorridor`)**: Menghapus `placeholderData` agar snapshot corridor lama segera terhapus saat berpindah corridor.
- **Hooks (`useCollections`)**: Menambah parameter `isSafe` untuk segregasi cache.
- **UI (`CollectionsPage`)**: Menambah `isFetching` guard pada `effectiveSelectedId` agar tidak jatuh (fallback) ke ID koleksi lama saat transisi.
- **UI (`ContextControls`)**: Sinkronisasi passing `safeMode` ke hook `useCollections`.

## Impacted Files
- `src/features/collections/queryKeys.ts` (modified)
- `src/features/collections/hooks/useCorridor.ts` (modified)
- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/CollectionsPage.tsx` (modified)
- `src/components/layout/top-bar/ContextControls.tsx` (modified)

## Goal
Aplikasi sekarang menangani perpindahan corridor secara mulus tanpa blank screen, dengan transisi loading state yang bersih dan segregasi data yang tepat antar corridor.

## Impact
- Perbaikan race condition pada transisi corridor.
- Reduksi error query akibat cross-corridor collection ID leakage.
- Tidak ada breaking changes pada API backend.
