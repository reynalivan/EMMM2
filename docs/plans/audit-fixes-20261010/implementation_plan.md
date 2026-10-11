# Fix audit findings 1–3

The user approved these fixes on 2026-10-10. Scope is limited to the startup
recovery splash, import source ownership, and case-insensitive browser schemes.

1. Add a startup recovery regression that observes the recovery dialog and its
   handoff to the dashboard. Dismiss the static splash when recovery is ready.
2. Reproduce source replacement between import validation and publication.
   Validate the opened source identity and retain the same handle through rename
   and any cross-volume fallback, preserving retry and rollback ownership.
3. Add HTTP(S) scheme-case regressions in frontend and backend. Recognize the
   scheme case-insensitively while preserving the original host/path spelling.

Verification: targeted frontend and Rust regression tests, app/E2E typechecks,
scoped lint/format, Rust checks, focused native Windows E2E with the existing
private profile, and review of the import ownership change. Use owned temporary
fixtures only. Record actual results and limitations in the implementation log.
