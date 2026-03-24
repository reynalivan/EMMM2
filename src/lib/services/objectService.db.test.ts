import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ItemStatus } from '../../types/object';
import { getObjects } from './objectService';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('objectService API tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('passes the correct payload to get_objects_cmd', async () => {
    // Arrange
    const filter = {
      game_id: 'gimi_1',
      safe_mode: false,
      meta_filters: { Element: ['Electro'], Rarity: ['5'] },
      sort_by: 'date' as const,
      status_filter: ItemStatus.Enabled,
      search_query: '',
      object_type: null,
    };

    vi.mocked(invoke).mockResolvedValue({ objects: [], lost_objects: [] });

    // Act
    await getObjects(filter);

    // Assert
    expect(invoke).toHaveBeenCalledWith('get_objects_cmd', { filter });
  });
});
