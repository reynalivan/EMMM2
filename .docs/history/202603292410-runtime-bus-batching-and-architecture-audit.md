# Add Runtime Bus Batching and Mods Architecture Audit

## Context

Fase 3 dan 4 masih tertahan oleh dua masalah: refresh bus belum punya batching ringan untuk burst mutation, dan belum ada guard test yang menjaga jalur utama `mods` agar tidak regress ke helper selection atau invalidation lama.

## Changes

- Menambahkan batching microtask pada runtime refresh bus.
  - Sebelum: setiap publish refresh bisa langsung invalidate query sendiri.
  - Sesudah: publish refresh di tick yang sama digabung per `QueryClient`, scope di-union, dan `refetchType` digabung ke level paling kuat yang dibutuhkan.
- Mengurangi satu surface compatibility yang tidak lagi dipakai consumer utama.
  - `useObjectListLogic()` tidak lagi mengekspor setter `setSelectedObjectFolderPath`; object selection tetap lewat runtime machine.
- Menambahkan operation test untuk runtime refresh bus.
  - Menguji batching scope dan merge `refetchType`.
  - Menguji descriptor kosong tidak memicu invalidate.
- Menambahkan audit test arsitektur untuk jalur utama `mods`.
  - Memastikan consumer runtime tidak memakai `publishRuntimeEvents(...)` langsung.
  - Memastikan consumer runtime tidak memakai `refreshObjectListQueries(...)`.
  - Memastikan helper `runtimeSelection` lama tidak direferensikan lagi.

## Impacted Files

- `src/features/runtime-sync/queryRefresh.ts` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/features/runtime-sync/queryRefresh.test.ts` (added)
- `src/features/workspace-runtime/runtimeArchitecture.audit.test.ts` (added)

## Goal

Runtime `mods` sekarang punya refresh bus yang lebih efisien saat burst mutation, dan repo punya guard test yang menjaga agar consumer utama tetap berada di arsitektur machine + descriptor + bus yang baru.

## Impact

- Refresh query runtime lebih hemat saat beberapa publish terjadi dalam tick yang sama.
- Surface utama `mods` makin sulit regress ke setter/invalidation legacy.
- Tidak ada perubahan schema atau API backend.

## Notes

- Scope audit tetap khusus runtime `mods`; flow non-`mods` seperti settings/collections tidak ikut diawasi oleh test ini.
