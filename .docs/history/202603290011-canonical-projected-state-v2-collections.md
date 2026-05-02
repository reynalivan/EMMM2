# Canonical projected state for collections/corridor v2

## Context

Collections v2 masih menyimpan dan mempreview state dari raw mod rows + tree container, sehingga apply/switch/topbar bisa drift, preview terlalu noisy, missing mod masih hard-fail, dan reset setup belum membersihkan runtime/cache state baru.

## Changes

- Mengubah hot path collections/corridor ke snapshot canonical berbasis projected state:
  - object states + visible active roots only
  - signature strict dihitung dari projected state, bukan raw member rows
- `get_corridor_state`, `get_collection_preview`, `preview_apply_collection`, dan `preview_corridor_switch` sekarang memakai projected state yang sama.
- Preview tree disederhanakan menjadi object -> visible main roots tanpa container/internal subtree.
- Apply pipeline sekarang:
  - diff dari projected root keys
  - treat missing mods sebagai warning
  - expose progress state sederhana
  - return final state name/mode setelah apply
- Reset database dibetulkan untuk membersihkan corridor/runtime/projection tables v2.

## Impacted Files

- Backend domain/services:
  - `src-tauri/src/domain/collection.rs` (modified)
  - `src-tauri/src/domain/corridor.rs` (modified)
  - `src-tauri/src/services/mod.rs` (modified)
  - `src-tauri/src/services/projected_state_service.rs` (added)
  - `src-tauri/src/services/apply_progress_service.rs` (added)
  - `src-tauri/src/services/collection_service.rs` (modified)
  - `src-tauri/src/services/corridor_service.rs` (modified)
- Backend pipeline/repo/commands:
  - `src-tauri/src/pipeline/apply_pipeline.rs` (modified)
  - `src-tauri/src/pipeline/steps/batch_rename.rs` (modified)
  - `src-tauri/src/pipeline/steps/compute_diff.rs` (modified)
  - `src-tauri/src/pipeline/steps/resolve_current_state.rs` (modified)
  - `src-tauri/src/pipeline/steps/resolve_target.rs` (modified)
  - `src-tauri/src/pipeline/steps/update_corridor.rs` (modified)
  - `src-tauri/src/pipeline/steps/validate_paths.rs` (modified)
  - `src-tauri/src/repo/corridor_repo.rs` (modified)
  - `src-tauri/src/repo/settings_repo.rs` (modified)
  - `src-tauri/src/commands/collections/cmds.rs` (modified)
  - `src-tauri/src/lib.rs` (modified)
  - `src-tauri/permissions/app-commands.toml` (modified)
- Frontend:
  - `src/types/collection.ts` (modified)
  - `src/lib/bindings.ts` (modified)
  - `src/features/collections/queryKeys.ts` (modified)
  - `src/features/collections/hooks/useCollections.ts` (modified)
  - `src/features/collections/hooks/useCorridorSwitch.ts` (modified)
  - `src/features/collections/components/ApplyCollectionModal.tsx` (modified)
  - `src/features/collections/components/CollectionPreviewPanel.tsx` (modified)
  - `src/features/collections/types.ts` (modified)
  - `src/features/safe-mode/ModeSwitchConfirmModal.tsx` (modified)
  - `src/features/collections/hooks/useCollections.test.ts` (modified)

## Goal

Collections, apply preview, switch preview, dan topbar sekarang membaca satu model projected state yang sama, lebih sederhana, lebih strict, dan lebih dekat ke apa yang user anggap sebagai “state collection aktif”.

## Impact

- Active collection match lebih stabil karena signature strict dihitung dari visible roots + object states.
- Apply tidak lagi berhenti karena missing target roots; warning tetap dikirim.
- Preview menjadi lebih bersih dan lebih sedikit noise.
- Ada perubahan kontrak payload FE/BE untuk snapshot/preview/apply result.

## Notes

- Legacy split tables masih dipertahankan sementara untuk compatibility/backfill, tetapi hot path runtime tidak lagi bergantung pada join-heavy reads.
- Rust test binaries masih tidak stabil dieksekusi di environment Windows ini, jadi validasi Rust pass ini menggunakan `cargo check` dan `cargo test --no-run`.
