# Refining Storage Optimizer UI

## Context

Modernizing the duplicate management interface to support multi-member groups and improve filtering precision.

## Changes

- **Targeted Keep Model**: Transitioned from binary A/B resolution to a flexible dropdown system where users select one member to "Keep" and others are implicitly deleted.
- **Confidence Filtering**: Relocated filtering tabs (All, High, Medium, Low) from the report component to the main page header for better visibility and state persistence.
- **Mass-Apply Logic**: Updated resolution request mapping to support `targetPath` for N-member group resolution.
- **UI Refinement**: Updated `DuplicateTable`, `DuplicateReport`, and `ResolutionModal` to support the new multi-member logic and visual indicators.

## Impacted Files

- `src/types/scanner.ts` (modified)
- `.docs/requirements/req-32-dedup-scanner.md` (modified)
- `src/features/scanner/StorageOptimizerPage.tsx` (modified)
- `src/features/scanner/DedupFeature.tsx` (modified)
- `src/features/scanner/components/DuplicateReport.tsx` (modified)
- `src/features/scanner/components/DuplicateTable.tsx` (modified)
- `src/features/scanner/components/ResolutionModal.tsx` (modified)
- `src/features/scanner/components/DuplicateTable.test.tsx` (modified)
- `src/features/scanner/components/DuplicateReport.test.tsx` (modified)

## Goal

Provide a robust, scalable duplicate resolution workflow that handles complex multi-folder scenarios with high confidence filtering.

## Impact

- Better usability for complex duplicate scenarios (3+ folders).
- Improved visibility of scanning confidence levels.
- Full type safety for the new resolution model.
