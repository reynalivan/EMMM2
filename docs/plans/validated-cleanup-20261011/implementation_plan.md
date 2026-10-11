# Validate and simplify the seven cleanup candidates

The user authorized cleanup only after revalidating each candidate on the current
working tree. Scope is the listed frontend/Rust wrappers, duplicate theme hooks,
and obsolete raw-hash compatibility APIs.

1. Check tracked and untracked runtime callers, entry points, IPC contracts and
   tests. Retain anything that still has a supported caller.
2. Remove the unused toast/barrel, three bulk hooks and stale mocks. Use the
   required backend path rewrite array directly and keep its empty/mixed-result
   coverage in the actual bulk hook tests.
3. Route General settings to the entity theme API and remove the page duplicate.
   Remove unused object query/service wrappers and imports.
4. Migrate meaningful raw-hash tests to the current typed harvester, then remove
   the unused compatibility pipeline and overlay trigger wrappers.
5. Run targeted frontend/Rust tests, app/E2E typechecks, scoped lint/format,
   architecture checks, strict Rust Clippy, a fresh private native build and
   focused native verification of retained theme/bulk/KeyViewer flows.

Preserve persisted migrations, journal recovery, filesystem identity guards,
active Rust service implementations and IPC registrations. Review the final
cleanup and document evidence and any verification limits.
