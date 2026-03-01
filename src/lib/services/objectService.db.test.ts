import { describe, it, expect, vi, beforeEach } from 'vitest';
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
      game_id: 'genshin',
      safe_mode: true,
      meta_filters: {
        Element: ['Pyro', 'Hydro'],
        Rarity: ['5-Star'],
      },
      sort_by: 'date' as const,
      status_filter: 'enabled' as const,
    };

    vi.mocked(invoke).mockResolvedValue([]);

    // Act
    await getObjects(filter);

    // Assert
    expect(invoke).toHaveBeenCalledWith('get_objects_cmd', { filter });
  });
});
