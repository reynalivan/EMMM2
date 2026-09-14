# Project setup script

## Context

Pengaturan environment proyek sebelumnya hanya tercantum sebagai perintah manual di README.

## Changes

- Menambahkan `setup.ps1` untuk memvalidasi Node 20+, Corepack, Cargo, dan Rust.
- Script memasang dependency frontend sesuai lockfile serta mengambil crate Cargo tanpa menjalankan aplikasi.
- README mengarahkan instalasi pengembangan ke script tersebut.

## Impacted Files

- `setup.ps1` (added)
- `README.md` (modified)

## Goal

Setup development dapat dijalankan konsisten dari satu perintah PowerShell.

## Impact

`setup.ps1` mengunduh dependency saat dijalankan; `-SkipFrontend` atau `-SkipRust` tersedia untuk setup parsial.
