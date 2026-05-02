# ObjectList State Contract And Repair Refactor

## Context

`ObjectList` masih rawan `Maximum update depth exceeded` karena selector runtime, filter derivation, store setter, search worker, dan disk-repair effect saling mendorong rerender untuk nilai yang sebenarnya sama.

## Changes

- Menambahkan contract stabil untuk workspace selection input agar query `WorkspaceViewModel` tidak lagi bergantung pada selector object literal yang berubah identitas tiap render.
- Memisahkan persisted object filters dari effective sanitized filters, lalu menghapus write-back sanitization dari `useObjectListLogic`.
- Menjadikan setter object-list di app store idempotent agar value yang sama tidak mem-publish state baru.
- Mengubah search worker menjadi result-cache service yang no-op bila query dan hasil semantik tidak berubah.
- Memindahkan disk validation/repair selection dari `ObjectList` ke hook khusus yang guarded dan idempotent.

## Impacted Files

- `src/features/workspace-runtime/useWorkspaceViewModel.ts` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/features/object-list/ObjectList.tsx` (modified)
- `src/features/object-list/hooks/useSearchWorker.ts` (modified)
- `src/stores/useAppStore.ts` (modified)
- `src/features/object-list/objectFilterState.ts` (added)
- `src/features/object-list/hooks/useObjectSelectionRepair.ts` (added)
- `src/features/object-list/objectFilterState.test.ts` (added)
- `src/features/object-list/hooks/useSearchWorker.test.ts` (added)
- `src/features/object-list/hooks/useObjectSelectionRepair.test.ts` (added)
- `src/stores/useAppStore.objectList.test.ts` (added)
- `src/features/object-list/useObjectListLogic.test.ts` (modified)

## Goal

`ObjectList` sekarang memakai selector/query input stabil, derived filters murni, setter store idempotent, dan repair effect terisolasi sehingga jalur render tidak lagi menulis ulang state ekuivalen.

## Impact

- Mengurangi risiko infinite render loop di `ObjectList`.
- Menurunkan churn rerender dari search/filter/store sync.
- Tidak ada perubahan backend, IPC, atau schema DB.

## Notes

- Refactor ini sengaja memindahkan boundary ke selector/helper/hook kecil agar class bug yang sama tidak kembali lewat effect write-back di surface utama.
