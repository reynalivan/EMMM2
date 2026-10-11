# Refine startup performance architecture plan

## Context

User requested a mature best-practice plan and permitted refactoring when it
simplifies ownership and improves stability.

## Changes

- Define watcher-session lifetime independently of activation publication.
- Sequence verified watcher reuse, directory-cache experiment, pending recovery
  join/progressive shell and measured projection improvements.
- Compare NoCache, background full cache, native extended notifications and
  persistent caching against the pinned dependencies and existing contracts.
- Specify race/rename/overflow/durability tests, performance gates and rollback.

## Impacted Files

- `docs/plans/startup-performance/implementation_plan.md` (modified)
- This history file (added)

## Goal and Impact

Provide an implementable sequence with one recovery owner and preserved native
proofs. No application code, dependencies, installed binary or user data changed.
Performance targets are acceptance goals, not measured results.

## Validation

Architecture and plan review completed; clarify that unchanged reconciliation
must avoid identity staging/key-value changes, not necessarily all SQL UPDATEs.
Pinned library source, Context7 and primary Microsoft/Tokio documentation reviewed.
Markdown formatting and diff whitespace checks passed.
