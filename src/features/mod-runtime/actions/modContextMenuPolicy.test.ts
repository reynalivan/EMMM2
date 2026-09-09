import type { TFunction } from 'i18next';
import { describe, expect, it } from 'vitest';
import type { WorkspaceCapabilities, WorkspaceExplorerNode } from '@/entities/workspace';
import { buildModContextMenuItems } from './modContextMenuPolicy';

const capabilities: WorkspaceCapabilities = {
  can_toggle: true,
  can_rename: true,
  can_delete: true,
  can_move: true,
  can_toggle_safe: true,
  can_sync: true,
  can_enable_only_this: false,
  can_pin: true,
  can_edit_metadata: false,
  can_reveal_in_explorer: true,
  can_move_category: false,
  can_open_in_explorer: true,
};

const baseFolder = {
  is_enabled: true,
  is_favorite: false,
  is_safe: true,
  node_type: 'VariantContainer',
  can_navigate: false,
  capabilities,
} as WorkspaceExplorerNode;

const translate = ((key: string) => {
  const labels: Record<string, string> = {
    'context.favorite': 'Favorite',
    'context.unfavorite': 'Unfavorite',
  };

  return labels[key] ?? key;
}) as unknown as TFunction;

const handlers = {
  rename: () => undefined,
  toggleEnabled: () => undefined,
  toggleFavorite: () => undefined,
  delete: () => undefined,
};

describe('buildModContextMenuItems', () => {
  it.each([
    { is_favorite: false, expectedLabel: 'Favorite' },
    { is_favorite: true, expectedLabel: 'Unfavorite' },
  ])(
    'uses a localized favorite label when is_favorite=$is_favorite',
    ({ is_favorite, expectedLabel }) => {
      const items = buildModContextMenuItems(translate, { ...baseFolder, is_favorite }, handlers);
      const favoriteItem = items.find((item) => item.id === 'toggle-favorite');

      expect(favoriteItem?.label).toBe(expectedLabel);
    },
  );
});
