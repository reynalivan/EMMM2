import { renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useActiveKeybindings } from './useActiveKeybindings';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { invoke } from '@tauri-apps/api/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(),
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
};

describe('useActiveKeybindings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should not fetch if activeGame is null', () => {
    vi.mocked(useActiveGame).mockReturnValue({ activeGame: null } as unknown as ReturnType<
      typeof useActiveGame
    >);
    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper() });

    expect(result.current.isLoading).toBe(false); // Query stays disabled
    expect(invoke).not.toHaveBeenCalled();
  });

  it('should fetch and return keybindings when activeGame is present', async () => {
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame: { id: 'game-1' },
    } as unknown as ReturnType<typeof useActiveGame>);
    const mockBindings = [{ id: '1', key: 'F1', action: 'test' }];
    vi.mocked(invoke).mockResolvedValue(mockBindings);

    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper() });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(invoke).toHaveBeenCalledWith('get_active_keybindings', { gameId: 'game-1' });
    expect(result.current.keybindings).toEqual(mockBindings);
    expect(result.current.isError).toBe(false);
  });

  it('should handle errors during fetch', async () => {
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame: { id: 'game-2' },
    } as unknown as ReturnType<typeof useActiveGame>);
    vi.mocked(invoke).mockRejectedValue(new Error('Fetch failed'));

    const { result } = renderHook(() => useActiveKeybindings(), { wrapper: createWrapper() });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.keybindings).toEqual([]);
    expect(result.current.isLoading).toBe(false);
  });
});
