import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useThumbnail, thumbnailKeys } from './useThumbnail';
import { commands } from '../lib/bindings';

vi.unmock('@tanstack/react-query');

vi.mock('../lib/bindings', () => ({
  commands: {
    getModThumbnail: vi.fn(),
  },
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn((path) => `asset://localhost/${path}`),
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

  it('TC-41-001: returns converted asset URL when backend provides absolute path', async () => {
    const fakePath = 'C:\\AppData\\cache\\thumbnails\\abc123hash.webp';
    const expectedUrl = `asset://localhost/${fakePath}`;
    vi.mocked(commands.getModThumbnail).mockResolvedValueOnce(fakePath);

    const { result } = renderHook(() => useThumbnail('mock-game-id', 'C:\\Mods\\TestMod'), {
      wrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBe(expectedUrl);
    expect(commands.getModThumbnail).toHaveBeenCalledWith({
      gameId: 'mock-game-id',
      folderPath: 'C:\\Mods\\TestMod',
    });
  });

  it('TC-41-002: returns null when backend has no thumbnail (fallback)', async () => {
    vi.mocked(commands.getModThumbnail).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useThumbnail('mock-game-id', 'C:\\Mods\\NoThumb'), {
      wrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
  });

  it('TC-41-003: does not fetch when enabled=false', () => {
    const { result } = renderHook(() => useThumbnail('mock-game-id', 'C:\\Mods\\TestMod', false), {
      wrapper,
    });

    expect(result.current.isLoading).toBe(false);
    expect(commands.getModThumbnail).not.toHaveBeenCalled();
  });

  it('TC-41-004: uses correct query key factory', () => {
    const key = thumbnailKeys.folder('C:\\Mods\\TestMod');
    expect(key).toEqual(['thumbnails', 'C:\\Mods\\TestMod']);
  });

  it('TC-41-005: returns null when backend returns empty string (falsy)', async () => {
    vi.mocked(commands.getModThumbnail).mockResolvedValueOnce('');

    const { result } = renderHook(() => useThumbnail('mock-game-id', 'C:\\Mods\\TestMod'), {
      wrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    // Hook now coalesces falsy values (including empty string) to null for safety
    // when using convertFileSrc.
    expect(result.current.data).toBeNull();
  });
});
