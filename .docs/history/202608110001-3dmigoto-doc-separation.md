# Separate 3DMigoto Guide and Gap Status

## Context

The 3DMigoto knowledge base mixed durable technical guidance with completed remediation status, making it harder for AI and maintainers to distinguish current behavior from historical work.

## Changes

- Reframed the main knowledge base around runtime concepts, how-to guidance, current EMMM flows, and regression checks.
- Removed the completed gap matrix and obsolete remediation roadmap from the main guide.
- Added a dedicated status document for T1–T10, all 23 audited gaps, verification evidence, and remaining runtime validation boundaries.
- Updated cross-links so the guide and status report remain discoverable from each other.

## Impacted Files

- `.docs/3dmigoto_context_knowledge.md` (modified)
- `.docs/3dmigoto_gap_status.md` (added)
- `.docs/history/202608110001-3dmigoto-doc-separation.md` (added)

## Goal

Provide a stable how-to knowledge base for AI-assisted 3DMigoto work while tracking implementation status and unresolved validation separately.

## Impact

- Documentation-only change; no runtime behavior, performance, or public API changes.
- Remaining manual smoke tests and EFMI capability limits are now explicit without cluttering the technical guide.
