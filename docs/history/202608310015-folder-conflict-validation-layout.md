# Folder Conflict Validation Layout

## Context

Rename validation errors could render beside the input instead of below it,
which made longer messages appear clipped in the conflict dialog.

## Changes

- Use an explicit column layout for the rename field.
- Make the input and validation message span the available field width.
- Allow validation copy to wrap naturally below the input.
- Keep the blur validation regression covered for empty and duplicate names.

## Impacted Files

- `src/widgets/mod-explorer/modals/FolderConflictCandidateCard.tsx`
- `src/widgets/mod-explorer/modals/FolderConflictManager.test.tsx`

## Validation

- Folder conflict Vitest suite: 23 tests passed.
- `pnpm i18n:lint` passed.
- `pnpm exec tsc --noEmit` passed.
- Prettier check passed.
- Production build passed.
