# Mods Manager: consistent file-lock recovery

Status: implemented and verified at the scoped Windows boundary, 2026-10-10. Repeated manual failures remain in the same overlay until success or Cancel. See [validation](validation.md) for executed checks and remaining attribution limits.

## Outcome and scope

Use one lock-error policy for Mods Manager structural mutations: mod/object toggle, rename, move, single/bulk soft delete, and the shared collection/import steps that execute those mutations. Preserve the existing MutationCoordinator, journal, filesystem identity proofs, no-overwrite publication, and disk reconciliation.

Provide two automatic retries, then the existing application-level overlay with manual Retry, current process information, and guarded close/terminate actions. Process control is always an explicit user action. An unidentified process remains a supported recovery outcome.

File writes used by Mods Manager, such as metadata and INI publication, must preserve the same structured lock diagnostics at the failed filesystem step. They must not replay a preceding successful structural mutation. This does not authorize a repository-wide rewrite of filesystem operations.

No new database schema, kernel driver, external handle-scanning executable, elevated helper, or general-purpose process killer is required.

## Verified gaps

| Area              | Current behavior                                                                | Required correction                                                           |
| ----------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Toggle            | Three very short delays: 15/40/80 ms; OS errors 32/33 are classified separately | Shared two-retry policy                                                       |
| Rename            | No bounded retry; all PermissionDenied errors become FileInUse/PathBusy         | Preserve raw OS errors and distinguish permissions from lock evidence         |
| Trash quarantine  | Rename/rollback return generic Io errors                                        | Preserve structured lock failures through command/results                     |
| Process discovery | Registers the folder itself with Restart Manager; retains only display names    | Register files, retain process identities, expose incomplete/failed discovery |
| Retry dialog      | Fires a void callback and immediately closes                                    | Await the result; keep recovery visible while pending/blocked                 |
| Bulk results      | Failures can be returned in a successful IPC result and rendered as text        | Carry structured per-item failures into the same recovery overlay             |
| Replay            | Callback repeats original variables, sometimes with a new operation ID          | Revalidate the original target identities and commit/rollback status          |

Microsoft documents that RmGetList returns ERROR_ACCESS_DENIED when a registered resource is a directory. The present folder registration cannot reliably provide process names. Restart Manager reports applications using files; that report alone does not prove which handle denied a folder rename.

## Retry policy

| Attempt       | Delay before attempt | Behavior                                                             |
| ------------- | -------------------- | -------------------------------------------------------------------- |
| Initial       | None                 | Execute the validated filesystem step                                |
| Retry 1       | 250 ms               | Revalidate ownership, root/parent and destination; try the same step |
| Retry 2       | 2 seconds            | Revalidate again; final automatic attempt                            |
| Still blocked | No automatic loop    | Settle rollback/recovery, release operation lease, show overlay      |

Exactly three total forward attempts at a blocked step. The delays are named constants, initially fixed rather than configurable settings. A normal successful action gains no delay.

- Retry Windows ERROR_SHARING_VIOLATION (32) and ERROR_LOCK_VIOLATION (33). Do not infer a lock from ErrorKind::PermissionDenied alone.
- ERROR_ACCESS_DENIED (5) remains distinct. Native tests must establish the raw errors for directory handles and locked descendants. If only ambiguous access denial is available, show permission/possible-use guidance rather than assert a process lock; disable unsafe process actions without valid attribution.
- Collision, missing source, changed identity, invalid namespace, invalid input, insufficient space and permissions errors do not enter the transient-lock loop.
- Share the forward retry budget at the operation boundary. Nested helpers must not each add another retry loop; a large batch must not accumulate multi-second waits per item.
- Stop a transactional batch at the first exhausted blocking step. Compensate only already-applied steps in reverse order. Compensation has its own bounded budget and preserves journal evidence when it cannot complete.
- Use async delays for async orchestration. Synchronous Windows operations and discovery belong in the existing blocking execution facility; no multi-second sleep on a Tokio/UI thread.
- Cancellation is honored at safe boundaries. Do not cancel by abandoning an in-flight native mutation or prematurely dropping its journal/lease.
- Keep the mutation lease during an in-progress transactional sequence, including bounded retries and compensation. Release it before user interaction or process discovery. App-operation contention remains separate from an external lock.

## Filesystem and replay safety

The shared retry helper retries one demonstrably uncommitted filesystem step. Its caller supplies the ownership/namespace validation used before each attempt. It must not capture a fresh identity after a delay and treat a replacement directory as the original source.

Use no-overwrite operations for forward and reverse rename. Preserve tracked cross-volume moves: failed publication/source cleanup must retain their existing committed/unchanged classification. Never wrap the entire copy-publish-delete fallback in a blind retry.

Manual Retry reacquires coordinator admission, validates the frozen game/root/target context, and resolves stable IDs against the original physical identity. If the game, configured root or source identity changed, expire the recovery action and request a refreshed selection. A same-named replacement must never inherit the old action.

For transactional batches, whole-operation replay is permitted only after complete rollback. For commands that intentionally return partial results, retry only explicitly failed, unchanged items and preserve committed successes. Committed quarantine cleanup resumes cleanup of that journal-owned quarantine; it does not delete the original path again. A committed mutation with projection/runtime-sync warnings retries synchronization, not filesystem mutation.

If rollback fails, expose recovery-required state and retain the journal. Do not offer a generic replay button until the recovery path establishes a safe state. Startup recovery is headless: it can retain evidence and warn, but never terminate processes or wait for UI.

Verified rollback binds the complete original transaction scope, including selected move no-ops; proposed replay writes must belong to that scope. Ancestor rewrites are reversed when freezing source paths.

Watcher suppression remains scoped to owned mutation paths. Delays must not discard unrelated external events. Revalidation precedes every attempt; post-operation reconciliation handles dirty/unproven observations, including cancellation and failed compensation.

## Process discovery and contracts

Replace Vec<String> with a typed diagnostic report. The report distinguishes completed discovery with no result, partial enumeration, access denied, unavailable API and API failure. Empty results never mean the folder is unlocked.

For a failed file operation, register the exact affected file. For a folder operation, enumerate regular descendant files under the validated physical root, prioritizing an exact failed child if available. Do not register directories, follow reparse points, read asset bytes or scan unrelated locations.

Start with bounded enumeration: 1,024 files, registration batches of 128, and a cooperative 1-second enumeration/discovery budget. These are implementation defaults to validate on representative large folders. A native Restart Manager call is not guaranteed to obey that deadline; execute off the async/UI thread and retain its session ownership until it finishes. Record truncation/inaccessible descendants and do not claim complete discovery.

Deduplicate by PID plus process creation time, not name. Preserve display name, executable basename when readable, Restart Manager application type, session, service information, process start time and action availability/reason. Represent creation time losslessly across IPC, not as a JavaScript floating-point FILETIME value.

Directory-only handles, inaccessible files, kernel/antivirus activity and files outside the bounded enumeration may remain unattributed. Show an explicit unknown/partial message and manual Retry/Task Manager guidance. Do not add undocumented system-handle enumeration as a fallback.

Extend structured AppError/CollectionError and per-item result errors without parsing localized error strings. Retain FileInUse/PathBusy compatibility during migration; add typed details only where callers need them. UI must distinguish transient lock, ambiguous access denial, app-operation contention, stale target and recovery-required state.

Backend issues a recovery context tied to the failed operation, authorized root/target identities and discovered process identities. UI receives opaque context/process references. Do not expose terminate(pid) or accept arbitrary frontend paths as authority. The bounded indexed in-memory store supports one maximum 10,000-target batch plus 64 additional contexts, deduplicates identical live authority and does not evict live entries. Process references expire after ten minutes; an open dialog can renew a retained context after all original proofs pass. Dismissed or evicted contexts cannot revive. This keeps manual Retry available after a long idle period while rejecting changed game/root/targets.

Proposed narrow IPC surface, finalized against existing module APIs:

- Refresh diagnostics for an existing recovery context.
- Request graceful close or explicit forced termination of one process reference in that context.

Existing mutation commands remain the execution path for manual Retry, with original identity/recovery validation added where necessary. Avoid a second generic mutation dispatcher. System owns OS process-control orchestration through its public API; platform owns Windows calls; library/workspace adapters own their relevant IPC context. Register new commands, explicit permissions and generated bindings together.

## Close/terminate policy

1. Show applications reported using the affected files, with name/PID and whether process actions are available. Describe these as reported file users rather than guaranteed blockers.
2. Offer **Close application** for validated normal user applications with an eligible window. Use a bounded close request; the application may show its own save confirmation or refuse to exit. Lack of a window disables graceful close without inventing a shutdown command.
3. Offer **Force terminate** as a separate danger action for eligible processes. Require a confirmation naming the application/PID and warning that unsaved application/game state can be lost. No automatic escalation from graceful close to force.
4. Before either action, refresh affected-resource attribution; validate target/root identities; open the exact process; compare creation time; verify same user/session and noncritical status using supported Windows APIs. Hold that validated process handle through the action. If any safety check cannot be established, deny the action.
5. Block System/Idle/critical processes, services, protected/security processes, Windows shell processes, EMMM and its application runtime processes. Enforce policy in Rust, not just a disabled UI button. Use OS identity/type/token evidence; a filename denylist alone is insufficient. Do not elevate or enable debug privileges to bypass denial.
6. Terminate only the explicitly selected process, not all matching executable names or its process tree. Repeated requests for the same action are serialized/idempotent; an already-exited identity is reported as such, while a reused PID is rejected.
7. Wait for exit with a bounded wait (initially 3 seconds). Successful TerminateProcess initiates asynchronous exit and does not certify released handles. On timeout, keep the dialog open with truthful status. Refresh diagnostics after close/exit.
8. Keep **Retry operation** explicit after process actions. Only a successful, freshly validated filesystem operation proves it can proceed; an empty process list or successful termination call does not.

Provide Open Task Manager/manual instructions for disallowed or unidentified holders. Explorer guidance asks the user to close related windows/preview panes; forced termination of the Windows shell is outside this feature.

## Overlay behavior

Reuse FileInUseDialog in the application shell and its native dialog/focus infrastructure. Avoid a second independent modal host.

Display operation label, folder/full path, attempt count, current diagnostic status and applications. An empty process list renders a clear unknown message. Actions: Retry operation, Refresh processes, eligible per-process Close/Force terminate, and Cancel.

Use a small explicit state union: inspecting, blocked, retrying, controlling-process, recovery-required. While retrying or controlling a process, disable duplicate submissions and keep progress visible. Retry awaits its result; success closes the owning dialog, another lock refreshes it, and a different failure renders its own actionable message. An old completion must not close a newer dialog.

Cancel while idle dismisses the pending recovery intent; it cannot schedule later mutations. During execution, cancellation follows the coordinator's safe boundaries. Expire stale recovery controls when the active game/root/selection changes. Preserve focus, Escape behavior, keyboard navigation, dialog stacking and screen-reader status. All text, labels, confirmations and reasons use EN/ID/ZH localization and existing semantic tokens.

## Implementation sequence

1. **Characterize Windows failures:** owned fixtures with delete-denying handles on the directory and a nested file; sharing-compatible handle; ACL-denied source; no-overwrite collisions. Record exact Win32 errors. Reproduce directory registration failure and current dialog/replay gaps.
2. **Shared retry/error handling:** platform filesystem retry/classification policy, injectable clock/operation for deterministic tests; thread through toggle, rename, trash, runtime/object paths and affected metadata publication. Preserve tracked move/rollback outcomes.
3. **Discovery and safe control:** typed Restart Manager report, bounded file enumeration, stable process identities, recovery-context authority, graceful-close/force policy and narrow IPC. Validate rejection paths before any UI action can call termination.
4. **UI and caller integration:** awaitable retry, structured partial-result handling, stable-target replay, progress/process controls, localization. Migrate core/bulk/object/collection callers and import result paths used by Mods Manager; audit remaining string-only conversions.
5. **Native verification and records:** freshly built isolated Windows app, full behavior matrix, architecture/bindings/permission checks, relevant builds/lint/tests, backend architecture map and concise implementation history. No tests terminate a user's application.

Likely touchpoints: platform/fs/{locking,rename,file_utils}; shared/errors; mutation coordinator/task/journal replay boundaries; library core_ops, bulk and trash; runtime/object/import mutation steps; system process-control adapter; FileInUseDialog; workspace-runtime actions/state; related mutation hooks; AppError decoding; three common locale files; IPC registration/permissions/generated bindings. Exact files remain scoped to verified callers rather than replacing every filesystem call globally.

## Acceptance and validation

| Scenario                                              | Expected result                                                                        |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Unlocked folder / sharing-compatible open handle      | Immediate success, no artificial waits                                                 |
| Lock clears before first retry                        | Success on attempt 2 after 250 ms                                                      |
| Lock clears before last retry                         | Success on attempt 3 after the additional 2-second delay                               |
| Persistent sharing/lock violation                     | Three forward attempts, then visible recovery overlay                                  |
| Folder-handle vs nested-file lock                     | Correct observed OS classification; supported file discovery or truthful unknown state |
| ACL denied / collision / missing source               | Distinct error, no misleading transient-lock loop                                      |
| Source/root/parent replaced during delay or dialog    | Original action rejects; replacement untouched                                         |
| Destination created during delay                      | No overwrite; actionable conflict                                                      |
| Transactional batch fails after earlier changes       | Compensation and journal status; whole replay only if rollback completed               |
| Partial-result bulk operation                         | Overlay captures structured blocked items; successes never replayed                    |
| Compensation itself locked                            | Bounded compensation, recovery-required state, retained evidence                       |
| Cross-volume fallback fails after publication         | Owned staging/commit state preserved; no blind full replay                             |
| Trash finalized/committed but cleanup or sync blocked | Resume cleanup/sync only; original path not deleted again                              |
| Enumeration cap/reparse/inaccessible descendant       | Bounded, contained discovery with explicit incomplete status                           |
| Multiple same-name applications                       | Separate PID/start-time identities and single selected target                          |
| Holder exits / PID reused / context forged or expired | Already-exited handling or rejection; unrelated process untouched                      |
| System/service/shell/protected/self/other-user holder | Terminate denied in backend with readable reason                                       |
| Graceful close refused or save prompt shown           | No forced escalation; dialog stays usable                                              |
| Owned fixture force-terminated                        | Explicit confirm, exact identity action, bounded exit wait, manual Retry               |
| Double Retry / process-action spam                    | One active request; no duplicate mutation or termination                               |
| Cancel / game switch / stale dialog completion        | No delayed stale action; current dialog/state preserved                                |
| External filesystem events during delay               | Unrelated observations preserved; final disk/DB/collection state reconciled            |
| Backend lock contended by another EMMM action         | App-busy feedback, no external-process termination controls                            |
| Process discovery fails                               | Original filesystem failure remains actionable; UI does not claim unlocked             |
| Overlay keyboard/fullscreen/localization              | Focus/stacking/disabled states and EN/ID/ZH copy work in native app                    |

Use deterministic unit tests for retry counts/timing, error mapping, context validation and replay boundaries; component tests for dialog outcomes; real owned Windows handles/processes for OS integration. Native WDIO validates actual overlay/IPC/disk behavior using private app-data and owned fixtures. Browser demo fixtures cannot certify this feature.

Completion gates: relevant Rust tests, scoped strict Clippy/rustfmt, frontend typecheck/build/lint/architecture tests, generated-command allowlist test, and the native cases above. Document any unsupported attribution or blocked native case explicitly; do not mark full coverage based on mocks.

## Decision rationale and sources

The design uses existing transaction/recovery mechanisms and supported Windows APIs. Bounded retries cover transient locks; manual recovery handles persistent locks without turning ordinary file operations into a background process killer. The senior-architect review guided explicit ownership boundaries and reuse of the existing coordinator/dialog rather than adding another workflow engine.

- [RmGetList: directory resources return access denied](https://learn.microsoft.com/en-us/windows/win32/api/restartmanager/nf-restartmanager-rmgetlist)
- [RM_UNIQUE_PROCESS: PID plus creation time](https://learn.microsoft.com/en-us/windows/win32/api/restartmanager/ns-restartmanager-rm_unique_process)
- [WM_CLOSE: application can request save/close confirmation](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-close)
- [TerminateProcess: unconditional, asynchronous exit and pending I/O](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess)
- [IsProcessCritical](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-isprocesscritical)
- [Critical system services](https://learn.microsoft.com/en-us/windows/win32/rstmgr/critical-system-services)
- [OpenProcess: access rights and handle ownership](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess)
