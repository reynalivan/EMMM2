# Client smoke regression fixes

Scope: approved local fixes after the client audit, using isolated native E2E data only. No commit, push, release installer or production-library mutation in this session. Original audit reports remain historical.

## Root causes and changes

- **First settings save failed:** activation advanced the backend revision while the client retained its pre-activation snapshot. The existing activation receipt now returns committed settings; a shared publisher rejects older snapshots and cancels superseded reads. No additional hydration IPC or optional-runtime wait.
- **Duplicate physical game roots accepted:** resolver validation bypassed the input-level rule. The Zod form schema now uses existing path equality; identical basenames in different directories remain valid.
- **Selection/preview lost on refresh:** listing revision was treated like a context change. Explicit selection survives refresh; snapshot-bound all-matching selection still invalidates. Dirty-editor confirmation applies grid/preview together and rewrites queued paths after a rename.
- **Rename/editor keys misbehaved:** grid shortcuts intercepted text-control events. Shared editable-target guards preserve actual typing while retaining grid shortcuts outside editors.
- **Empty metadata description reverted:** refreshed source and stale save acknowledgements replaced newer drafts. A single draft/baseline state merges only clean fields, scopes acknowledgements to the active selection/save, and preserves newer edits. Existing autosave delay is unchanged.
- **Duplicate rename opened diagnostic consent:** the mutation reported the conflict but its rejected Promise escaped the inline handler. The UI boundary now consumes that reported rejection; only successful mutations close the dialog or run success callbacks.
- **Bulk Enable intermittently lost after Disable:** the overlap gate compared raw paths/listing revision while the first action's disk rename was already rewriting selection. Explicit same-physical selections now survive that window only with complete matching filesystem identities. Game/query/mode and all-matching snapshot guards remain; different physical same-name folders are not conflated.
- Native click instrumentation also confirmed a **completed-operation gap**: new selection paths were submitted against old cached query candidates. A scoped last-commit receipt bridges only matching explicit targets through the existing identity-validated bulk command until the listing covers those paths; fresh/contradictory candidates, another context/selection, or a failure retire the proof.
- Receipt paths may use a different namespace/spelling than UI paths (relative/absolute or Windows/slash forms). The existing workspace rewrite helper maps original selected paths into their current UI form; only complete, unique source identities that also occur in the backend receipt are admitted. No basename matching. Context ABA retires the proof and membership checks are linear.
- **Trust Escape/address editing/typed errors:** native modal lifecycle restores focus and closes on Escape, focused addresses remain editable, and structured errors use the existing formatter.

No new dependencies, mutation queues, schema migrations or background-refresh waits on the switch critical path. The bulk handoff retains one transient committed receipt. Concurrent Mod Inbox edits are preserved, not attributed to these fixes.

## Verification

Focused regressions were observed failing before their fixes. Read-only reviews found no remaining actionable findings within the changed scope.

- Native isolated Settings: **44/44 passed**, including first-save revision handling, duplicate path validation and real Trust-dialog Escape.
- Native Browser direct address entry: **1/1 passed** without the audit's prefix workaround; loopback server closed and timestamp restored.
- Native rename replay: **1/1 passed** for blank, Cancel, duplicate and Unicode outcomes after fixing keyboard handling and the unhandled rejection.
- Final whole frontend suite: **212 files passed; 1,203 tests passed, 1 skipped**. Frontend/E2E typechecks, full ESLint (zero warnings), architecture lint, Rust formatting and diff checks passed.
- Final native workspace replay **without temporary product diagnostics: 20/20 passed (1m 19.7s)**. Exact disk prefixes, seven captured rapid switch events, both actual bulk button clicks, collision sentinels, INI guard/save, empty metadata description and collection restoration were verified. A prior 18/20 run and subsequent 19/20 runs exposed the bulk handoff gap; passing isolated reruns were not treated as the fix.
- Generated bindings export and command-permission registry tests: **1 passed each**. Fresh frontend and debug native no-bundle builds succeeded.

Native harnesses verified the private E2E identifier before reset, stopped their owned process trees and removed marked temporary fixtures. Global hotkeys/telemetry remained off; no anonymous report was sent.

## Remaining boundaries

Passing focused smoke tests does not certify every app button or a large-library performance SLA. Native file pickers, Explorer/default editor, external viewer, real game/loader/global hotkeys, clipboard-image paste, positive mod/image recycle-bin deletion, drag/resize gestures and bulk Move-to-Object menus were not proven in this fix replay. Full Browser navigation suite was not rerun. Existing audit recycle-bin residue was not removed by emptying the user's bin.
