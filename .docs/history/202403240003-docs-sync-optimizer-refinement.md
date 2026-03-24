# 202403240003 - Documentation Sync & Storage Optimizer Refinement

## Context

After implementing FolderGrid bulk selection and refining the Storage Optimizer (Ignored Pairs recovery) and KeyViewer (rendering pipeline), the technical documentation needed synchronization to reflect the actual architecture and UI decisions.

## Changes

- **FolderGrid UI**: Updated `req-12` and `req-14` to specify checkbox placement (top-right corner), scale-in animations, and standard keyboard shortcuts (`Escape`, `Delete`, `Ctrl+A`).
- **Storage Optimizer**: Updated `req-32` to include the `IgnoredPairsModal` for whitelist recovery and the new header controls (Start/Stop scan).
- **Dynamic KeyViewer**: Updated `req-43` to document the unified `help.ini` notification pipeline, replacing the resource-selection logic with a single-tree arbitration structure for better rendering stability.
- **TRD**: Added Storage Optimizer to the core functional pipelines in the main Technical Requirements Document.

## Impacted Files

- `.docs/trd.md` (modified)
- `.docs/requirements/req-12-folder-grid-ui.md` (modified)
- `.docs/requirements/req-14-bulk-operations.md` (modified)
- `.docs/requirements/req-32-dedup-scanner.md` (modified)
- `.docs/requirements/req-43-dynamic-keyviewer.md` (modified)

## Goal

Establish a single source of truth in the `.docs/` directory that matches the current production implementation of the mentioned features.

## Impact

- Clearer technical guidance for future development on these components.
- Accurate AC (Acceptance Criteria) for verification and testing.
