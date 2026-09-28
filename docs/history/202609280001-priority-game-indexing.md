# Priority game indexing with fail-closed core readiness

## Context

Indexing beberapa game menahan navigasi dan status lama dapat membolehkan operasi mod sebelum disk, DB, dan watcher sinkron.

## Changes

- Scan onboarding kini dapat dipromosikan per game, yield antarbatches root, memakai checkpoint root lengkap, dan hanya mengulang snapshot target yang invalid.
- Mutasi biasa ditolak sampai hasil core applied terbukti oleh authority watcher pada revisi yang sama; lease memeriksa ulang gate setelah lock. Journal yang belum diterapkan dapat dibatalkan secara aman.
- Aktivasi, background, dan startup memverifikasi kontinuitas watcher sebelum Ready; runtime/KeyViewer ditunda dari critical path core.
- Pilihan game menampilkan loading/progres authoritative, mengabaikan respons aktivasi lama, dan menunda query workspace sampai target siap.

## Impacted Files

- Reconciliation: `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs`; `src-tauri/src/modules/reconciliation/application/disk_reconcile/{disk_snapshot.rs,emit.rs,onboarding_session.rs}`; `src-tauri/src/modules/reconciliation/application/disk_reconcile/orchestrator/{entry.rs,state.rs,tests.rs}`.
- Watcher, startup, settings: `src-tauri/src/modules/workspace/application/scanner/watcher/lifecycle.rs`; `src-tauri/src/modules/workspace/adapters/tauri/workspace_cmds.rs`; `src-tauri/src/modules/system/application/app/{bootstrap.rs,post_apply.rs}`; `src-tauri/src/modules/settings/adapters/tauri/settings_cmds.rs`.
- Mutation and commands: `src-tauri/src/modules/mutation/{coordinator.rs,journal.rs,tests.rs}`; `src-tauri/src/modules/mutation/application/workspace_mutation/{import_commit.rs,object_status.rs}`; `src-tauri/src/modules/library/adapters/tauri/{conflict_cmds.rs,mod_bulk_cmds.rs,mod_core_cmds.rs,mod_meta_cmds.rs,mod_thumbnail_cmds.rs,preview_cmds.rs,thumbnail_cmds.rs,trash_cmds.rs}`; `src-tauri/src/modules/catalog/adapters/tauri/object_cmds.rs`; `src-tauri/src/modules/duplicates/adapters/tauri/tauri.rs`; `src-tauri/src/modules/automation/application/hotkeys/{cycle_preset.rs,safe_mode.rs}`.
- Frontend: `src/app/entrypoint/App.tsx`; `src/app/store/{appStore/gameSlice.ts,useAppStore.test.ts,gameActivationListenerRace.test.ts}`; `src/features/workspace-runtime/actions/useGameSwitch.ts`; `src/features/workspace-runtime/hooks/{useBackgroundIndexingStatus.ts,useBackgroundIndexingStatus.test.tsx}`; `src/widgets/app-shell/{AppShell.tsx,AppShell.test.tsx}`; `src/widgets/top-bar/{GameIndexingOverlay.tsx,GameIndexingOverlay.test.tsx,GameSelector.tsx,GameSelector.test.tsx,TopBar.tsx}`; `src/shared/i18n/locales/{en,id,zh}/layout.json`.
- Plan: `docs/plans/priority-game-indexing/implementation_plan.md`.

## Goal

Game awal/terpilih operasional hanya setelah indeks inti tervalidasi, sementara game lain berjalan di background dan pekerjaan opsional tidak menahan switch.

## Impact

Mutasi pada game belum siap kini gagal cepat; UI dapat berpindah game saat scan. Tidak ada migrasi DB atau perubahan semantik nama folder. Benchmark scanner sintetis bukan bukti latency onboarding end-to-end; native watcher/rage-click masih memerlukan uji interaktif.

## Notes

Root tunggal sangat besar belum dapat dipreempt di tengah traversal; checkpoint hanya pada root lengkap. Error gate masih memakai `AppError::Io`, belum DTO typed khusus. Acceptance yang belum dibuktikan tetap terbuka di plan.
