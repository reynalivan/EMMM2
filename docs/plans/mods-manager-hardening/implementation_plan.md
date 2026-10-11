# Mods Manager hardening

## Authorized scope

User approved MM-1 through MM-6 and specified conflict grouping and reversible
thumbnail visibility. Work stays in the existing checkout and preserves other
in-flight edits.

## Changes and acceptance

1. Carry canonical validated filesystem paths into bulk metadata writers while
   preserving caller paths for reporting. Relative requests must write only
   below their game Mods root; SQL failures restore the same original files.
2. Consume structured bulk outcomes in ObjectList. Total/partial failure must
   be visible, and old-game responses must not report success in the new game.
3. Use physical canonical containment for external INI editor targets, including
   junctions toward DISABLED-equivalent external directories.
4. Key native thumbnail caching by physical identity. Existing conflict reports
   define grouped explorer cards and focused conflict overlays; each physical
   candidate retains its own preview. Normal toggles reuse cached image bytes.
5. Persist hide/show state without deleting image files. Gallery menus show
   hidden images and restore them; grid thumbnails honor the same state.
6. Preserve original folder-creation ancestry and staging ownership. Stage
   creation happens under the shared coordinator lock; journal plans include
   native identity, and publication/compensation never overwrite foreign paths.

## Validation

- Reproduce regressions before production changes, then run focused Rust and
  frontend tests, typechecks, strict lint/Clippy, formatting and whitespace checks.
- Register and generate the two additive visibility IPC commands together with
  permission tests. No new dependency or database migration is planned.
- Use the existing isolated native Windows WDIO harness for affected app flows.
  Never reset production app data or use browser fixtures as native evidence.
- Recheck changed source and review correctness before handoff. Record actual
  results and any concurrent-work or environmental limitations in history.

## Approved native-completion continuation

The user requested completion of MM-5/MM-6 on 2026-10-10. Reproduce the native
mutation rejection, test the delayed-event identity-cache boundary, and correct
only proven watcher failures without bypassing authority or recovery gates.
Verify hide/show/restore and clear-gallery with exact image-byte preservation,
folder creation through a configured junction, and persistence through an
actual native restart. Add interrupted-creation recovery coverage using the
existing durable journal and disposable fixtures, without production test IPC.
Keep external editor/clipboard OS integration outside this continuation's
acceptance boundary. Preserve unrelated concurrent changes and record the final
native and regression results separately from the previous partial report.

The user also requested correcting the "Manual reload required" notice. Skip
reload when the configured game is confirmed stopped, while retaining real
manual-reload outcomes for running/unknown games and focus-gated input replay.
Verify publication still succeeds without a reload binding when the game is
closed; do not hide genuine publication failures or loosen focus checks.

Completed at the approved Windows boundary: seven final native cases, 527 Rust
tests and 366 frontend tests passed. See history
`202610100011-complete-mods-preview-and-folder-recovery.md` and the schema-validated
`completion-report.json` for outcomes and remaining manual integration/release checks.
