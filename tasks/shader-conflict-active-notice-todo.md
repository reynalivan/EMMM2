# Checklist: Active Shader Conflict Notice

- [x] Backend red tests: nested disabled INI, nested disabled shader replacement, deep INI
- [x] Runtime-aware recursive discovery implemented
- [x] Hash namespace and `match_priority` regression tests added
- [x] Frontend red tests: INI, import, watcher invalidation, dismissal signature
- [x] Conflict query freshness fixes implemented
- [x] Notice/modal type and evidence labels aligned in EN/ID/ZH
- [x] Targeted Vitest suites green (19 tests)
- [ ] Full targeted Rust rerun blocked by unrelated object/workspace compile errors; metadata conflict suite passed 10 tests after the traversal fix
- [ ] Full build blocked by unrelated stale frontend contract tests; scoped ESLint and i18n lint green
- [x] Session history recorded
