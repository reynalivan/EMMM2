import { describe, expect, it } from 'vitest';
import type { ModFolder } from '@/entities/game-object';
import { sortFolders } from './folderCache';

function createFolder(overrides: Partial<ModFolder> = {}): ModFolder {
  return {
    node_type: 'FlatModRoot',
    classification_reasons: [],
    id: 'folder-1',
    owner_object_id: 'object-1',
    owner_object_folder_path: 'Objects/Alpha',
    name: 'Alpha Mod',
    folder_name: 'Alpha Mod',
    path: 'Objects/Alpha/Alpha Mod',
    is_enabled: false,
    is_directory: true,
    thumbnail_path: null,
    modified_at: 0,
    size_bytes: 0,
    has_info_json: false,
    is_favorite: false,
    is_misplaced: false,
    is_safe: true,
    is_safety_classified: true,
    contains_safe_mods: true,
    contains_unsafe_mods: false,
    metadata: null,
    category: 'Character',
    conflict_group_id: null,
    conflict_state: null,
    warnings: [],
    ...overrides,
  };
}

describe('sortFolders', () => {
  it.each([
    { field: 'name' as const, order: 'asc' as const },
    { field: 'name' as const, order: 'desc' as const },
    { field: 'modified_at' as const, order: 'asc' as const },
    { field: 'modified_at' as const, order: 'desc' as const },
    { field: 'size_bytes' as const, order: 'asc' as const },
    { field: 'size_bytes' as const, order: 'desc' as const },
  ])('keeps favorites first for $field $order sorting', ({ field, order }) => {
    const favoritePack = createFolder({
      id: 'favorite-pack',
      name: 'Favorite Pack',
      node_type: 'ModPackRoot',
      is_favorite: true,
      modified_at: 1,
      size_bytes: 1,
    });
    const regularContainer = createFolder({
      id: 'regular-container',
      name: 'Regular Container',
      node_type: 'ContainerFolder',
      modified_at: 2,
      size_bytes: 2,
    });

    expect(sortFolders([regularContainer, favoritePack], field, order)[0]?.id).toBe(
      'favorite-pack',
    );
  });

  it('sorts items inside each priority tier using the selected direction', () => {
    const folders = [
      createFolder({ id: 'favorite-z', name: 'Favorite Z', is_favorite: true }),
      createFolder({ id: 'regular-a', name: 'Regular A' }),
      createFolder({ id: 'favorite-a', name: 'Favorite A', is_favorite: true }),
      createFolder({ id: 'regular-z', name: 'Regular Z' }),
    ];

    expect(sortFolders(folders, 'name', 'desc').map((folder) => folder.id)).toEqual([
      'favorite-z',
      'favorite-a',
      'regular-z',
      'regular-a',
    ]);
  });
});
