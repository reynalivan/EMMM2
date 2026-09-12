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
import { publishRuntimeDescriptor } from '@/shared/lib/queryRefresh';
import {
  openFolderConflictManagerDialog,
  openRenameConfirmationDialog,
  openWorkspaceFileInUseDialog,
} from '../state/workspaceDialogs';
import { notifyCommittedMutationSyncWarning } from '../../../shared/lib/committedMutationWarning';

export type WorkspaceSwitchSurface = 'folder_grid' | 'preview' | 'object_list' | 'collections';

export type WorkspaceSwitchFallbackClass = 'folderSwitch' | 'objectSwitch';
export interface WorkspaceSwitchEffectsOptions {
  publish?: boolean;
}

export interface WorkspaceRenameConflictPayload {
  type: 'RenameConflict';
  attempted_target: string;
  existing_path: string;
  base_name: string;
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

export function buildNodePendingKey(node: WorkspaceNode): string {
  if (isWorkspaceObjectNode(node)) {
    return `object:${node.id}`;
  }

  return `folder:${node.path}`;
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
export async function executeWorkspaceSwitch(
  input: WorkspaceSwitchInput,
): Promise<WorkspaceSwitchResult | null> {
  try {
    const result = await commands.executeWorkspaceSwitch(input);
    notifyCommittedMutationSyncWarning(result);
    return result;
  } catch (error) {
    const renameConflict = parseRenameConflict(error);
    if (renameConflict) {
      const report = await commands
        .reconcileDiskStateCmd(input.game_id, 'ManualRepair', null, true)
        .catch(() => null);
      if (report?.status === 'AppliedWithFolderConflicts' && report.folder_conflicts.length > 0) {
        useAppStore.getState().setFolderConflicts(input.game_id, report.folder_conflicts);
        openFolderConflictManagerDialog();
      } else if (
        report?.status === 'NeedsRenameConfirmation' &&
        report.rename_confirmations.length > 0
      ) {
        useAppStore.getState().setRenameConfirmations(input.game_id, report.rename_confirmations);
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
}

/**
 * The post-switch cache work every switch shape shares: replay the backend's
 * path rewrites, then publish the refresh scopes.
 *
 * The rewrite list is the backend's own account of what moved — empty for a
 * no-op — so it replays unconditionally. Thumbnails are identity-keyed and
 * survive a toggle, so nothing is dropped here.
 */
export async function applyWorkspaceSwitchEffects(
  queryClient: QueryClient,
  result: WorkspaceSwitchResult,
  fallbackClass: WorkspaceSwitchFallbackClass,
  options: WorkspaceSwitchEffectsOptions = {},
): Promise<void> {
  applyRuntimeEffects(
    queryClient,
    buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
  );

  if (options.publish !== false) {
    await publishRuntimeDescriptor(
      queryClient,
      buildSwitchRefreshDescriptor(result.impact, fallbackClass),
      'active',
    );
  }
}
