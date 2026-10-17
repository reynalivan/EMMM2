# Package verified app changes for main

## Context

The user requested pushing completed application work to main and Windows installers.

## Changes

- Prepare 105 application/audit files: executable detection, import and Collection
  fixes, runtime publication, persistent Conflict entry, and regression coverage.
- Keep landing work, new Collection UI edits, captures and unrelated staged document
  removals outside this commit.
- Build production-identifier NSIS/MSI version 0.1.0 and updater signatures.
  Load the saved signing password into the child environment without logging it.

## Impacted Files

- This history file (added). No production source edits made during packaging.
- Application commit file list and artifact hashes:
  `.audit-output/release-20261017-manifest.json` (added, ignored).
- `.audit-output/release-20261017-*.log`, source snapshot, manifest/signature verifier,
  bundle helper and explicit commit pathspec (added, ignored).
- `src-tauri/target/release/bundle/{nsis,msi}/` (generated installers/signatures).

## Goal and Impact

Local x64 installers correspond to the selected, verified application source.
Both updater signatures verify against the configured public key. No installation,
version bump, GitHub release or updater feed publication was performed.

## Validation

Frontend: 245 files, 1,566 tests passed, one skipped with four workers; source and
native-spec ESLint passed with zero warnings. An initial unrestricted parallel run
had one UI wait timeout; the isolated file and bounded full rerun passed. Existing
JSDOM document-navigation notice remains. Previous Rust validation: 1,902 passed,
40 ignored, all-target Clippy and four native cases passed. Production build passed;
installer bundle/signing exit 0. Each selected input hash matches the pre-packaging
snapshot. Concurrent unstaged Collection UI and capture edits are excluded.

NSIS SHA-256: `6217392de5520186107e5c12965c025bda60d13e25db0ad05f7bf25e790a1921`.
MSI SHA-256: `853cea84cc306c368ac5c99e45984a62bd032a080a96402df708d6cb30f5767b`.
