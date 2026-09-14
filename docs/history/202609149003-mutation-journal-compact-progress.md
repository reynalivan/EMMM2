# Compact mutation journal progress

## Context

The P1 baseline showed that rewriting the complete operation plan after every filesystem step was the dominant source of mutation-journal write amplification. Recovery durability could not be relaxed: every transition still needs an atomic durable checkpoint before the corresponding next mutation action.

## Changes

- Kept the full, checksummed `mutation-journal.json` snapshot as the durable immutable plan and terminal history record.
- Added a compact, checksummed, revisioned active-state sidecar per UUID operation. It stores only operation status, database projection status, packed two-bit step statuses, and error detail.
- Kept one atomic persistence operation per transition. A terminal transition writes the full snapshot before removing its active-state sidecar; a stale sidecar is ignored when the full record is terminal.
- Added sidecar artifact recovery: the canonical sidecar wins; if it is absent, startup restores the newest valid replacement artifact and rejects conflicting or invalid candidates.
- Added coverage for snapshot immutability while active, terminal cleanup, and recovery from a missing canonical active-state file.

## Benchmark

Seven post-warm-up samples per matrix cell, measured as logical serialized journal bytes. The number of durable transitions is unchanged.

| History | Steps | Commit p50 / bytes | Partial rollback p50 / bytes |
| --- | ---: | ---: | ---: |
| 1 | 500 | 1,080 ms / 563,542 B | 1,109 ms / 411,985 B |
| 1 | 1,000 | 2,348 ms / 1,376,045 B | 2,328 ms / 1,073,238 B |
| 64 | 500 | 1,064 ms / 612,862 B | 1,080 ms / 436,645 B |
| 64 | 1,000 | 2,543 ms / 1,425,365 B | 2,422 ms / 1,097,898 B |
| 256 | 500 | 1,194 ms / 764,638 B | 1,149 ms / 512,533 B |
| 256 | 1,000 | 2,519 ms / 1,577,141 B | 2,509 ms / 1,173,786 B |

For the comparable successful-commit cases, P1 wrote 76.8–406.5 MB while this design writes 0.56–1.58 MB: a 99%+ reduction in serialized bytes. The p50 also fell from 4.2–20.2 seconds to 1.1–2.5 seconds. Tail latency remains storage-dependent and should continue to be observed in real bulk workloads.

## Safety

- Atomic writes, path validation, mutation guards, rollback, database projection ordering, watcher suppression, and reconcile behavior are unchanged.
- The plan is durable before the coordinator can perform a disk mutation.
- An active state is durable before the coordinator advances to its next step.
- A crash after the terminal snapshot but before sidecar deletion is safe because the terminal snapshot is authoritative.

## Impacted Files

- `src-tauri/src/modules/mutation/journal.rs` (modified)
- `src-tauri/src/modules/mutation/tests.rs` (modified)
- `src-tauri/AGENT.md` (modified)
- `docs/history/202609149003-mutation-journal-compact-progress.md` (added)
