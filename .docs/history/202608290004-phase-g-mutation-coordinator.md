# Phase G: Backend Mutation Coordinator

## Context
Menjalankan Phase G dari `.docs/relocation.md`. EMMM2 menggunakan kombinasi Disk dan SQLite sebagai truth, sehingga proses mutasi file destruktif harus ditangani secara terpusat untuk menghindari race condition dan memungkinkan recovery. `OperationLock` lama yang tersebar perlu digantikan oleh `MutationCoordinator` yang nantinya akan mendukung workflow recoverable mutation (Operation Journal).

## Changes
- Membuat `src-tauri/src/app/runtime/mutation_coordinator.rs` sebagai pembungkus `OperationLock`.
- Membuat kerangka dasar (mock) untuk `OperationJournal` (`operation_journal.rs`), `RecoveryRunner` (`recovery_runner.rs`), dan `TaskRegistry` (`task_registry.rs`) di dalam layer `app/runtime`.
- Merefactor Tauri commands di layer inbound adapters (seperti `workspace_cmds.rs`, `disk_reconcile_cmds.rs`, dll) agar mengambil lock dari `MutationCoordinator` (`app.state::<MutationCoordinator>()`) alih-alih `OperationLock` langsung.
- Mendaftarkan State `MutationCoordinator` dan `OperationJournal` di `src-tauri/src/lib.rs`.

## Impacted Files
- `src-tauri/src/app/mod.rs` (modified)
- `src-tauri/src/app/runtime/mod.rs` (added)
- `src-tauri/src/app/runtime/mutation_coordinator.rs` (added)
- `src-tauri/src/app/runtime/operation_journal.rs` (added)
- `src-tauri/src/app/runtime/recovery_runner.rs` (added)
- `src-tauri/src/app/runtime/task_registry.rs` (added)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/modules/storage_optimizer/adapters/inbound/tauri.rs` (modified)
- `src-tauri/src/modules/workspace/adapters/inbound/disk_reconcile_cmds.rs` (modified)
- `src-tauri/src/modules/workspace/adapters/inbound/tauri.rs` (modified)
- `src-tauri/src/modules/workspace/adapters/inbound/workspace_cmds.rs` (modified)

## Goal
Menyediakan infrastruktur dasar untuk Recoverable Mutation Workflow tanpa merusak fungsionalitas locking yang sudah berjalan stabil.

## Impact
- Dependensi `OperationLock` di level framework (Tauri State) kini diarahkan ke `MutationCoordinator`.
- `OperationLock` masih digunakan secara internal di `MutationCoordinator` untuk menjaga backward compatibility terhadap `OpGuard` yang di-pass ke domain services, sehingga tidak perlu merombak ribuan baris kode yang sudah mapan seketika.
- Arsitektur kini siap di-upgrade untuk mencatat log mutasi (Pending -> Executed -> Completed) pada iterasi-iterasi selanjutnya.

## Notes
- `OperationJournal` saat ini menggunakan *in-memory mock*. Implementasi real (ke SQLite atau file disk) dapat dilakukan secara bertahap saat *recoverable mutation pipeline* diimplementasikan penuh.
- `cargo test` menghasilkan 865 lulus dan 0 gagal.

