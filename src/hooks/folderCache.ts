import type { FolderGridResponse, ModFolder, SortField, SortOrder } from '../types/mod';

export type { ModFolder, FolderGridResponse };

export const folderKeys = {
  all: ['mod-folders'] as const,
  list: (modsPath: string, subPath?: string, safeMode?: boolean) =>
    [...folderKeys.all, modsPath, subPath ?? '', safeMode ?? null] as const,
};

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
