import type { QueryClient } from '@tanstack/react-query';
import type { FolderGridResponse, ModFolder, SortField, SortOrder } from '../types/mod';
import { workspaceKeys } from '../features/workspace-runtime/useWorkspaceViewModel';
import {
  isWorkspaceExplorerNode,
  type WorkspaceExplorerNode,
  type WorkspaceViewModel,
} from '../types/workspace';

export type { ModFolder, FolderGridResponse };

export const folderKeys = {
  all: ['mod-folders'] as const,
  list: (modsPath: string, subPath?: string, safeMode?: boolean) =>
    [...folderKeys.all, modsPath, subPath ?? '', safeMode ?? null] as const,
};

export function updateFolderCache(
  queryClient: QueryClient,
  pathsToUpdate: string[],
  updater?: (folder: ModFolder) => ModFolder,
  remove: boolean = false,
) {
  if (pathsToUpdate.length === 0) {
    return;
  }

  const queries = queryClient.getQueriesData<FolderGridResponse>({ queryKey: folderKeys.all });
  queries.forEach(([queryKey, data]) => {
    if (!data) {
      return;
    }

    const updatedChildren = remove
      ? data.children.filter((folder) => !pathsToUpdate.includes(folder.path))
      : updater
        ? data.children.map((folder) =>
            pathsToUpdate.includes(folder.path) ? updater(folder) : folder,
          )
        : data.children;

    queryClient.setQueryData(queryKey, {
      ...data,
      children: updatedChildren,
    });
  });

  queryClient.setQueriesData<WorkspaceViewModel>({ queryKey: workspaceKeys.all }, (current) => {
    if (!current) {
      return current;
    }

    const nextChildren = remove
      ? current.explorer.children.filter((folder) => !pathsToUpdate.includes(folder.path))
      : updater
        ? current.explorer.children.map((folder) =>
            pathsToUpdate.includes(folder.path)
              ? ({
                  ...folder,
                  ...updater(folder),
                } as WorkspaceExplorerNode)
              : folder,
          )
        : current.explorer.children;

    const previewNode = current.preview.selected_node;
    const previewMatches =
      isWorkspaceExplorerNode(previewNode) && pathsToUpdate.includes(previewNode.path);
    const nextSelectedNode =
      previewNode && previewMatches && updater && !remove
        ? ({
            ...previewNode,
            ...updater(previewNode),
          } as WorkspaceExplorerNode)
        : remove && previewMatches
          ? null
          : previewNode;

    return {
      ...current,
      explorer: {
        ...current.explorer,
        children: nextChildren,
      },
      preview: {
        ...current.preview,
        selected_path: remove && previewMatches ? null : current.preview.selected_path,
        selected_node: nextSelectedNode,
      },
    };
  });
}

export function sortFolders<TFolder extends ModFolder>(
  folders: TFolder[],
  field: SortField,
  order: SortOrder,
): TFolder[] {
  const sortGroup = (group: TFolder[]): TFolder[] => {
    const sorted = [...group].sort((left, right) => {
      if (left.is_favorite !== right.is_favorite) {
        return left.is_favorite ? -1 : 1;
      }

      const comparison = (() => {
        switch (field) {
          case 'name':
            return left.name.localeCompare(right.name, undefined, { sensitivity: 'base' });
          case 'modified_at':
            return left.modified_at - right.modified_at;
          case 'size_bytes':
            return left.size_bytes - right.size_bytes;
          default:
            return 0;
        }
      })();

      if (comparison !== 0) {
        return comparison;
      }

      return left.name.localeCompare(right.name, undefined, { sensitivity: 'base' });
    });

    if (order === 'desc') {
      sorted.reverse();
    }

    return sorted;
  };

  const containers = folders.filter((folder) => folder.node_type === 'ContainerFolder');
  const packs = folders.filter((folder) => folder.node_type !== 'ContainerFolder');

  return [...sortGroup(containers), ...sortGroup(packs)];
}
