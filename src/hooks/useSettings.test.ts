import { renderHook } from '@testing-library/react';
import { useSettings } from './useSettings';
import { useQuery, useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useToastStore } from '../stores/useToastStore';
import { vi, describe, it, expect, beforeEach } from 'vitest';

vi.mock('@tanstack/react-query', () => {
  const mUseQueryClient = {
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

vi.mock('../stores/useToastStore', () => {
  return {
    useToastStore: vi.fn(),
  };
});

describe('useSettings', () => {
  const mockAddToast = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
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
        safe_mode: { enabled: false, pin_hash: null, keywords: [], force_exclusive_mode: false },
        ai: { enabled: false, api_key: null, base_url: null },
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
          if (options.onSuccess) options.onSuccess(res, ...args);
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
    expect(result.current.setPinAsync).toBeDefined();
    expect(result.current.verifyPin).toBeDefined();
    expect(result.current.runMaintenance).toBeDefined();

    const mutationCall = vi.mocked(useMutation).mock.calls.find((c) => {
      // checking the onSuccess existence to find saveSettingsMutation
      const callArgs = c[0] as Parameters<typeof useMutation>[0];
      return callArgs.mutationFn?.toString().includes('save_settings');
    });

    expect(mutationCall).toBeDefined();

    const mockOptions = mutationCall![0] as Parameters<typeof useMutation>[0];

    // Simulate mutationFn
    // @ts-expect-error mutationFn is defined
    await mockOptions.mutationFn({ theme: 'dark' });
    expect(invoke).toHaveBeenCalledWith('save_settings', { settings: { theme: 'dark' } });

    // Simulate onSuccess
    // @ts-expect-error onSuccess is defined
    mockOptions.onSuccess(undefined, { theme: 'dark' });
    expect(mockAddToast).toHaveBeenCalledWith(
      'success',
      expect.stringContaining('Configuration updated successfully'),
    );
  });
});
