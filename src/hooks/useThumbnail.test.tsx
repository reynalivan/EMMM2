import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useThumbnail, thumbnailKeys } from './useThumbnail';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('useThumbnail (TC-41)', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    vi.clearAllMocks();
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it('TC-41-001: returns thumbnail URL when backend provides one', async () => {
    const fakeUrl = 'asset://localhost/mods/TestMod/preview.webp';
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(fakeUrl);

    const { result } = renderHook(() => useThumbnail('C:\\Mods\\TestMod'), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBe(fakeUrl);
    expect(invoke).toHaveBeenCalledWith('get_mod_thumbnail', {
      folderPath: 'C:\\Mods\\TestMod',
    });
  });

  it('TC-41-002: returns null when backend has no thumbnail (fallback)', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useThumbnail('C:\\Mods\\NoThumb'), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
  });

  it('TC-41-003: does not fetch when enabled=false', () => {
    const { result } = renderHook(() => useThumbnail('C:\\Mods\\TestMod', false), { wrapper });

    expect(result.current.isLoading).toBe(false);
    expect(invoke).not.toHaveBeenCalled();
  });

  it('TC-41-004: uses correct query key factory', () => {
    const key = thumbnailKeys.folder('C:\\Mods\\TestMod');
    expect(key).toEqual(['thumbnails', 'C:\\Mods\\TestMod']);
  });

  it('TC-41-005: returns empty string when backend returns empty string (falsy, not null)', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce('');

    const { result } = renderHook(() => useThumbnail('C:\\Mods\\TestMod'), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    // Hook uses `res ?? null` which only coalesces null/undefined, not empty string.
    // Callers are responsible for treating falsy values as "no thumbnail".
    expect(result.current.data).toBe('');
  });
});
