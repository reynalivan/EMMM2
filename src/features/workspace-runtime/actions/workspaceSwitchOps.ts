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
import { identityPathKey } from '@/shared/lib/pathKey';

export type WorkspaceSwitchSurface = 'folder_grid' | 'preview' | 'object_list' | 'collections';

export type WorkspaceSwitchFallbackClass = 'folderSwitch' | 'objectSwitch';
export interface WorkspaceSwitchEffectsOptions {
  publish?: boolean;
  gameId?: string;
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
const projectedRevisionByGame = new Map<string, number>();
const projectionWaiters = new Map<string, Set<{ revision: number; resolve: () => void }>>();
let projectionListenerReady: Promise<boolean> | null = null;

interface WorkspaceSwitchProjectedEvent {
  game_id: string;
  disk_revision: number;
}

export function recordWorkspaceProjectedRevision(gameId: string, revision: number): void {
  if (!gameId || !Number.isSafeInteger(revision) || revision < 0) {
    return;
  }
  projectedRevisionByGame.set(gameId, Math.max(projectedRevisionByGame.get(gameId) ?? 0, revision));
  const waiters = projectionWaiters.get(gameId);
  if (!waiters) {
    return;
  }
  for (const waiter of waiters) {
    if (waiter.revision <= revision) {
      waiters.delete(waiter);
      waiter.resolve();
    }
  }
  if (waiters.size === 0) {
    projectionWaiters.delete(gameId);
  }
}

export function ensureWorkspaceProjectionListener(): Promise<boolean> {
  projectionListenerReady ??= listen<WorkspaceSwitchProjectedEvent>(
    'workspace_switch:projected',
    ({ payload }) => {
      recordWorkspaceProjectedRevision(payload.game_id, payload.disk_revision);
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

export async function waitForWorkspaceProjection(gameId: string, revision: number): Promise<void> {
  await ensureWorkspaceProjectionListener();
  let retryMs = 250;
  while (true) {
    const snapshot = await commands.getWorkspaceSwitchSnapshot(gameId);
    if (snapshot.projected_revision >= revision) {
      recordWorkspaceProjectedRevision(gameId, snapshot.projected_revision);
      return;
    }
    if ((projectedRevisionByGame.get(gameId) ?? 0) >= revision) {
      return;
    }
    await new Promise<void>((resolve) => {
      const waiter = { revision, resolve };
      const waiters = projectionWaiters.get(gameId) ?? new Set();
      waiters.add(waiter);
      projectionWaiters.set(gameId, waiters);
      window.setTimeout(() => {
        waiters.delete(waiter);
        if (waiters.size === 0 && projectionWaiters.get(gameId) === waiters) {
          projectionWaiters.delete(gameId);
        }
        resolve();
      }, retryMs);
    });
    retryMs = Math.min(retryMs * 2, 2_000);
  }
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

export function buildNodePendingKey(node: WorkspaceNode): string {
  if (isWorkspaceObjectNode(node)) {
    return `object:${node.id}`;
  }

  return `folder:${node.id ?? identityPathKey(node.path) ?? node.path}`;
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
 * no-op — so it replays unconditionally. Thumbnails are identity-keyed and
 * survive a toggle, so nothing is dropped here.
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
  const healthKeys = options.gameId
    ? affectedPaths.map((path) => modHealthKeys.report(options.gameId!, path))
    : [];
  const cancellation = Promise.all([
    descriptor ? cancelRuntimeDescriptorQueries(queryClient, descriptor) : Promise.resolve(),
    ...healthKeys.map((queryKey) => queryClient.cancelQueries({ queryKey })),
  ]);

  applyRuntimeEffects(
    queryClient,
    buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
  );

  const backgroundRefresh = async () => {
    await cancellation;

    const diskRevision =
      'disk_revision' in result && typeof result.disk_revision === 'number'
        ? result.disk_revision
        : null;
    if (options.gameId && diskRevision !== null) {
      await waitForWorkspaceProjection(options.gameId, diskRevision);
    }

    if (options.gameId && !isWorkspaceGameCurrent(options.gameId)) {
      return;
    }

    await Promise.all([
      ...healthKeys.map((queryKey) =>
        queryClient.invalidateQueries({ queryKey, refetchType: 'active' }),
      ),
      descriptor ? publishRuntimeDescriptor(queryClient, descriptor, 'active') : Promise.resolve(),
    ]);
  };

  return backgroundRefresh().catch((error: unknown) => {
    console.error('[WorkspaceSwitch] Background cache refresh failed:', error);
    throw error;
  });
}
