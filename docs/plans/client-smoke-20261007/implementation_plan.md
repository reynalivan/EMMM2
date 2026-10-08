# Fix confirmed client smoke regressions

Approved scope: user's `perbaiki bug` request following the audit. Preserve concurrent unrelated workspace edits. No release/push/installers in this change.

## Goals

- Settings cache receives the authoritative post-activation snapshot/revision without another hydration IPC or waiting for optional indexing/runtime.
- Duplicate physical Mods paths are rejected in the real form schema; same basename at different directories remains valid.
- Trust dialogs use the existing native modal lifecycle; Escape and focus restoration work.
- Address entry remains focused/editable while a typed prefix becomes a valid URL.
- Ordinary listing refresh preserves explicit selection/preview. Snapshot-bound all-matching selection remains invalidated.
- Dirty INI transitions run the existing unsaved guard before selection changes, including queued confirmation/cancellation.
- Typed error payloads remain actionable instead of `[object Object]`.
- Grid shortcuts do not intercept typing inside rename, metadata or other editable controls.
- Metadata refresh/save acknowledgements merge clean fields without overwriting newer edits, including empty descriptions.
- Rename conflicts report through the existing mutation feedback without escaping as an unhandled UI rejection.
- Opposite bulk intents survive in-flight path/revision rewrites only with complete matching filesystem identities; another physical selection remains rejected.
- Bridge the completed-toggle/stale-listing interval with one matching committed receipt and existing identity validation; retire it when listing coverage/context supersedes it.

## Work and verification

1. Add focused failing regressions at the real store/hook/component seam, then make the smallest root-cause fixes.
2. Return the committed settings snapshot with the existing activation acknowledgement; publish only non-superseded, non-older snapshots to the query cache. Preserve revision conflict validation and disk-first responsiveness.
3. Correct form resolver validation, native dialog lifecycle and address focus using existing primitives.
4. Separate explorer context invalidation from listing revision; route card/checkbox/Ctrl/Shift navigation through the existing transition guard without clearing the old selection first.
5. Format owned files, targeted tests, frontend/E2E types, lint/architecture, build and focused review. Replay native regressions serially on isolated fixtures when the driver environment permits; do not weaken assertions to manufacture green.
6. Record actual results, unreplayed boundaries and fixture cleanup in history. Audit observations remain historical evidence, not silently rewritten as passes.

Replay-driven additions use the same gates: failing focused tests first, native reproduction, minimal fix, and read-only review. Collection apply replay locates the actually visible modal rather than assuming a native `open` attribute; no product assertion or disk check is removed.

UI direction: preserve `design.md` Workbench/semantic surfaces and existing controls (ENERGY 1 / RHYTHM 2 / MOTION 1). No new decoration, layouts, dependencies, queues, general-purpose caches, schema migrations or broad Windows path-normalization change. The replay-proven bulk handoff uses one transient committed receipt.
