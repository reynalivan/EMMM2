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
import type {
  DiskReconcileReason,
  GameActivationPhase,
  OnboardingIndexingWorkPlan,
  Result,
} from './bindings.gen';
import { resolveDemoCommand } from '@/demo/commands';

// Re-export the generated types that callers historically imported from this
// module, so `import type { X } from './bindings'` keeps working.
export type {
  AppSettings,
  BrowserBookmark,
  BrowserHistoryEntry,
  BrowserPrivacySummary,
  ApplyGameModsDirectoryRequest,
  ApplyGameModsDirectoryResult,
  ConfigStatus,
  CreateCollectionMode,
  CustomTheme,
  DiskReconcileChangeCounts,
  DiskReconcileChangeSummary,
  DiskReconcilePathKind,
  DiskReconcilePathUpdate,
  DiskReconcileReason,
  DiskReconcileResult,
  DiskReconcileStatus,
  FolderConflictRename,
  FolderConflictSummary,
  FolderNameConflictCandidate,
  FolderNameConflictGroup,
  GameModsDirectoryCandidateSummary,
  GameModsDirectoryClassification,
  GameModsDirectoryInspection,
  GameActivationPhase,
  GameActivationResult,
  GameObject,
  IniDocument,
  IniFileEntry,
  IniLineUpdate,
  IniVariable,
  ImportBatch,
  ImportBatchReport,
  ImportDecision,
  ImportFlow,
  ImportItem,
  ImportItemStatus,
  ImportSourceKind,
  KeyBinding,
  KeyViewerRuntimeDiagnostics,
  LiquidAppearance,
  LiquidMaterial,
  LiquidQuality,
  LiquidRoleConfig,
  LiquidThemeConfig,
  MoveModsToObjectInput,
  ModInboxEntry,
  ModInboxEntryKind,
  ModInboxLayout,
  ModInboxRootState,
  ModInboxSnapshot,
  OnboardingIndexingSession,
  OnboardingIndexingWorkPlan,
  PipelineTask,
  ProcessedModInboxDestination,
  ProcessedModInboxSource,
  ApplyRandomizedLoadoutInput,
  ApplyRandomizedLoadoutResult,
  RandomModProposal,
  RandomizedLoadoutPreview,
  RandomizedLoadoutPreviewItem,
  RandomizedLoadoutBackupInput,
  RandomizedLoadoutBackupResult,
  RandomizerSafetyFilter,
  RandomizerLoadoutMode,
  RandomizerScope,
  RuntimeReloadStatus,
  RuntimeSyncPublicationStatus,
  RenameConfirmationGroup,
  RenameConfirmationKind,
  RenameConfirmationReason,
  RenameConfirmationResolution,
  RenameConfirmationResolutionAction,
  StableCategory,
  SuggestRandomModsInput,
  TargetMode,
  TaskStatus,
  ThemeBackground,
  ThemeBackgroundKind,
  ThemeConfig,
  ThemeMetadata,
  WorkspaceMoveTarget,
} from './bindings.gen';

// Disk reconcile progress is an event payload rather than an IPC command
// signature, so tauri-specta does not place it in bindings.gen.ts.
export type DiskReconcilePhase =
  'DiscoveringRoots' | 'ScanningRoots' | 'Projecting' | 'Finalizing' | 'Completed' | 'Failed';

export type DiskReconcileProgress = {
  game_id: string;
  run_id: string;
  reason: DiskReconcileReason;
  phase: DiskReconcilePhase;
  completed_units: number;
  total_units: number | null;
  current_root: string | null;
  elapsed_ms: number;
  eta_ms: number | null;
};

export type RuntimeSyncPhase =
  'queued' | 'running' | 'succeeded' | 'needs_manual_reload' | 'failed';

export type RuntimeSyncStatus = {
  game_id: string;
  generation: number;
  phase: RuntimeSyncPhase;
  cause: string;
  message: string | null;
};

export type GameActivationStatus = {
  game_id: string | null;
  generation: number;
  phase: GameActivationPhase;
  reconcile_revision: number | null;
  runtime_sync_generation: number | null;
  error: string | null;
};

export type OnboardingIndexingWorkPlanUpdate = {
  session_id: string;
  work_plan: OnboardingIndexingWorkPlan;
};

export type OnboardingIndexingSnapshotProgress = {
  session_id: string;
  game_id: string;
  phase: 'Metadata' | 'Classifying' | 'Ready' | 'Rechecking';
  completed_games: number;
  total_games: number;
  completed_roots: number;
  total_roots: number;
  files_inspected: number;
  folders_classified: number;
  current_root: string | null;
  elapsed_ms: number;
};

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

function telemetryOperation(commandName: string): string {
  const normalized = commandName.toLowerCase();
  if (normalized.includes('refreshimportitemsuggestions')) return 'auto_match';
  if (normalized.includes('setimportitemdecision')) return 'classification_review';
  if (normalized.includes('classification')) return 'classification';
  if (normalized.includes('reconcile')) return 'reconcile';
  if (normalized.includes('watcher')) return 'watcher';
  if (normalized.includes('import')) return 'import';
  if (normalized.includes('extract') || normalized.includes('analyze')) return 'extract';
  if (normalized.includes('collection')) return 'collection_apply';
  if (normalized.includes('restore')) return 'restore';
  if (normalized.includes('launch')) return 'launch';
  if (normalized.includes('bulk')) return 'bulk_action';
  if (normalized.includes('toggle') || normalized.includes('setmod')) return 'toggle';
  return 'error';
}

function telemetryErrorCode(error: unknown): string {
  if (typeof error !== 'object' || error === null || !('type' in error)) return 'unknown';
  const type = String((error as { type: unknown }).type);
  const payload = (error as { payload?: unknown }).payload;
  const nestedType =
    typeof payload === 'string'
      ? payload
      : typeof payload === 'object' && payload !== null
        ? Object.keys(payload)[0]
        : undefined;
  if (type === 'Collection') {
    const codes: Record<string, string> = {
      NotFound: 'not_found',
      MissingMods: 'not_found',
      DuplicateName: 'conflict',
      Validation: 'validation',
      Db: 'database',
      RuntimeState: 'invariant',
      Io: 'io',
      FileInUse: 'external',
      PathBusy: 'external',
    };
    return nestedType === undefined ? 'unknown' : (codes[nestedType] ?? 'unknown');
  }
  if (type === 'Metadata') {
    const codes: Record<string, string> = {
      Security: 'permission',
      NotFound: 'not_found',
      Io: 'io',
      Db: 'database',
      Validation: 'validation',
    };
    return nestedType === undefined ? 'unknown' : (codes[nestedType] ?? 'unknown');
  }
  if (type === 'Browser') {
    const codes: Record<string, string> = {
      InvalidUrl: 'validation',
      InvalidSetting: 'validation',
      JobIncomplete: 'validation',
      Download: 'network',
      QueueFull: 'conflict',
      DownloadAlreadyActive: 'conflict',
      DownloadConfirmationUnavailable: 'not_found',
      Io: 'io',
      Db: 'database',
      WindowUnavailable: 'external',
      WebviewNotFound: 'external',
      Import: 'external',
      QueueClosed: 'external',
    };
    return nestedType === undefined ? 'unknown' : (codes[nestedType] ?? 'unknown');
  }
  if (type === 'Scanner') {
    const codes: Record<string, string> = {
      Security: 'permission',
      PathEscape: 'permission',
      PathNotFound: 'not_found',
      NotADirectory: 'not_found',
      Parse: 'validation',
      Validation: 'validation',
      Network: 'network',
      Io: 'io',
      Db: 'database',
    };
    return nestedType === undefined ? 'unknown' : (codes[nestedType] ?? 'unknown');
  }
  const codes: Record<string, string> = {
    Db: 'database',
    Io: 'io',
    Validation: 'validation',
    NotFound: 'not_found',
    Security: 'permission',
    Cancelled: 'cancelled',
    ArchiveUnsupported: 'unsupported',
    DuplicateConflict: 'conflict',
    PathBusy: 'external',
    FileInUse: 'external',
    RuntimeState: 'invariant',
    RuntimePathNotFound: 'not_found',
    Internal: 'invariant',
    ArchivePasswordRequired: 'validation',
    ArchivePasswordIncorrect: 'validation',
    ObjectHasMods: 'conflict',
    ExplorerSnapshotExpired: 'conflict',
  };
  return codes[type] ?? 'unknown';
}

function recordNativeCommandFailure(commandName: string, error: unknown): void {
  if (commandName === 'recordNativeErrorMetric') return;
  void gen
    .recordNativeErrorMetric(telemetryOperation(commandName), telemetryErrorCode(error))
    .catch(() => undefined);
}

export const commands: Commands = new Proxy({} as Commands, {
  get(_target, name: string) {
    const command = gen[name as keyof typeof gen] as (...args: unknown[]) => Promise<unknown>;
    return (...args: unknown[]) => {
      const demoResult = resolveDemoCommand(name, args);
      if (demoResult.handled) {
        return Promise.resolve(demoResult.value);
      }

      return command(...args)
        .then(unwrap)
        .catch((error: unknown) => {
          recordNativeCommandFailure(name, error);
          throw error;
        });
    };
  },
});
