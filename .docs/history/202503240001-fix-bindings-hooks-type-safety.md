# Bindings & Hooks Type Safety Alignment

## Context

The project was experiencing numerous TypeScript compilation errors (`tsc`) due to structural mismatches between the Rust backend commands and the frontend IPC bindings in `bindings.ts`. Additionally, many frontend hooks and components were using inconsistently defined types for common domain objects like `RandomModProposal` and `MatchedDbEntry`.

## Changes

- **`bindings.ts` Consolidation**:
  - Removed all duplicate command definitions (e.g., `hasPin`, `readModInfo`, `pasteThumbnail`).
  - Synchronized and corrected 45+ command signatures to match `lib.rs` and frontend usages.
  - Injected missing internal types: `IniDocument`, `IniLineUpdate`, `MatchedDbEntry`, `IniVariable`, `KeyBinding`.
  - Standardized parameter naming (e.g., merging `id`/`gameId`, `path`/`folderPath`) using optional parameters to maintain backward compatibility.
- **`RandomizerModal.tsx`**:
  - Updated `RandomModProposal` interface to include `name` and allow `null` for `thumbnail_path`, matching backend behavior.
- **Onboarding Alignment**:
  - Adjusted `addGameManual` and `autoDetectGames` signatures to accommodate optional parameters used in `ManualSetupForm.tsx` and `WelcomeScreen.tsx`.

## Impacted Files

- `src/lib/bindings.ts` (modified/rewritten)
- `src/features/randomizer/RandomizerModal.tsx` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/onboarding/ManualSetupForm.tsx` (verified/aligned)
- `src/features/onboarding/WelcomeScreen.tsx` (verified/aligned)

## Goal

Achieve a 100% clean TypeScript build (`tsc --noEmit`) and ensure robust, type-safe communication between the React frontend and Rust backend.

## Impact

- **Stability**: Elimination of silent runtime failures caused by command signature mismatches.
- **Developer Experience**: Accurate IntelliSense and type checking for all IPC calls.
- **Performance**: Consolidated `bindings.ts` reduces file complexity and potential for shadowed logic.

## Notes

- `matchObjectWithDb` was switched to return `MatchedDbEntry | null` to align with the database matching UI.
- All preview/INI commands were verified against the backend `preview_cmds.rs` registry.
