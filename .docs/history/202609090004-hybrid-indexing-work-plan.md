# Hybrid indexing work plan

## Context

Global onboarding progress needs to reflect both many small files and fewer large files. Root counts alone overstate progress for small roots and understate it for roots with substantial metadata.

## Changes

- Added an onboarding-only filesystem planning command that records each mod root's file count, total bytes, and hybrid work units.
- Hybrid work units add a fixed metadata cost per file to its byte size, so file-heavy folders remain visible in global progress.
- Updated the indexing-progress calculation to weight finished roots and games by that plan while retaining discovery, scanning, projection, and finalization stages.
- Preserved the existing reconcile scanner; the planner only reads directory entries and file metadata, never file contents.

## Impacted files

- `src-tauri/src/modules/reconciliation/application/disk_reconcile/{types,work_plan}.rs`
- `src-tauri/src/modules/reconciliation/adapters/tauri/disk_reconcile_cmds.rs`
- `src-tauri/src/lib.rs`, `src-tauri/permissions/app-commands.toml`, and generated Tauri bindings
- `src/pages/onboarding/{WelcomeScreen.tsx,hooks/useOnboardingDiskProgress.ts,utils/indexingProgress.ts}` and their tests

## Goal

Provide a global onboarding percentage that accounts for all games, all indexing stages, file count overhead, and file-size differences without reading file contents.
