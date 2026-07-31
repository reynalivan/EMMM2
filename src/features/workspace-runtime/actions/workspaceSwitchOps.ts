/**
 * Hook-free building blocks for Workspace Switch.
 *
 * Everything here is either pure (payload/descriptor/key builders) or a plain
 * async operation, so it can be tested without rendering a component.
 */

import type { QueryClient } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import { extractFileInUsePayload, formatAppError } from '../../../lib/appError';
import { toast } from '../../../stores/useToastStore';
import { thumbnailKeys } from '../../../hooks/useThumbnail';
import type {
  WorkspaceExplorerNode,
  WorkspaceImpact,
  WorkspaceNode,
  WorkspaceObjectNode,
  WorkspaceSwitchInput,
  WorkspaceSwitchResult,
} from '../../../types/workspace';
import { applyRuntimeEffects } from '../optimistic/applyOptimisticEffects';
import {
  buildQueryRemovalDescriptor,
  buildRuntimeMutationDescriptor,
  buildRefreshDescriptor,
  buildWorkspacePathRewritesDescriptor,
} from '../optimistic/descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from '../optimistic/descriptor';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import {
  openWorkspaceConflictDialog,
  openWorkspaceFileInUseDialog,
} from '../state/workspaceDialogs';

export type WorkspaceSwitchSurface =
  'folder_grid' | 'preview' | 'object_list' | 'collections' | 'corridor';

export type WorkspaceSwitchFallbackClass = 'folderSwitch' | 'objectSwitch';

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

/**
 * Explorer switches fold the thumbnail drop and path rewrites into a single
 * effect descriptor; the refreshed counts come from the follow-up refetch.
 */
export function buildExplorerSwitchEffectDescriptor(
  node: WorkspaceExplorerNode,
  impact: WorkspaceImpact,
) {
  return mergeRuntimeEffectDescriptors(
    buildQueryRemovalDescriptor([thumbnailKeys.folder(node.path)], []),
    buildWorkspacePathRewritesDescriptor(impact.rewrites, []),
  );
}

/** Runs the switch command, routing known failures to their dialogs. */
export async function executeWorkspaceSwitch(
  input: WorkspaceSwitchInput,
): Promise<WorkspaceSwitchResult | null> {
  try {
    return await commands.executeWorkspaceSwitch(input);
  } catch (error) {
    const renameConflict = parseRenameConflict(error);
    if (renameConflict) {
      openWorkspaceConflictDialog(renameConflict);
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
 * Shared post-switch cache work: drop the stale thumbnail, replay path
 * rewrites when the target actually moved, then publish the refresh scopes.
 */
export async function applyWorkspaceSwitchEffects(
  queryClient: QueryClient,
  result: WorkspaceSwitchResult,
  previousPath: string,
  fallbackClass: WorkspaceSwitchFallbackClass,
): Promise<void> {
  applyRuntimeEffects(
    queryClient,
    buildQueryRemovalDescriptor([thumbnailKeys.folder(previousPath)], []),
  );

  if (result.primary_path !== previousPath) {
    applyRuntimeEffects(
      queryClient,
      buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
    );
  }

  await publishRuntimeDescriptor(
    queryClient,
    buildSwitchRefreshDescriptor(result.impact, fallbackClass),
    'active',
  );
}

/** "Enable only this" touches a set of siblings, so rewrites always replay. */
export async function applyEnableOnlyThisEffects(
  queryClient: QueryClient,
  result: WorkspaceSwitchResult,
): Promise<void> {
  if (result.changed_folder_paths.length > 0) {
    applyRuntimeEffects(
      queryClient,
      buildQueryRemovalDescriptor(
        result.changed_folder_paths.map((path) => thumbnailKeys.folder(path)),
        [],
      ),
    );
  }

  applyRuntimeEffects(
    queryClient,
    buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
  );
  await publishRuntimeDescriptor(
    queryClient,
    buildSwitchRefreshDescriptor(result.impact, 'folderSwitch'),
    'active',
  );
}
