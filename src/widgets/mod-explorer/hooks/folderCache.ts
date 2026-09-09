import type { ModFolder } from '@/entities/game-object';
import type { SortField, SortOrder } from '@/entities/mod';

export const folderKeys = {
  all: ['mod-folders'] as const,
};

export function sortFolders<TFolder extends ModFolder>(
  folders: TFolder[],
  field: SortField,
  order: SortOrder,
): TFolder[] {
  return [...folders].sort((left, right) => {
    // Favorites are a pinned priority and must stay above regular items even
    // when the selected sort direction is descending.
    if (left.is_favorite !== right.is_favorite) {
      return left.is_favorite ? -1 : 1;
    }

    // Keep the existing container-first grouping within each favorite tier.
    const leftIsContainer = left.node_type === 'ContainerFolder';
    const rightIsContainer = right.node_type === 'ContainerFolder';
    if (leftIsContainer !== rightIsContainer) {
      return leftIsContainer ? -1 : 1;
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
      return order === 'desc' ? -comparison : comparison;
    }

    const nameComparison = left.name.localeCompare(right.name, undefined, {
      sensitivity: 'base',
    });
    return order === 'desc' ? -nameComparison : nameComparison;
  });
}
