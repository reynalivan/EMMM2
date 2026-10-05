/**
 * Hook-free building blocks for Workspace Switch.
 *
 * Everything here is either pure (payload/descriptor/key builders) or a plain
 * async operation, so it can be tested without rendering a component.
 */

import type { QueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { commands } from '../../../shared/api/tauri/bindings';
import { extractFileInUsePayload, formatAppError } from '../../../shared/lib/appError';
import { toast } from '@/shared/ui/toast';
import i18next from '@/shared/lib/i18n';
import { useAppStore } from '@/app/store';
import type {
  WorkspaceImpact,
  WorkspaceNode,
  WorkspaceObjectNode,
  WorkspaceSwitchInput,
  WorkspaceSwitchResult,
} from '@/entities/workspace';
import { applyRuntimeEffects } from '../optimistic/applyOptimisticEffects';
import {
  buildRuntimeMutationDescriptor,
  buildRefreshDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '../optimistic/descriptorBuilders';
import {
  cancelRuntimeDescriptorQueries,
  publishRuntimeDescriptor,
} from '@/shared/lib/queryRefresh';
import {
  openFolderConflictManagerDialog,
  openRenameConfirmationDialog,
  openWorkspaceFileInUseDialog,
} from '../state/workspaceDialogs';
import { notifyCommittedMutationSyncWarning } from '../../../shared/lib/committedMutationWarning';
import { modHealthKeys } from '@/entities/mod';
import { canonicalPathKey, identityPathKey } from '@/shared/lib/pathKey';
import {
  WorkspaceProjectionTracker,
  WorkspaceProjectionNeedsRepairError,
  WorkspaceRootEpochChangedError,
} from './workspaceProjectionTracker';
import type { RuntimeEffectDescriptor, RuntimeRefreshEvent } from '@/shared/lib/runtimeEffects';

export type WorkspaceSwitchSurface = 'folder_grid' | 'preview' | 'object_list' | 'collections';

export type WorkspaceSwitchFallbackClass = 'folderSwitch' | 'objectSwitch';
export interface WorkspaceSwitchEffectsOptions {
  publish?: boolean;
  replayPathRewrites?: boolean;
  gameId?: string;
  onSyncError?: (error: unknown) => void;
  shouldNotifySyncError?: () => boolean;
}

export interface WorkspaceRenameConflictPayload {
  type: 'RenameConflict';
  attempted_target: string;
  existing_path: string;
  base_name: string;
}

let latestIntentRevision = 0;

export function nextWorkspaceIntentRevision(): number {
  latestIntentRevision = Math.max(Date.now() * 1_000, latestIntentRevision + 1);
  return latestIntentRevision;
}

export function admitWorkspaceIntentOverride(
  gameId: string,
  targets: WorkspaceSwitchInput['target'][],
  revision: number,
): void {
  void commands.admitWorkspaceSwitchIntent(gameId, targets, revision).catch((error: unknown) => {
    console.error('[WorkspaceSwitch] Could not admit latest intent:', error);
  });
}
const projectionTracker = new WorkspaceProjectionTracker(
  (gameId) => commands.getWorkspaceSwitchSnapshot(gameId),
  (gameId) => isWorkspaceGameCurrent(gameId),
);
export const subscribeWorkspaceRootEpoch = projectionTracker.onEpochChange.bind(projectionTracker);
export const primeWorkspaceRootEpoch = projectionTracker.prime.bind(projectionTracker);
export const invalidateWorkspaceRootEpoch = projectionTracker.invalidate.bind(projectionTracker);
let projectionListenerReady: Promise<boolean> | null = null;

interface WorkspaceSwitchProjectedEvent {
  game_id: string;
  disk_revision: number;
  source_epoch?: string;
}

export function recordWorkspaceProjectedRevision(
  gameId: string,
  revision: number,
  sourceEpoch?: string,
): void {
  projectionTracker.record(gameId, revision, sourceEpoch);
}

export function ensureWorkspaceProjectionListener(): Promise<boolean> {
  projectionListenerReady ??= listen<WorkspaceSwitchProjectedEvent>(
    'workspace_switch:projected',
    ({ payload }) => {
      recordWorkspaceProjectedRevision(
        payload.game_id,
        payload.disk_revision,
        payload.source_epoch,
      );
    },
  ).then(
    () => true,
    (error: unknown) => {
      projectionListenerReady = null;
      console.error('[WorkspaceSwitch] Projection listener unavailable:', error);
      return false;
    },
  );
  return projectionListenerReady;
}

export function waitForWorkspaceProjection(
  gameId: string,
  revision: number,
  sourceEpoch?: string,
): Promise<void> {
  void ensureWorkspaceProjectionListener();
  return projectionTracker.wait(gameId, revision, sourceEpoch);
}

export function isWorkspaceObjectNode(node: WorkspaceNode): node is WorkspaceObjectNode {
  return node.node_kind === 'object';
}

export function parseRenameConflict(error: unknown): WorkspaceRenameConflictPayload | null {
  const raw =
    typeof error === 'object' &&
    error !== null &&
    'type' in error &&
    error.type === 'Io' &&
    'payload' in error &&
    typeof error.payload === 'string'
      ? error.payload
      : error instanceof Error
        ? error.message
        : String(error);
  if (!raw.includes('"type":"RenameConflict"')) {
    return null;
  }

  try {
    const parsed: unknown = JSON.parse(raw);
    if (
      typeof parsed !== 'object' ||
      parsed === null ||
      !('type' in parsed) ||
      parsed.type !== 'RenameConflict' ||
      !('attempted_target' in parsed) ||
      typeof parsed.attempted_target !== 'string' ||
      !('existing_path' in parsed) ||
      typeof parsed.existing_path !== 'string' ||
      !('base_name' in parsed) ||
      typeof parsed.base_name !== 'string'
    ) {
      return null;
    }
    return parsed as WorkspaceRenameConflictPayload;
  } catch {
    return null;
  }
}

export async function showWorkspaceRenameConflictDialog(
  gameId: string,
  error: unknown,
): Promise<boolean> {
  if (!parseRenameConflict(error)) return false;
  const report = await commands
    .reconcileDiskStateCmd(gameId, 'ManualRepair', null, true)
    .catch(() => null);
  if (!isWorkspaceGameCurrent(gameId)) return true;

  const appStore = useAppStore.getState();
  const appliedReport = report ? appStore.applyFolderConflictReconcileResult(report) : false;
  if (report?.status === 'AppliedWithFolderConflicts' && report.folder_conflicts.length > 0) {
    if (appliedReport) {
      appStore.setRenameConfirmations(gameId, []);
      openFolderConflictManagerDialog();
    }
  } else if (
    report?.status === 'NeedsRenameConfirmation' &&
    report.rename_confirmations.length > 0
  ) {
    if (appliedReport) {
      appStore.setRenameConfirmations(gameId, report.rename_confirmations);
      openRenameConfirmationDialog();
    }
  } else {
    toast.error(formatAppError(error));
  }
  return true;
}

export function isWorkspaceGameCurrent(gameId: string): boolean {
  return useAppStore.getState().activeGameId === gameId;
}

interface SwitchRefreshRequest {
  gameId?: string;
  sourceEpoch?: string;
  diskRevision: number | null;
  descriptor: RuntimeEffectDescriptor | null;
  affectedPaths: string[];
  onSyncError?: (error: unknown) => void;
  shouldNotifySyncError?: () => boolean;
  callbackKey?: string;
  afterEpochVerification?: () => void;
}

type SwitchCancellationResult = { ok: true } | { ok: false; error: unknown };

interface PendingSwitchRefresh {
  sourceEpoch?: string;
  diskRevision: number | null;
  version: number;
  events: Set<RuntimeRefreshEvent>;
  healthKeys: Map<string, readonly unknown[]>;
  errorCallbacks: Map<string, { onError?: (error: unknown) => void; shouldNotify?: () => boolean }>;
  verifiedEffects: Map<string, { order: number; apply: () => void }>;
  cancellation: Promise<SwitchCancellationResult>;
  completion: Promise<void>;
  resolve: () => void;
  reject: (error: unknown) => void;
  wake?: () => void;
}

const pendingSwitchRefreshes = new WeakMap<QueryClient, Map<string, PendingSwitchRefresh>>();
const runningSwitchRefreshes = new WeakMap<QueryClient, Map<string, Promise<void>>>();
const SYNC_REPAIR_WARNING_DEDUPE_MS = 30_000;
const SYNC_REPAIR_WARNING_DURATION_MS = 7_000;
const MAX_RECENT_SYNC_REPAIR_WARNINGS = 64;
const recentSyncRepairWarnings = new Map<string, number>();

function showWorkspaceSwitchRepairWarning(gameKey: string, sourceEpoch?: string): void {
  const key = JSON.stringify([gameKey, sourceEpoch]);
  const now = Date.now();
  const lastShownAt = recentSyncRepairWarnings.get(key);
  if (lastShownAt !== undefined && now - lastShownAt < SYNC_REPAIR_WARNING_DEDUPE_MS) return;
  if (
    recentSyncRepairWarnings.size >= MAX_RECENT_SYNC_REPAIR_WARNINGS &&
    !recentSyncRepairWarnings.has(key)
  ) {
    const oldestKey = recentSyncRepairWarnings.keys().next().value;
    if (oldestKey !== undefined) recentSyncRepairWarnings.delete(oldestKey);
  }
  recentSyncRepairWarnings.delete(key);
  recentSyncRepairWarnings.set(key, now);
  toast.warning(
    i18next.t('common:reconcile.workspace_switch_needs_repair'),
    SYNC_REPAIR_WARNING_DURATION_MS,
  );
}

/** Shared revision barrier and union refresh: a failed burst owns one retry timer. */
export function scheduleWorkspaceSwitchRefresh(
  queryClient: QueryClient,
  request: SwitchRefreshRequest,
): Promise<void> {
  const gameKey = request.gameId ?? '';
  if (request.gameId && !isWorkspaceGameCurrent(request.gameId)) return Promise.resolve();
  const pendingByGame =
    pendingSwitchRefreshes.get(queryClient) ?? new Map<string, PendingSwitchRefresh>();
  pendingSwitchRefreshes.set(queryClient, pendingByGame);
  let pending = pendingByGame.get(gameKey);
  if (pending && pending.sourceEpoch !== request.sourceEpoch) {
    pending.reject(new WorkspaceRootEpochChangedError());
    pendingByGame.delete(gameKey);
    pending.wake?.();
    pending = undefined;
  }
  const isNew = !pending;
  if (!pending) {
    let resolve!: () => void;
    let reject!: (error: unknown) => void;
    const completion = new Promise<void>((onResolve, onReject) => {
      resolve = onResolve;
      reject = onReject;
    });
    pending = {
      sourceEpoch: request.sourceEpoch,
      diskRevision: request.diskRevision,
      version: 0,
      events: new Set(),
      healthKeys: new Map(),
      errorCallbacks: new Map(),
      verifiedEffects: new Map(),
      cancellation: Promise.resolve({ ok: true }),
      completion,
      resolve,
      reject,
    };
    pendingByGame.set(gameKey, pending);
  }
  pending.version += 1;
  if (request.diskRevision !== null)
    pending.diskRevision = Math.max(pending.diskRevision ?? 0, request.diskRevision);
  for (const event of request.descriptor?.refreshEvents ?? []) pending.events.add(event);
  const newHealthKeys = request.gameId
    ? request.affectedPaths.map((path) => modHealthKeys.report(request.gameId!, path))
    : [];
  for (const key of newHealthKeys) pending.healthKeys.set(JSON.stringify(key), key);
  {
    // Replacing the same scope's callback keeps rage-click failure bookkeeping bounded.
    const scopeKey =
      request.callbackKey ??
      request.affectedPaths
        .map((path) => canonicalPathKey(path) ?? path)
        .sort()
        .join('|');
    pending.errorCallbacks.set(scopeKey, {
      onError: request.onSyncError,
      shouldNotify: request.shouldNotifySyncError,
    });
  }
  if (request.afterEpochVerification) {
    pending.verifiedEffects.set(request.callbackKey ?? '', {
      order: request.diskRevision ?? pending.version,
      apply: request.afterEpochVerification,
    });
  }
  const cancellation = captureSwitchCancellation(
    Promise.all([
      request.descriptor
        ? cancelRuntimeDescriptorQueries(queryClient, request.descriptor)
        : Promise.resolve(),
      ...newHealthKeys.map((queryKey) => queryClient.cancelQueries({ queryKey })),
    ]),
  );
  pending.cancellation = Promise.all([pending.cancellation, cancellation]).then(
    ([prior, latest]) => (prior.ok ? latest : prior),
  );
  if (isNew) {
    const owner = pending;
    queueMicrotask(() => {
      void drainWorkspaceSwitchRefresh(queryClient, request.gameId, pendingByGame, gameKey, owner);
    });
  }
  return pending.completion;
}

// Keep a cancellation failure explicit without an unhandled rejection during retry backoff.
function captureSwitchCancellation(promise: Promise<unknown>): Promise<SwitchCancellationResult> {
  return promise.then(
    () => ({ ok: true }),
    (error: unknown) => ({ ok: false, error }),
  );
}

async function drainWorkspaceSwitchRefresh(
  queryClient: QueryClient,
  gameId: string | undefined,
  pendingByGame: Map<string, PendingSwitchRefresh>,
  gameKey: string,
  pending: PendingSwitchRefresh,
): Promise<void> {
  let retryMs = 250;
  let retryCancellation = false;
  const retry = async (error: unknown) => {
    for (const callback of pending.errorCallbacks.values()) callback.onError?.(error);
    console.warn('[WorkspaceSwitch] Shared refresh pending; retrying:', error);
    await new Promise<void>((resolve) => {
      const timer = setTimeout(resolve, retryMs);
      pending.wake = () => {
        clearTimeout(timer);
        resolve();
      };
    });
    pending.wake = undefined;
    retryMs = Math.min(retryMs * 2, 30_000);
  };
  try {
    while (pendingByGame.get(gameKey) === pending) {
      if (gameId && !isWorkspaceGameCurrent(gameId)) {
        pending.resolve();
        return;
      }
      try {
        if (retryCancellation) {
          pending.cancellation = captureSwitchCancellation(
            Promise.all([
              pending.events.size
                ? cancelRuntimeDescriptorQueries(
                    queryClient,
                    buildRefreshDescriptor([...pending.events]),
                  )
                : Promise.resolve(),
              ...[...pending.healthKeys.values()].map((queryKey) =>
                queryClient.cancelQueries({ queryKey }),
              ),
            ]),
          );
        }
        try {
          const cancellation = await pending.cancellation;
          if (!cancellation.ok) throw cancellation.error;
          retryCancellation = false;
        } catch (error) {
          retryCancellation = true;
          throw error;
        }
        const version = pending.version;
        if (gameId && pending.sourceEpoch !== undefined) {
          await projectionTracker.verifyEpoch(gameId, pending.sourceEpoch);
        }
        if (pendingByGame.get(gameKey) !== pending) return;
        if (gameId && !isWorkspaceGameCurrent(gameId)) {
          pending.resolve();
          return;
        }
        for (const effect of [...pending.verifiedEffects.values()].sort(
          (left, right) => left.order - right.order,
        ))
          effect.apply();
        pending.verifiedEffects.clear();
        if (gameId && pending.diskRevision !== null) {
          await waitForWorkspaceProjection(gameId, pending.diskRevision, pending.sourceEpoch);
        }
        if (pendingByGame.get(gameKey) !== pending) return;
        if (gameId && !isWorkspaceGameCurrent(gameId)) {
          pending.resolve();
          return;
        }
        const runningByGame =
          runningSwitchRefreshes.get(queryClient) ?? new Map<string, Promise<void>>();
        runningSwitchRefreshes.set(queryClient, runningByGame);
        await runningByGame.get(gameKey)?.catch(() => undefined);
        if (pendingByGame.get(gameKey) !== pending) return;
        if (gameId && !isWorkspaceGameCurrent(gameId)) {
          pending.resolve();
          return;
        }
        const refresh = Promise.all([
          ...[...pending.healthKeys.values()].map((queryKey) =>
            queryClient.invalidateQueries({ queryKey, refetchType: 'active' }),
          ),
          pending.events.size
            ? publishRuntimeDescriptor(
                queryClient,
                buildRefreshDescriptor([...pending.events]),
                'active',
              )
            : Promise.resolve(),
        ]).then(() => undefined);
        runningByGame.set(gameKey, refresh);
        try {
          await refresh;
          if (gameId && !isWorkspaceGameCurrent(gameId)) {
            pending.resolve();
            return;
          }
          if (version === pending.version) {
            pending.resolve();
            return;
          }
          retryMs = 250;
        } finally {
          if (runningByGame.get(gameKey) === refresh) runningByGame.delete(gameKey);
        }
      } catch (error) {
        if (
          error instanceof WorkspaceRootEpochChangedError ||
          error instanceof WorkspaceProjectionNeedsRepairError
        )
          throw error;
        if (pendingByGame.get(gameKey) !== pending) return;
        if (gameId && !isWorkspaceGameCurrent(gameId)) {
          pending.resolve();
          return;
        }
        await retry(error);
      }
    }
  } catch (error) {
    if (
      error instanceof WorkspaceProjectionNeedsRepairError &&
      pendingByGame.get(gameKey) === pending &&
      (!gameId || isWorkspaceGameCurrent(gameId))
    ) {
      for (const callback of pending.errorCallbacks.values()) callback.onError?.(error);
      if (
        pending.diskRevision !== null &&
        [...pending.errorCallbacks.values()].some((callback) => callback.shouldNotify?.() !== false)
      ) {
        showWorkspaceSwitchRepairWarning(gameKey, pending.sourceEpoch);
      }
    }
    pending.reject(error);
  } finally {
    if (pendingByGame.get(gameKey) === pending) pendingByGame.delete(gameKey);
  }
}

export function buildNodePendingKey(node: WorkspaceNode): string {
  if (isWorkspaceObjectNode(node)) {
    return `object:${node.id}`;
  }

  if (node.filesystem_identity) return `folder:physical:${node.filesystem_identity}`;
  return `folder:${node.id ?? canonicalPathKey(node.path) ?? node.path}`;
}

/** Immutable add/remove for the pending-key map backing the switch spinner. */
export function togglePendingKey(
  current: Record<string, boolean>,
  key: string,
  pending: boolean,
): Record<string, boolean> {
  if (pending) {
    return { ...current, [key]: true };
  }

  if (!current[key]) {
    return current;
  }

  const next = { ...current };
  delete next[key];
  return next;
}

export function buildSwitchRefreshDescriptor(
  impact: WorkspaceImpact | null | undefined,
  fallbackClass: WorkspaceSwitchFallbackClass,
) {
  if (!impact || impact.refresh_scopes.length === 0) {
    return buildRuntimeMutationDescriptor(fallbackClass);
  }

  return buildRefreshDescriptor(impact.refresh_scopes);
}

/** Runs the switch command, routing known failures to their dialogs. */
export function executeWorkspaceSwitch(
  input: WorkspaceSwitchInput,
  intentRevision = nextWorkspaceIntentRevision(),
): Promise<WorkspaceSwitchResult | null> {
  return (async () => {
    try {
      void ensureWorkspaceProjectionListener();
      const result = await commands.executeWorkspaceSwitch(input, intentRevision);
      if (isWorkspaceGameCurrent(input.game_id)) {
        notifyCommittedMutationSyncWarning(result);
      }
      return result;
    } catch (error) {
      if (!isWorkspaceGameCurrent(input.game_id)) {
        return null;
      }
      if (await showWorkspaceRenameConflictDialog(input.game_id, error)) {
        return null;
      }

      const fileInUse = extractFileInUsePayload(error);
      if (fileInUse) {
        openWorkspaceFileInUseDialog({ path: fileInUse.path, processes: fileInUse.processes });
        return null;
      }

      toast.error(formatAppError(error));
      return null;
    }
  })();
}

/** Runs one all-or-nothing object batch through the workspace mutation pipeline. */
export function executeWorkspaceObjectBulkSwitch(
  gameId: string,
  objectIds: string[],
  desiredEnabled: boolean,
  intentRevision = nextWorkspaceIntentRevision(),
): Promise<WorkspaceSwitchResult | null> {
  return (async () => {
    try {
      void ensureWorkspaceProjectionListener();
      const result = await commands.executeWorkspaceObjectBulkSwitch(
        gameId,
        objectIds,
        desiredEnabled,
        intentRevision,
      );
      if (isWorkspaceGameCurrent(gameId)) {
        notifyCommittedMutationSyncWarning(result);
      }
      return result;
    } catch (error) {
      if (!isWorkspaceGameCurrent(gameId)) {
        return null;
      }
      const fileInUse = extractFileInUsePayload(error);
      if (fileInUse) {
        openWorkspaceFileInUseDialog({ path: fileInUse.path, processes: fileInUse.processes });
        return null;
      }

      toast.error(formatAppError(error));
      return null;
    }
  })();
}

/**
 * The post-switch cache work every switch shape shares: replay the backend's
 * path rewrites, then publish the refresh scopes.
 *
 * The rewrite list is the backend's own account of what moved — empty for a
 * no-op — so the initial receipt replays it unconditionally. Reactivation
 * skips these historical rewrites and refreshes the current state instead.
 * Thumbnails are identity-keyed and survive a toggle, so nothing is dropped here.
 */
export function applyWorkspaceSwitchEffects(
  queryClient: QueryClient,
  result: WorkspaceSwitchResult,
  fallbackClass: WorkspaceSwitchFallbackClass,
  options: WorkspaceSwitchEffectsOptions = {},
): Promise<void> {
  if (options.gameId && !isWorkspaceGameCurrent(options.gameId)) {
    return Promise.resolve();
  }
  if (result.status === 'noop') return Promise.resolve();
  const sourceEpoch =
    'source_epoch' in result && typeof result.source_epoch === 'string'
      ? result.source_epoch
      : undefined;

  const descriptor =
    options.publish === false ? null : buildSwitchRefreshDescriptor(result.impact, fallbackClass);
  const seen = new Set<string>();
  const affectedPaths = (result.changed_folder_paths ?? []).filter((path) => {
    const key = identityPathKey(path) ?? path;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });

  const applyPathRewrites = () =>
    applyRuntimeEffects(
      queryClient,
      buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
    );
  const deferPathRewrites =
    options.gameId && !projectionTracker.acceptsEpoch(options.gameId, sourceEpoch);
  if (options.replayPathRewrites !== false && !deferPathRewrites) applyPathRewrites();

  return scheduleWorkspaceSwitchRefresh(queryClient, {
    gameId: options.gameId,
    sourceEpoch,
    diskRevision: typeof result.disk_revision === 'number' ? result.disk_revision : null,
    descriptor,
    affectedPaths,
    onSyncError: options.onSyncError,
    shouldNotifySyncError: options.shouldNotifySyncError,
    callbackKey: canonicalPathKey(result.primary_path) ?? undefined,
    afterEpochVerification:
      options.replayPathRewrites !== false && deferPathRewrites ? applyPathRewrites : undefined,
  });
}
