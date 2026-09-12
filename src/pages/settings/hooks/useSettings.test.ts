import { renderHook } from '@testing-library/react';
import { useSettings } from '@/entities/settings';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useToastStore } from '@/shared/ui/toast';
import { vi, describe, it, expect, beforeEach } from 'vitest';

vi.mock('@tanstack/react-query', () => {
  const mUseQueryClient = {
    getQueryData: vi.fn(),
    setQueryData: vi.fn(),
    invalidateQueries: vi.fn(),
  };
  return {
    useQueryClient: vi.fn(() => mUseQueryClient),
    useQuery: vi.fn(),
    useMutation: vi.fn(),
  };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@/shared/ui/toast', () => {
  return {
    useToastStore: vi.fn(),
  };
});

describe('useSettings', () => {
  const mockAddToast = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === 'save_settings') {
        return {
          settings: { ...(args as { settings: object }).settings, revision: 1 },
          sync_warning: null,
        };
      }
      return null;
    });
    vi.mocked(useToastStore).mockReturnValue({
      addToast: mockAddToast,
      toasts: [],
      removeToast: vi.fn(),
    });

    // Default mock implementation for useQuery
    vi.mocked(useQuery).mockReturnValue({
      data: {
        theme: 'light',
        language: 'en',
        games: [],
        active_game_id: null,
        safety: { keywords: [] },
        ai: { enabled: false, has_api_key: false, base_url: null },
      },
      isLoading: false,
      error: null,
    } as unknown as ReturnType<typeof useQuery>);

    // Default mock implementation for useMutation
    vi.mocked(useMutation).mockImplementation(((options: Parameters<typeof useMutation>[0]) => {
      // Simulate typical mutation object return
      return {
        mutate: options.mutationFn,
        mutateAsync: async (...args: unknown[]) => {
          // @ts-expect-error options has mutationFn
          const res = await options.mutationFn(...args);
          // @ts-expect-error options has onSuccess
          if (options.onSuccess) await options.onSuccess(res, ...args);
          return res;
        },
      } as unknown as ReturnType<typeof useMutation>;
    }) as unknown as typeof useMutation);
  });

  it('should return settings data from useQuery', () => {
    const { result } = renderHook(() => useSettings());
    expect(result.current.settings).toBeDefined();
    expect(result.current.settings?.theme).toBe('light');
    expect(result.current.isLoading).toBe(false);
  });

  it('should call invoke to fetch settings', async () => {
    renderHook(() => useSettings());
    // We can't easily test the exact queryFn without extracting it,
    // but we can just test if invoke was set up
    const queryCall = vi.mocked(useQuery).mock.calls[0]?.[0];
    expect(queryCall).toBeDefined();
    expect(queryCall.queryKey).toEqual(['settings']);

    vi.mocked(invoke).mockResolvedValueOnce({ theme: 'dark' });
    const res = await (queryCall as unknown as { queryFn: () => Promise<unknown> }).queryFn();
    expect(invoke).toHaveBeenCalledWith('get_settings');
    expect(res).toEqual({ theme: 'dark' });
  });

  it('should mutate settings properly and show toast', async () => {
    const { result } = renderHook(() => useSettings());

    // Test that the mutations were set up
    expect(result.current.saveSettingsAsync).toBeDefined();
    expect(result.current.runMaintenance).toBeDefined();

    await result.current.saveSettingsAsync({ theme: 'dark' } as never);
    expect(invoke).toHaveBeenCalledWith('save_settings', { settings: { theme: 'dark' } });
    expect(mockAddToast).toHaveBeenCalledWith('success', expect.any(String));
  });

  it('updateTheme merges with current settings and invokes save_settings', async () => {
    const { result } = renderHook(() => useSettings());

    expect(result.current.updateTheme).toBeDefined();
    expect(result.current.updateTheme.mutateAsync).toBeDefined();

    await result.current.updateTheme.mutateAsync('onyx');

    expect(invoke).toHaveBeenCalledWith('save_settings', {
      settings: expect.objectContaining({
        theme: 'onyx',
        language: 'en',
      }),
    });
  });

  it('refreshes safety-dependent views when classification keywords change', async () => {
    const queryClient = vi.mocked(useQueryClient)();
    vi.mocked(queryClient.getQueryData).mockReturnValue({
      theme: 'light',
      language: 'en',
      games: [],
      active_game_id: null,
      safety: { keywords: [] },
      ai: { enabled: false, has_api_key: false, base_url: null },
    });
    const { result } = renderHook(() => useSettings());

    await result.current.saveSettingsAsync({
      theme: 'light',
      language: 'en',
      games: [],
      active_game_id: null,
      safety: { keywords: ['private'] },
      ai: { enabled: false, has_api_key: false, base_url: null },
    } as never);

    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['workspace', 'mods'],
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['v2-collections'],
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: ['v2-collection-runtime'],
      refetchType: 'active',
    });
  });
});
