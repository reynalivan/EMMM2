# Guided Conflict Resolution Design

## Objective

Turn the enabled-mod conflict dialog from a passive report into a safe resolution workflow. Users can inspect conflict evidence, choose which mod remains enabled, review the global impact, and disable the other selected mods through the existing reversible toggle operation.

The first release operates at whole-mod scope. It does not rewrite INI sections, modify `match_priority`, rename individual ShaderFixes files, or attempt to evaluate the complete 3DMigoto runtime. Those operations are harder to reverse and cannot be recommended reliably from the evidence currently available.

## Interaction model

Each conflict card shows kind, certainty, hash, section or shader stage, and its participating enabled mods. Every mod row offers:

- `Keep enabled`
- `Disable`
- `Open folder`
- `Open INI` when the evidence points to an INI file

No decision is preselected. A `definite` conflict may use stronger warning copy, but the application does not guess the winner. A `potential` conflict explicitly explains that runtime conditions may prevent a real overlap and requires a manual choice.

Selecting `Keep enabled` for one conflict marks its other participants as disabled globally. Mod decisions are global across the dialog because one mod may participate in several conflicts. If a new choice contradicts an earlier choice, the UI explains which conflict groups will change before applying it. A group is resolved in the preview only when at most one participating mod remains enabled.

The primary action progresses from `Review changes` to an inline confirmation view. The confirmation lists exact mod paths, the number of conflict groups resolved, groups that remain unresolved, and any selected mod involved in additional conflicts. The final button reads `Disable N mods`; closing or going back performs no mutation.

## Components and state

`ConflictModal` remains the entry surface but delegates the guided flow to focused components:

- `ConflictGroupCard`: evidence and per-mod choices.
- `ConflictModDecisionRow`: keep/disable/open actions.
- `ConflictResolutionSummary`: global impact preview and confirmation.

The dialog owns a path-keyed decision map: `Record<modPath, 'keep' | 'disable'>`. Conflict identity is derived from the existing stable conflict signature fields (`kind`, `hash`, shader stage, and evidence paths), not array indexes. Derived selectors calculate resolved groups, unresolved groups, and the unique disable set.

The mutation should reuse `bulkToggleMods(gameId, paths, false)` through a dedicated hook rather than calling the command directly from presentation components. On terminal completion it refreshes workspace structure and active conflicts using the established runtime mutation descriptor. The dialog remains open while the refreshed result arrives so resolved groups disappear without losing context.

`Open folder` reuses the existing explorer command. `Open INI` should navigate to the existing preview INI editor when that route can be addressed reliably; otherwise the first implementation opens the containing folder and selects the source file where supported.

## Errors and recovery

Disable is reversible, but it still requires explicit confirmation because it changes runtime state. While the mutation runs, decisions and action buttons are locked against duplicate submission.

Bulk results are handled per path:

- Successful paths become disabled and are removed from the pending decision map.
- Failed paths remain selected and display their actionable error beside the corresponding mod.
- Active conflicts are always refetched after partial or complete success.
- The dialog closes only when the user closes it; partial success never masquerades as completion.

If a path changed externally before confirmation, the backend result is authoritative. The UI must not optimistically claim that the mod was disabled. If the source becomes unavailable or reconcile reports a conflict, no additional paths are submitted and the existing recovery guidance is shown.

The first release does not provide automatic rollback. Users can re-enable a mod through the normal mod controls, avoiding an undo action that might overwrite a newer external state change.

## Testing and acceptance criteria

Component tests cover definite and potential copy, global decisions shared across multiple groups, contradictory choices, impact counts, no-preselection behavior, and accessible keyboard operation.

Mutation tests cover full success, partial failure, complete failure, duplicate submission prevention, conflict-query refresh, and retaining unresolved choices after refetch. An integration test verifies that disabling one participant changes its folder to the normal disabled representation, removes the resolved active conflict, and does not disable the selected winner.

Acceptance criteria:

1. Opening the dialog never changes filesystem or database state.
2. Users can identify the exact evidence and source file for every participant.
3. A confirmation view shows every mod that will be disabled before mutation.
4. One mod cannot be simultaneously kept and disabled across different conflict groups.
5. Potential conflicts receive no automatic winner or destructive recommendation.
6. Partial failures remain visible and retryable.
7. Successful resolution refreshes the notice immediately and resolved groups disappear.
8. All controls are keyboard accessible and use translated labels in EN, ID, and ZH.
