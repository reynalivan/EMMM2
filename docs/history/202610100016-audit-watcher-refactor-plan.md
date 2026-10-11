# Audit watcher consolidation and Tauri separation plan

## Context

User requested a detailed, audited follow-up plan after four watcher bug fixes.
Planning only; preserve all application source and shared dirty changes.

## Changes

- Correct six design gaps: rename endpoints, explicit I/O versus pure policy,
  distinct publication lease windows, receiver/onboarding lifetime, real session
  versus binding evidence, and directed facade/adapter migration.
- Add concrete target ownership, caller inventory, staged gates, rollback rules
  and missing integration/race acceptance cases. No broad refactor implemented.

## Impacted Files

- `docs/plans/file-watcher-stability/refactor_plan.md` (modified).
- `docs/plans/file-watcher-stability/plan_audit.md` (added).
- `docs/plans/file-watcher-stability/plan_audit_source_snapshot.json` (added).
- This history file (added).

## Goal and Impact

Plan suitable for approval with explicit evidence and safeguards. No new queue,
service, dependency, IPC/schema, watcher instance, native launch or source change.

## Validation

Independent read-only architecture review plus caller/test contract mapping.
Documentation format, local links/JSON and source-integrity checks only; previous
build/test counts are historical, not rerun or certification of the planned work.
