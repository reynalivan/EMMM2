# Phase D & E: Frontend FSD Cleanup & Backend Tauri Commands Finalization

## Context
Menjalankan tahap sisa dari blueprint relocation.md. Folder src/types di frontend masih menjadi dumping ground proxy yang melanggar aturan arsitektur, dan src-tauri/src/commands/ masih menyimpan mayoritas API Tauri yang belum direlokasi ke modul vertikalnya.

## Changes
- Menghapus folder src/types/ dan src/entities/common/ sepenuhnya.
- Melakukan rewrite ratusan import di seluruh .ts dan .tsx agar langsung menunjuk ke domain spesifiknya (e.g., @/entities/game/model/game).
- Merelokasi seluruh *Tauri commands* tersisa ke modules/*/adapters/inbound/.
- Menghapus proxy src-tauri/src/commands/ dan mendaftarkan ulang modul ke emmm_collect_commands!.

## Impacted Files
- src/types/* (removed)
- src/entities/common/* (moved to entities/<domain>/model/*)
- 171 files di frontend (modified imports)
- src-tauri/src/commands/* (moved to modules/*/adapters/inbound/*)
- src-tauri/src/lib.rs (modified macro collection)
- src-tauri/src/modules/mod.rs (modified)

## Goal
Frontend sekarang bersih dari *shim types* layer global (pure FSD entities). Seluruh Tauri Commands di backend kini terdistribusi secara Modular Monolith.

## Impact
- FSD lebih ketat di Frontend.
- Tidak ada file yang tertinggal di src-tauri/src/commands/.
- Perbaikan import secara otomatis mengurangi *technical debt*.
