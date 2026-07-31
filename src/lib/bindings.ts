/**
 * IPC bindings for EMMM.
 *
 * `bindings.gen.ts` (tauri-specta) is the single source of truth for command
 * names, parameter order, and payload types. This module adds exactly one thing:
 * `Result<T, AppError>` unwrapping, so call sites keep the conventional
 * resolve/reject promise contract instead of branching on `status` everywhere.
 *
 * There is no hand-written per-command code here — the wrapper is derived from
 * the generated signatures, so any drift is a compile error at the call site.
 */

import { commands as gen } from './bindings.gen';
import type { Result } from './bindings.gen';

// Re-export the generated types that callers historically imported from this
// module, so `import type { X } from '../lib/bindings'` keeps working.
export type {
  ApplyObjectMatchInput,
  ConfigStatus,
  CreateCollectionMode,
  CustomTheme,
  DeepmatchPreviewForObjectsInput,
  DiskReconcileChangeCounts,
  DiskReconcileChangeSummary,
  DiskReconcilePathKind,
  DiskReconcilePathUpdate,
  DiskReconcileReason,
  DiskReconcileResult,
  DiskReconcileStatus,
  GameObject,
  IniDocument,
  IniFileEntry,
  IniLineUpdate,
  IniVariable,
  KeyBinding,
  MatchedDbEntry,
  MoveModsToObjectInput,
  PipelineTask,
  RandomModProposal,
  TaskStatus,
  ThemeConfig,
  ThemeMetadata,
  WorkspaceMoveTarget,
} from './bindings.gen';

type OkOf<T> = Extract<T, { status: 'ok' }>;

/** Rust unit `()` serialises as `null`; callers treat those commands as void. */
type NullToVoid<D> = [D] extends [null] ? void : D;

/** `Result<D, E>` -> `D`; anything else passes through unchanged. */
type Unwrapped<T> = [OkOf<T>] extends [never]
  ? T
  : OkOf<T> extends { status: 'ok'; data: infer D }
    ? NullToVoid<D>
    : T;

type Commands = {
  [K in keyof typeof gen]: (
    ...args: Parameters<(typeof gen)[K]>
  ) => Promise<Unwrapped<Awaited<ReturnType<(typeof gen)[K]>>>>;
};

/**
 * Serde defaults a missing `Option` field to `None`, exactly as if `null` had
 * been sent, so a sparse patch object is a valid payload for an all-nullable
 * update type. This cast records that wire contract in one place.
 */
export function sparse<T>(patch: NoInfer<Partial<T>>): T {
  return patch as T;
}

function isResult(value: unknown): value is Result<unknown, unknown> {
  return (
    typeof value === 'object' &&
    value !== null &&
    'status' in value &&
    ((value as { status: unknown }).status === 'ok' ||
      (value as { status: unknown }).status === 'error')
  );
}

function unwrap(value: unknown): unknown {
  if (!isResult(value)) return value;
  if (value.status === 'ok') return value.data;
  throw value.error;
}

export const commands: Commands = new Proxy({} as Commands, {
  get(_target, name: string) {
    const command = gen[name as keyof typeof gen] as (...args: unknown[]) => Promise<unknown>;
    return (...args: unknown[]) => command(...args).then(unwrap);
  },
});
