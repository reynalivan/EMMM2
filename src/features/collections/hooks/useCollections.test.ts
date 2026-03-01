import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { createWrapper } from '../../../testing/test-utils';
import { useCollections } from './useCollections';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('useCollections', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('loads collections for active game id', async () => {
    vi.mocked(invoke).mockResolvedValue([
      {
        id: 'c-1',
        name: 'Abyss Team',
        game_id: 'g-1',
        is_safe_context: true,
        member_count: 5,
      },
    ]);

    const { result } = renderHook(() => useCollections('g-1'), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledWith('list_collections', { gameId: 'g-1' });
    expect(result.current.data?.[0].name).toBe('Abyss Team');
  });
});
