# Fix verified Settings, shell and telemetry defects

The user authorized the five confirmed findings and four verified cleanup items
from the latest audit on 11 October 2026. Work stays in the shared checkout,
preserves unrelated edits and does not create a branch, stash, worktree or commit.

## Scope and ownership

- Parent: SS-1 frontend telemetry payload/privacy boundary, typed fake SDK/transport
  regressions, final integration/registry/build/native validation and documentation.
- Frontend worker: SS-2 compact portal lifecycle, SS-3 game editor session/cancel
  ownership (including parent inspection/confirmation), SS-4 resize cleanup; migrate
  callers/mocks and delete the two frontend re-export shims. A tiny shared layout
  context binds the actual compact target without widget-private imports or polling.
- Backend worker: SS-5 consent generation, short shared writer lock and captured
  producer handles across queued/long-running operations. Retain the existing
  bounded mpsc channel; stamp and reject old epochs. Cover dedicated/full-settings
  consent admission, reset/initialization, writes/export acknowledgements and local
  voluntary crash behavior. Caller changes only carry the captured handle.
- Backend cleanup: delete unused is_conflict_ignored; move strict classification
  coverage to scan_folder_strict + classify_folder_strict_from_scan, then delete
  the test-only classify_folder_strict wrapper. Preserve tables/migrations/guards.

## Verification

Reproducible behavior changes use RED→GREEN tests before production patches.
Run affected frontend/Rust suites, app/E2E types, scoped lint/architecture and
format checks; perform actual registry/Specta validation if signatures change.
Independently review consent/persistence and async UI ownership before accepting.
Finally use a fresh isolated Windows native fixture build for compact navigation,
GameForm cancel/session behavior and resize termination. Telemetry validation uses
fake transports and owned databases, never real endpoints/credentials or library.

## Shared workspace handoff

Performance completed its successful WPR retry, verified 1,578 inputs/owned
shutdown, restored private config and preserved the tested binary. Source,
compiler, tests and native slots were handed back before implementation.
Its startup readonly E2E/WPR helper changes are outside this task's edit scope.
Workers have disjoint files; one Cargo command owns the compiler at a time.

Unverified thumbnail-age, reset-retention and get_logs candidates remain outside
the authorized confirmed-fix scope. No dependency removal is assumed safe.

## Warning follow-up

The user additionally authorized fixing all warnings. Full lint includes ignored
audit script formatting, corrected public test imports and incomplete QueryClient
mocks. Normalize verified CRLF files to the repository's LF rule with archived
before-content and exact content comparison. Remove Rust type complexity with a
private alias and invoke the existing native build script directly with Node to
remove the shell-spawn deprecation. Preserve all lint rules and warning levels.

The first private release build also exposed three production dead-code warnings
hidden by all-targets tests. Validate their callers and data contracts while the
native source capture is frozen, then make the smallest safe cleanup and repeat
production warning checks and native verification after releasing that capture.

Production release Clippy additionally exposed six internal enum naming warnings.
Rename Rust symbols while retaining numeric game serialization, nine refresh
wire strings and the existing uppercase preflight cache key. Verify all qualified
reference replacements against snapshots and prove generated bindings unchanged.

Direct-Node full frontend logs revealed React act/mock-prop warnings hidden by the
pnpm launcher. Repair the sixteen demonstrated frontend test files with settled
updates and realistic mock props; preserve production code/assertions/negative
logging. Focused raw regressions precede one final full diagnostic run.

Native save verification fills the owned required executable input and holds real
SaveSettingsResult callback delivery rather than assigning Tauri's read-only
invoke property. Release/restoration disable future holding; exercise normal and
late completion cleanup before final native verification.

Keep immutable audit/browser evidence out of source control and leave generated
bindings and the pnpm lockfile in their generator's native format. Prettier still
checks all authored project files; repair the remaining documentation and HTML
formatting. Archive their before content and verify the HTML edit changes only
whitespace. Do not alter the existing lint, type, or Clippy warning gates.
