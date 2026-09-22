/**
 * Hook-free building blocks for Workspace Switch.
 *
 * Everything here is either pure (payload/descriptor/key builders) or a plain
 * async operation, so it can be tested without rendering a component.
 */

import type { QueryClient } from '@tanstack/react-query';
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

const workspaceMutationQueues = new Map<string, Promise<void>>();

export function enqueueWorkspaceGameMutation<T>(
  gameId: string,
  operation: () => Promise<T>,
): Promise<T> {
  const previous = workspaceMutationQueues.get(gameId);
  const result = previous ? previous.catch(() => undefined).then(operation) : operation();
  const completion = result.then(
    () => undefined,
    () => undefined,
  );

  workspaceMutationQueues.set(gameId, completion);
  void completion.then(() => {
    if (workspaceMutationQueues.get(gameId) === completion) {
      workspaceMutationQueues.delete(gameId);
    }
  });

  return result;
}

export function isWorkspaceObjectNode(node: WorkspaceNode): node is WorkspaceObjectNode {
  return node.node_kind === 'object';
}

export function parseRenameConflict(error: unknown): WorkspaceRenameConflictPayload | null {
  const raw = error instanceof Error ? error.message : String(error);
  if (!raw.includes('"type":"RenameConflict"')) {
    return null;
  }

  try {
    return JSON.parse(raw) as WorkspaceRenameConflictPayload;
  } catch {
    return null;
  }
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
): Promise<WorkspaceSwitchResult | null> {
  return enqueueWorkspaceGameMutation(input.game_id, async () => {
    try {
      const result = await commands.executeWorkspaceSwitch(input);
      if (isWorkspaceGameCurrent(input.game_id)) {
        notifyCommittedMutationSyncWarning(result);
      }
      return result;
    } catch (error) {
      if (!isWorkspaceGameCurrent(input.game_id)) {
        return null;
      }
      const renameConflict = parseRenameConflict(error);
      if (renameConflict) {
        const report = await commands
          .reconcileDiskStateCmd(input.game_id, 'ManualRepair', null, true)
          .catch(() => null);
        if (!isWorkspaceGameCurrent(input.game_id)) {
          return null;
        }
        const appStore = useAppStore.getState();
        const appliedReport = report ? appStore.applyFolderConflictReconcileResult(report) : false;
        if (report?.status === 'AppliedWithFolderConflicts' && report.folder_conflicts.length > 0) {
          if (!appliedReport) {
            return null;
          }
          appStore.setRenameConfirmations(input.game_id, []);
          openFolderConflictManagerDialog();
        } else if (
          report?.status === 'NeedsRenameConfirmation' &&
          report.rename_confirmations.length > 0
        ) {
          if (!appliedReport) {
            return null;
          }
          appStore.setRenameConfirmations(input.game_id, report.rename_confirmations);
          openRenameConfirmationDialog();
        } else toast.error(formatAppError(error));
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
  });
}

/** Runs one all-or-nothing object batch through the workspace mutation pipeline. */
export function executeWorkspaceObjectBulkSwitch(
  gameId: string,
  objectIds: string[],
  desiredEnabled: boolean,
): Promise<WorkspaceSwitchResult | null> {
  return enqueueWorkspaceGameMutation(gameId, async () => {
    try {
      const result = await commands.executeWorkspaceObjectBulkSwitch(
        gameId,
        objectIds,
        desiredEnabled,
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
  });
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
): void {
  if (options.gameId && !isWorkspaceGameCurrent(options.gameId)) {
    return;
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

  void backgroundRefresh().catch((error: unknown) => {
    console.error('[WorkspaceSwitch] Background cache refresh failed:', error);
  });
}
