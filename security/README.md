# Windows dependency advisory policy

Upgrade compatible vulnerable dependencies first. Do not disable audit warnings,
ignore whole crate families, or describe an accepted maintenance risk as fixed.

`node scripts/audit-rust-dependencies.mjs <output-directory>` runs pinned
`cargo-audit 0.22.2 --json --deny warnings` without any advisory, severity or target
filter. CI uploads that complete report even when evaluation fails. Cargo/tool,
JSON and unexpected output failures block the gate.

Only `x86_64-pc-windows-msvc` distribution is covered. The gate obtains the actual
normal/build dependency tree using locked Cargo resolution. It includes build
dependencies and unions features/parents across host and target contexts. Unlike
`cargo metadata`, this excludes inactive optional SQL drivers. Tests and other
platforms are not certified by this policy.

## Exception contract

Every entry in `rustsec-exceptions.json` names an exact advisory, package version,
finding kind, owner, evidence, follow-up and absolute UTC review deadline. Reviews
last at most 30 days. New/changed findings, newly reachable packages, changed
features or parents, expired reviews and unused exceptions fail CI. A maintainer
must review the evidence before changing an entry; updating a deadline is not a
dependency fix. Existing notices remain visible in the raw audit and decision log.

- RSA, GLib, obsolete macros and D-Bus entries must remain outside the Windows
  production tree. A future Linux release needs its own assessment.
- `rand 0.7.3` is a PHF build dependency. Its advisory requires the `log` feature,
  which is disabled; enabling it blocks the gate even if the graph is renewed.
- `fxhash`, `paste` and Unicode tables retain explicitly accepted, temporary
  maintenance risk through selectors, Specta and URLPattern. Follow up with the
  owning upstream; unrelated new vulnerabilities are not accepted.
- ZIP advisories concern convenience extraction APIs and Unix permissions/links.
  The application manually preflights and copies entries in
  `modules/library/application/mods/archive/zip_reader.rs` and the catalog pack
  import/update readers. Mod extraction rejects links, unsafe entry types, path
  traversal and collisions, and uses bounded copying with no-overwrite creation.
  Catalog extraction accepts bounded metadata/avatar entries, not arbitrary
  archive destinations. The updater disables its legacy ZIP feature and retains
  signature-verified raw MSI/NSIS support. Do not re-enable ZIP updater assets.

ZIP 9 was tested and rejected: unmarked bytes `C3 A9` changed from CP437
`├⌐` to UTF-8 `é`, breaking legitimate INI asset references. The regression test
`staging_preserves_cp437_asset_names_that_also_form_valid_utf8` protects this
compatibility contract. Existing archive tests cover traversal, links, modes,
collisions, passwords, compression and resource limits.

For this temporary source-reachability justification, the exception binds the
entire Rust source tree (including build.rs) to a SHA-256 digest, normalized to LF
for Windows checkouts. This is deliberately conservative: _any_ Rust source
change requires a renewed reachability review, including new callers or aliases
that a text search for `.extract()` could miss. Do not automatically regenerate
the digest in CI. Remove these exceptions after a compatible upstream fix.

The 2026-10-20 DAL/DTO relocation was independently reviewed before renewing this
source digest. Archive readers, extraction entry points and updater features did
not change; the existing review deadline remains unchanged.

The subsequent Rust 1.99 lint compatibility change replaces three constant-size
chunk iterators with array chunks and one test-only literal formatting call.
Independent review confirmed the existing even-length guards and equivalent
iteration semantics; archive/updater reachability and review deadlines did not
change when renewing the source digest.

References: [RustSec ignores](https://github.com/rustsec/rustsec/blob/main/cargo-audit/README.md#ignoring-advisories),
[OpenVEX justifications](https://github.com/openvex/spec/blob/main/OPENVEX-SPEC.md#status-labels),
[ZIP advisory](https://rustsec.org/advisories/RUSTSEC-2026-0368.html).
