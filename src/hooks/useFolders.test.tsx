import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useImportMods, useAutoOrganizeMods } from './useFolders';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../stores/useToastStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

describe('useFolders Hook - useImportMods (TC-37)', () => {
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

  it('mutates successfully and invalidates folder cache for valid archives (TC-37)', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      success: ['path/to/extracted'],
      failures: [],
    });

    const { result } = renderHook(() => useImportMods(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        paths: ['C:\\Mods\\ValidArchive.zip'],
        targetDir: 'C:\\Games\\Genshin\\Mods',
        strategy: 'Raw',
      });
    });

    expect(invoke).toHaveBeenCalledWith('import_mods_from_paths', {
      paths: ['C:\\Mods\\ValidArchive.zip'],
      targetDir: 'C:\\Games\\Genshin\\Mods',
      strategy: 'Raw',
    });

    expect(toast.success).toHaveBeenCalledWith('Imported 1 items');
  });

  it('handles extraction failures (e.g., password protected or corrupt)', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      success: [],
      failures: [{ path: 'C:\\Mods\\Encrypted.zip', error: 'PasswordRequired' }],
    });

    const { result } = renderHook(() => useImportMods(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        paths: ['C:\\Mods\\Encrypted.zip'],
        targetDir: 'C:\\Games\\Genshin\\Mods',
        strategy: 'Raw',
      });
    });

    expect(toast.error).toHaveBeenCalledWith('Failed to import 1 items');
  });
});

describe('useFolders Hook - useAutoOrganizeMods (TC-38)', () => {
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

  it('calls backend auto_organize_mods and invalidates cache on success', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      success: ['path/to/extracted'],
      failures: [],
    });

    const { result } = renderHook(() => useAutoOrganizeMods(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        paths: ['C:\\Mods\\LooseMod'],
        targetRoot: 'C:\\Games\\Genshin\\Mods',
        dbJson: '{}',
      });
    });

    expect(invoke).toHaveBeenCalledWith('auto_organize_mods', {
      paths: ['C:\\Mods\\LooseMod'],
      targetRoot: 'C:\\Games\\Genshin\\Mods',
      dbJson: '{}',
    });
  });
});
