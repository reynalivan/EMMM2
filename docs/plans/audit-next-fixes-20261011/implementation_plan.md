# Settings and conflict audit fixes

Authorized by the user's explicit request to fix all seven findings and three
verified cleanup candidates. Preserve unrelated working-tree changes; no worktree,
dependency additions, commits or actual updater installation.

## Scope and ownership

1. Browser/AI frontend owner: bind homepage drafts/cache publication to the source
   game; block save before current data loads; preserve dirty endpoint and later
   key drafts across asynchronous saves. Add actual-component/query regressions.
2. Updater owner: retain operation/progress state outside GeneralTab, reject duplicate
   frontend requests and admit one backend job with failure-safe guard release.
   Verify check/install interleaving and remount without network/install side effects.
3. Theme owner: reject built-in custom IDs; use Onyx for new defaults and resolve
   historical dark without overwriting a legitimate custom dark theme. Remove
   unused save/delete/export commands consistently across registry, permissions
   and generated bindings; preserve active import/load/list/template helpers.
   Remove legacy CSS aliases and verify actual custom theme configuration remains.
4. Parent: prevent native conflict cancellation while pending; migrate winner/overlap
   coverage to chooseConflictModSetWinner before deleting chooseConflictWinner.

## Verification

- Establish failing tracked regression tests before each behavioral fix.
- Run focused frontend tests, typecheck, scoped lint/architecture checks and format checks.
- Run targeted Rust theme/settings/updater/IPC registry tests and strict lint checks.
- Use the existing freshly built private WDIO native harness for relevant UI/IPC
  scenarios after coordinating the shared compiler/driver slot. Actual update
  installation, external API credentials and real user library mutation are excluded.
- Read-only review of the completed diff; repair actionable findings, then record
  exact validation results, remaining native limits and all affected files.
