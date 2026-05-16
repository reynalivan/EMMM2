import type { QueryClient } from '@tanstack/react-query';
import { useAppStore } from '../../../stores/useAppStore';
import { recordInternalWorkspacePathRewrites, workspaceKeys } from '../useWorkspaceViewModel';
import { dispatchWorkspaceRuntimeEvent } from '../state/workspaceStoreBridge';
import { rewriteWorkspacePathValue } from '../pathRewrite';
import {
  isWorkspaceExplorerNode,
  type WorkspaceExplorerNode,
  type WorkspaceNode,
  type WorkspaceObjectNode,
  type WorkspaceViewModel,
} from '../../../types/workspace';
import type { RuntimeEffectDescriptor } from './descriptor';

type RuntimePathRewrite = RuntimeEffectDescriptor['rewrites'][number];
type WorkspacePathRewriteSource = 'internal' | 'disk_reconcile';

function rewriteExplorerNode(
  node: WorkspaceExplorerNode,
  rewrites: RuntimePathRewrite[],
): WorkspaceExplorerNode {
  const path = rewriteWorkspacePathValue(node.path, rewrites) ?? node.path;
  const ownerObjectFolderPath =
    rewriteWorkspacePathValue(node.owner_object_folder_path, rewrites) ??
    node.owner_object_folder_path;

  return {
    ...node,
    path,
    owner_object_folder_path: ownerObjectFolderPath,
  };
}

function rewriteObjectNode(
  node: WorkspaceObjectNode,
  rewrites: RuntimePathRewrite[],
): WorkspaceObjectNode {
  const folderPath = rewriteWorkspacePathValue(node.folder_path, rewrites) ?? node.folder_path;

  return {
    ...node,
    folder_path: folderPath,
  };
}

function rewriteWorkspaceNode(node: WorkspaceNode, rewrites: RuntimePathRewrite[]): WorkspaceNode {
  if (isWorkspaceExplorerNode(node)) {
    return rewriteExplorerNode(node, rewrites);
  }

  return rewriteObjectNode(node, rewrites);
}

function rewriteWorkspaceViewModel(
  current: WorkspaceViewModel,
  rewrites: RuntimePathRewrite[],
): WorkspaceViewModel {
  return {
    ...current,
    objects: current.objects.map((object) => rewriteObjectNode(object, rewrites)),
    explorer: {
      ...current.explorer,
      self_owner_object_folder_path:
        rewriteWorkspacePathValue(current.explorer.self_owner_object_folder_path, rewrites) ??
        current.explorer.self_owner_object_folder_path,
      ancestor_disabled_path:
        rewriteWorkspacePathValue(current.explorer.ancestor_disabled_path, rewrites) ??
        current.explorer.ancestor_disabled_path,
      children: current.explorer.children.map((child) => rewriteExplorerNode(child, rewrites)),
    },
    preview: {
      ...current.preview,
      selected_path: rewriteWorkspacePathValue(current.preview.selected_path, rewrites) ?? null,
      selected_node: current.preview.selected_node
        ? rewriteWorkspaceNode(current.preview.selected_node, rewrites)
        : null,
    },
    selection: {
      ...current.selection,
      selected_object_folder_path:
        rewriteWorkspacePathValue(current.selection.selected_object_folder_path, rewrites) ?? null,
      explorer_sub_path:
        rewriteWorkspacePathValue(current.selection.explorer_sub_path, rewrites) ?? null,
      selected_mod_path:
        rewriteWorkspacePathValue(current.selection.selected_mod_path, rewrites) ?? null,
    },
  };
}

export function rewriteWorkspaceViewModelCache(
  queryClient: QueryClient,
  rewrites: RuntimePathRewrite[],
): void {
  if (rewrites.length === 0) {
    return;
  }

  queryClient.setQueriesData<WorkspaceViewModel>({ queryKey: workspaceKeys.all }, (current) => {
    if (!current) {
      return current;
    }

    return rewriteWorkspaceViewModel(current, rewrites);
  });
}

export function applyWorkspacePathRewrites(
  queryClient: QueryClient,
  rewrites: RuntimePathRewrite[],
  source: WorkspacePathRewriteSource,
): void {
  if (rewrites.length === 0) {
    return;
  }

  if (source === 'internal') {
    recordInternalWorkspacePathRewrites(rewrites, Date.now());
  }

  const appStore = useAppStore.getState();
  appStore.replaceGridSelections(rewrites);
  dispatchWorkspaceRuntimeEvent({
    type: 'PATHS_REWRITTEN',
    rewrites,
  });
  rewriteWorkspaceViewModelCache(queryClient, rewrites);
}
