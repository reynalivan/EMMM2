# Fix Mod Inbox processed containment

## Context

`Processed` dapat mengarah ke lokasi di luar Mod Inbox melalui symlink atau junction.

## Changes

- Validasi canonical `Processed` sebagai child langsung dari canonical inbox sebelum recycle.
- Terapkan validasi yang sama sebelum finalisasi source Ready to Move.
- Tambahkan regresi test untuk canonical processed root yang berada di luar inbox.

## Impacted Files

- `src-tauri/src/modules/ingestion/application/import_batch/mod_inbox.rs` (modified)
- `src-tauri/src/modules/mutation/application/workspace_mutation/import_commit.rs` (modified)
- `src-tauri/src/modules/mutation/application/workspace_mutation/tests.rs` (modified)

## Goal

Processed source tidak dapat mengarah ke dan menghapus file di luar Mod Inbox.

## Impact

Import dengan folder `Processed` yang tidak berada di bawah inbox kini gagal aman dengan error security.
