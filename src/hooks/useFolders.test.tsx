import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useImportMods, useAutoOrganizeMods, useToggleMod } from './useFolders';
import { invoke } from '@tauri-apps/api/core';
import { toast } from '../stores/useToastStore';
import { reconcileActiveCollection } from '../features/collections/utils/reconcileActiveCollection';
import { refetchCurrentCorridorRuntime } from '../features/collections/utils/refetchCurrentCorridorRuntime';
import { useAppStore } from '../stores/useAppStore';

vi.unmock('@tanstack/react-query');

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

vi.mock('../features/collections/utils/reconcileActiveCollection', () => ({
  reconcileActiveCollection: vi.fn().mockResolvedValue(false),
}));

vi.mock('../features/collections/utils/refetchCurrentCorridorRuntime', () => ({
  refetchCurrentCorridorRuntime: vi.fn().mockResolvedValue(undefined),
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
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd) => {
      if (cmd === 'import_mods_from_paths') {
        return Promise.resolve({
          success: [{ path: 'path/to/extracted' }],
          failures: [],
        });
      }
      return Promise.resolve();
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
    expect(reconcileActiveCollection).toHaveBeenCalled();
  });

  it('handles extraction failures (e.g., password protected or corrupt)', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd) => {
      if (cmd === 'import_mods_from_paths') {
        return Promise.resolve({
          success: [],
          failures: [{ path: 'C:\\Mods\\Encrypted.zip', error: 'PasswordRequired' }],
        });
      }
      return Promise.resolve();
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
    expect(reconcileActiveCollection).toHaveBeenCalled();
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
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd) => {
      if (cmd === 'auto_organize_mods') {
        return Promise.resolve({
          success: [{ path: 'path/to/extracted' }],
          failures: [],
        });
      }
      return Promise.resolve();
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
    expect(reconcileActiveCollection).toHaveBeenCalled();
  });
});

describe('useFolders Hook - useToggleMod runtime drift refresh', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    vi.clearAllMocks();
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    useAppStore.setState({
      activeGameId: 'game-123',
      safeMode: true,
      gridSelection: new Set(),
    });
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it('forces strict corridor runtime refetch after manual toggle', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd) => {
      if (cmd === 'toggle_mod') {
        return Promise.resolve('C:\\Games\\Mods\\DISABLED Raiden');
      }

      return Promise.resolve([]);
    });

    const { result } = renderHook(() => useToggleMod(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        path: 'C:\\Games\\Mods\\Raiden',
        enable: false,
        gameId: 'game-123',
        suppressToast: true,
      });
    });

    expect(refetchCurrentCorridorRuntime).toHaveBeenCalledWith(queryClient, 'game-123');
  });
});
