# Fix Unexpected 'any' in bindings.ts

## Context

The IDE reported a type violation in `bindings.ts` where `any` was used in the `MatchedDbEntry` interface, triggering an "Unexpected any" error.

## Changes

- **`bindings.ts`**: Replaced `Record<string, any>` with `Record<string, unknown>` in the `MatchedDbEntry` interface's `metadata` property.

## Impacted Files

- `src/lib/bindings.ts` (modified)

## Goal

Improve type safety and resolve IDE/linting errors.

## Impact

- Resolved one TypeScript error.
- No behavioral changes, as `unknown` is a safer alternative to `any`.
