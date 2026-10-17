import { act, fireEvent, render, screen, waitFor, within } from '../../tests/testing/test-utils';
import { listen } from '@tauri-apps/api/event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '../../shared/api/tauri/bindings';
import {
  openImportBatchWizard,
  subscribeImportBatchWizard,
  type ImportBatchLaunchRequest,
} from '@/features/import-batches';
import { toast } from '@/shared/ui/toast';
import { modInboxCommands } from './api';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import ModInboxPage from './ModInboxPage';
import { StrictMode, type ReactNode } from 'react';
import type { AppSettings, ImportBatch } from '../../shared/api/tauri/bindings.gen';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
import type { ModInboxSnapshot } from './types';

const mockSetWorkspaceView = vi.fn();
const mockSetSettingsTab = vi.fn();
const mockSetSelectedObjectFolderPath = vi.fn();
const mockSetGridSelection = vi.fn();

const mockStoreState: {
  activeGameId: string | null;
  setWorkspaceView: typeof mockSetWorkspaceView;
  setSettingsTab: typeof mockSetSettingsTab;
  setSelectedObjectFolderPath: typeof mockSetSelectedObjectFolderPath;
  setGridSelection: typeof mockSetGridSelection;
} = {
  activeGameId: 'game-1',
  setWorkspaceView: mockSetWorkspaceView,
  setSettingsTab: mockSetSettingsTab,
  setSelectedObjectFolderPath: mockSetSelectedObjectFolderPath,
  setGridSelection: mockSetGridSelection,
};

vi.mock('@/app/store', () => ({
  useAppStore: Object.assign(
    (selector: (state: typeof mockStoreState) => unknown) => selector(mockStoreState),
    { getState: () => mockStoreState },
  ),
}));

vi.mock('../../shared/api/tauri/bindings', () => ({
  commands: {
    createModInboxBatch: vi.fn(),
    createModInboxFolder: vi.fn(),
    deleteProcessedModInboxSources: vi.fn(),
    getModInbox: vi.fn(),
    getObject: vi.fn(),
    openModInboxFolder: vi.fn(),
    openInExplorer: vi.fn(),
    getSettings: vi.fn(),
    saveSettings: vi.fn(),

    startModInboxWatcher: vi.fn(),
    stopModInboxWatcher: vi.fn(),
  },
}));

vi.mock('@/features/import-batches/launcher', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../features/import-batches/launcher')>();
  return { ...actual, openImportBatchWizard: vi.fn(actual.openImportBatchWizard) };
});

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

const readySnapshot = {
  gameId: 'game-1',
  rootPath: 'C:/Downloads/Mods/Genshin',
  rootState: 'ready' as const,
  readyEntries: [
    {
      entryKey: 'entry-a',
      name: 'Raiden Pack',
      path: 'C:/Downloads/Mods/Genshin/Raiden Pack',
      kind: 'folder' as const,
      archiveFormat: null,
      sizeBytes: 1024,
      modifiedUnixMs: '1787932800000',
      layout: 'folder_pack' as const,
      detectedRootCount: 2,
      pendingBatchId: null,
    },
    {
      entryKey: 'entry-b',
      name: 'Nahida.zip',
      path: 'C:/Downloads/Mods/Genshin/Nahida.zip',
      kind: 'archive' as const,
      archiveFormat: 'zip',
      sizeBytes: 2048,
      modifiedUnixMs: '1787932800000',
      layout: 'direct_mod' as const,
      detectedRootCount: 1,
      pendingBatchId: null,
    },
  ],
  processedSources: [],
} satisfies ModInboxSnapshot;

const processedSnapshot = {
  ...readySnapshot,
  readyEntries: [],
  processedSources: [
    {
      sourceId: 'source-a',
      name: 'Raiden Pack',
      sourceKind: 'folder' as const,
      originalPath: 'C:/Downloads/Mods/Genshin/Raiden Pack',
      processedPath: 'C:/Downloads/Mods/Genshin/Processed/Raiden Pack',
      processedAt: '2026-08-29T10:00:00Z',
      sourceDeletedAt: null,
      destinations: [
        {
          objectId: 'object-1',
          objectName: 'Raiden Shogun',
          placedPath: 'Characters/Raiden/Mods/DISABLED raiden-pack',
          plannedName: 'DISABLED raiden-pack',
          status: 'done' as const,
        },
      ],
    },
    {
      sourceId: 'source-deleted',
      name: 'Old Pack',
      sourceKind: 'archive' as const,
      originalPath: 'C:/Downloads/Mods/Genshin/Old.zip',
      processedPath: null,
      processedAt: '2026-08-28T10:00:00Z',
      sourceDeletedAt: '2026-08-29T11:00:00Z',
      destinations: [],
    },
  ],
} satisfies ModInboxSnapshot;

const createdBatch: ImportBatch = {
  id: 'old-game-batch',
  gameId: 'game-1',
  flow: 'ready_to_move',
  targetMode: 'auto',
  targetObjectId: null,
  targetSubpath: null,
  status: 'draft',
  sourceArchivePath: null,
  items: [],
  createdAt: '2026-10-11T18:00:00Z',
  updatedAt: '2026-10-11T18:00:00Z',
};

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe('ModInboxPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const portalTarget = document.createElement('div');
    portalTarget.id = 'topbar-actions-portal';
    document.body.appendChild(portalTarget);
    mockStoreState.activeGameId = 'game-1';
    vi.mocked(modInboxCommands.getModInbox).mockResolvedValue(readySnapshot);
    vi.mocked(modInboxCommands.startModInboxWatcher).mockResolvedValue(undefined);
    vi.mocked(modInboxCommands.stopModInboxWatcher).mockResolvedValue(undefined);

    vi.mocked(modInboxCommands.openModInboxFolder).mockResolvedValue(undefined);
    vi.mocked(commands.openInExplorer).mockResolvedValue(undefined as never);
    vi.mocked(listen).mockResolvedValue(vi.fn());
  });

  afterEach(() => {
    document.getElementById('topbar-actions-portal')?.remove();
  });

  it.each(['unmount', 'game round trip'] as const)(
    'does not launch a completed old batch after %s',
    async (retirement) => {
      const pendingBatch = deferred<ImportBatch>();
      const gameTwo = { ...readySnapshot, gameId: 'game-2', rootPath: 'D:/Inbox/Game Two' };
      vi.mocked(modInboxCommands.getModInbox)
        .mockReset()
        .mockImplementation(async (gameId) => (gameId === 'game-1' ? readySnapshot : gameTwo));
      vi.mocked(modInboxCommands.createModInboxBatch).mockReturnValueOnce(pendingBatch.promise);
      const received: ImportBatchLaunchRequest[] = [];
      const unsubscribe = subscribeImportBatchWizard((request) => received.push(request));
      try {
        const view = render(<ModInboxPage />);
        fireEvent.click(await screen.findByRole('button', { name: 'Review selected (2)' }));
        expect(modInboxCommands.createModInboxBatch).toHaveBeenCalledWith({
          gameId: 'game-1',
          entryKeys: ['entry-a', 'entry-b'],
        });
        if (retirement === 'unmount') view.unmount();
        mockStoreState.activeGameId = 'game-2';
        if (retirement === 'unmount') render(<ModInboxPage />);
        else view.rerender(<ModInboxPage />);
        expect(await screen.findByText(gameTwo.rootPath)).toBeInTheDocument();
        if (retirement === 'game round trip') {
          mockStoreState.activeGameId = 'game-1';
          view.rerender(<ModInboxPage />);
          await screen.findByText(readySnapshot.rootPath);
          expect(screen.getByRole('button', { name: 'Review selected (2)' })).toBeEnabled();
        }
        const scanCount = vi.mocked(modInboxCommands.getModInbox).mock.calls.length;
        await act(async () => {
          pendingBatch.resolve(createdBatch);
          await pendingBatch.promise;
        });
        expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(scanCount);
        expect(received).toEqual([]);
        expect(screen.getByRole('checkbox', { name: 'Select Raiden Pack' })).toBeChecked();
      } finally {
        pendingBatch.resolve(createdBatch);
        unsubscribe();
      }
    },
  );

  it('does not launch a batch when the page retires during its refresh', async () => {
    const pendingRefresh = deferred<ModInboxSnapshot>();
    vi.mocked(modInboxCommands.getModInbox)
      .mockReset()
      .mockResolvedValueOnce(readySnapshot)
      .mockReturnValueOnce(pendingRefresh.promise);
    vi.mocked(modInboxCommands.createModInboxBatch).mockResolvedValueOnce(createdBatch);
    const view = render(<ModInboxPage />);
    fireEvent.click(await screen.findByRole('button', { name: 'Review selected (2)' }));
    await waitFor(() => expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(2));
    view.unmount();
    await act(async () => {
      pendingRefresh.resolve(readySnapshot);
      await pendingRefresh.promise;
    });
    expect(openImportBatchWizard).not.toHaveBeenCalled();
  });

  it('does not report an old batch error after unmount', async () => {
    const pendingBatch = deferred<ImportBatch>();
    vi.mocked(modInboxCommands.createModInboxBatch).mockReturnValueOnce(pendingBatch.promise);
    const reportError = vi.spyOn(toast, 'error');
    try {
      const view = render(<ModInboxPage />);
      fireEvent.click(await screen.findByRole('button', { name: 'Review selected (2)' }));
      view.unmount();
      await act(async () => {
        pendingBatch.reject(new Error('Old batch failed'));
        await pendingBatch.promise.catch(() => undefined);
      });
      expect(reportError).not.toHaveBeenCalled();
    } finally {
      reportError.mockRestore();
    }
  });

  it('waits for retired watcher startup and cleanup before starting the remounted watcher', async () => {
    const gameTwo = { ...readySnapshot, gameId: 'game-2', rootPath: 'D:/Inbox/Game Two' };
    const oldStart = deferred<void>();
    const oldStop = deferred<void>();
    const backend: { active: { gameId: string; rootPath: string } | null } = { active: null };
    vi.mocked(modInboxCommands.getModInbox)
      .mockReset()
      .mockImplementation(async (gameId) => (gameId === 'game-1' ? readySnapshot : gameTwo));
    vi.mocked(modInboxCommands.startModInboxWatcher).mockImplementation(async (gameId) => {
      if (gameId === 'game-1') await oldStart.promise;
      backend.active = {
        gameId,
        rootPath: gameId === 'game-1' ? readySnapshot.rootPath : gameTwo.rootPath,
      };
    });
    vi.mocked(modInboxCommands.stopModInboxWatcher).mockImplementation(async (gameId, rootPath) => {
      if (gameId === 'game-1') await oldStop.promise;
      if (backend.active?.gameId === gameId && backend.active.rootPath === rootPath)
        backend.active = null;
    });
    try {
      const oldView = render(<ModInboxPage />);
      await waitFor(() =>
        expect(modInboxCommands.startModInboxWatcher).toHaveBeenCalledWith('game-1'),
      );
      oldView.unmount();
      mockStoreState.activeGameId = 'game-2';
      render(<ModInboxPage />);
      await screen.findByText(gameTwo.rootPath);
      expect(modInboxCommands.startModInboxWatcher).not.toHaveBeenCalledWith('game-2');
      await act(async () => {
        oldStart.resolve();
        await oldStart.promise;
      });
      await waitFor(() =>
        expect(modInboxCommands.stopModInboxWatcher).toHaveBeenCalledWith(
          'game-1',
          readySnapshot.rootPath,
        ),
      );
      expect(modInboxCommands.startModInboxWatcher).not.toHaveBeenCalledWith('game-2');
      await act(async () => {
        oldStop.resolve();
        await oldStop.promise;
      });
      await waitFor(() => expect(backend.active?.gameId).toBe('game-2'));
      expect(backend.active?.rootPath).toBe(gameTwo.rootPath);
    } finally {
      await act(async () => {
        oldStart.resolve();
        oldStop.resolve();
        await oldStart.promise;
        await oldStop.promise;
      });
    }
  });

  it.each(['startup', 'cleanup'] as const)(
    'continues the shared watcher queue after rejected %s',
    async (failure) => {
      const gameTwo = { ...readySnapshot, gameId: 'game-2', rootPath: 'D:/Inbox/Game Two' };
      const warning = vi.spyOn(toast, 'warning');
      const cleanupWarning = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
      vi.mocked(modInboxCommands.getModInbox)
        .mockReset()
        .mockImplementation(async (gameId) => (gameId === 'game-1' ? readySnapshot : gameTwo));
      if (failure === 'startup') {
        vi.mocked(modInboxCommands.startModInboxWatcher).mockRejectedValueOnce(
          new Error('Startup failed'),
        );
      } else {
        vi.mocked(modInboxCommands.stopModInboxWatcher).mockRejectedValueOnce(
          new Error('Cleanup failed'),
        );
      }
      try {
        const oldView = render(<ModInboxPage />);
        await waitFor(() =>
          expect(modInboxCommands.startModInboxWatcher).toHaveBeenCalledWith('game-1'),
        );
        oldView.unmount();
        mockStoreState.activeGameId = 'game-2';
        const newView = render(<ModInboxPage />);
        await waitFor(() =>
          expect(modInboxCommands.startModInboxWatcher).toHaveBeenCalledWith('game-2'),
        );
        expect(modInboxCommands.stopModInboxWatcher).toHaveBeenCalledWith(
          'game-1',
          readySnapshot.rootPath,
        );
        if (failure === 'startup')
          expect(warning).toHaveBeenCalledWith(expect.stringContaining('Startup failed'));
        else
          expect(cleanupWarning).toHaveBeenCalledWith(
            'Could not stop Mod Inbox watcher',
            expect.any(Error),
          );
        newView.unmount();
        await waitFor(() =>
          expect(modInboxCommands.stopModInboxWatcher).toHaveBeenCalledWith(
            'game-2',
            gameTwo.rootPath,
          ),
        );
      } finally {
        warning.mockRestore();
        cleanupWarning.mockRestore();
      }
    },
  );

  it('starts a scoped watcher, refreshes for inbox events, and stops it on unmount', async () => {
    let changeHandler: (() => void) | undefined;
    const unlisten = vi.fn();
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      changeHandler = () => handler({ payload: {} } as never);
      return unlisten;
    });

    try {
      const { unmount } = render(<ModInboxPage />);

      await waitFor(() => {
        expect(modInboxCommands.startModInboxWatcher).toHaveBeenCalledWith('game-1');
        expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(1);
      });
      expect(listen).toHaveBeenCalledWith('mod-inbox://changed', expect.any(Function));

      vi.useFakeTimers();
      changeHandler?.();
      changeHandler?.();
      expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(1);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(600);
      });
      expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(2);

      unmount();
      await act(async () => {
        await Promise.resolve();
      });
      expect(unlisten).toHaveBeenCalledTimes(1);
      expect(modInboxCommands.stopModInboxWatcher).toHaveBeenCalledWith(
        'game-1',
        readySnapshot.rootPath,
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it('replaces the watcher when the ready inbox root changes', async () => {
    let changeHandler: (() => void) | undefined;
    const movedSnapshot = {
      ...readySnapshot,
      rootPath: 'D:/New Inbox',
    };
    vi.mocked(modInboxCommands.getModInbox)
      .mockResolvedValueOnce(readySnapshot)
      .mockResolvedValueOnce(movedSnapshot);
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      changeHandler = () => handler({ payload: {} } as never);
      return () => undefined;
    });

    render(<ModInboxPage />);

    await waitFor(() =>
      expect(modInboxCommands.startModInboxWatcher).toHaveBeenCalledWith('game-1'),
    );

    changeHandler?.();

    await waitFor(() => {
      expect(modInboxCommands.stopModInboxWatcher).toHaveBeenCalledWith(
        'game-1',
        readySnapshot.rootPath,
      );
      expect(modInboxCommands.startModInboxWatcher).toHaveBeenCalledTimes(2);
    });
  });

  it('refreshes pending inbox entries before opening the created batch in the shared wizard', async () => {
    const reservedSnapshot = {
      ...readySnapshot,
      readyEntries: readySnapshot.readyEntries.map((entry) => ({
        ...entry,
        pendingBatchId: 'batch-1',
      })),
    };
    vi.mocked(modInboxCommands.getModInbox)
      .mockResolvedValueOnce(readySnapshot)
      .mockResolvedValueOnce(reservedSnapshot);
    vi.mocked(modInboxCommands.createModInboxBatch).mockResolvedValue({ id: 'batch-1' } as never);
    render(
      <StrictMode>
        <ModInboxPage />
      </StrictMode>,
    );

    await screen.findByText('Raiden Pack');
    expect(screen.queryByText('Mod pack · 2 roots')).not.toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'Select all ready entries' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Select Raiden Pack' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Select Nahida.zip' })).toBeChecked();
    fireEvent.click(screen.getByRole('button', { name: 'Review selected (2)' }));

    await waitFor(() => {
      expect(modInboxCommands.createModInboxBatch).toHaveBeenCalledWith({
        gameId: 'game-1',
        entryKeys: ['entry-a', 'entry-b'],
      });
      expect(openImportBatchWizard).toHaveBeenCalledWith({
        kind: 'existing',
        batchId: 'batch-1',
      });
      expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(2);
    });
    expect(screen.getByRole('checkbox', { name: 'Select Raiden Pack' })).toBeDisabled();
    expect(screen.getByRole('checkbox', { name: 'Select Nahida.zip' })).toBeDisabled();
  });

  it('virtualizes large ready inbox lists instead of mounting every row', async () => {
    const largeReadySnapshot = {
      ...readySnapshot,
      readyEntries: Array.from({ length: 81 }, (_, index) => ({
        ...readySnapshot.readyEntries[0],
        entryKey: `large-entry-${index}`,
        name: `Large Entry ${index}`,
        path: `C:/Downloads/Mods/Genshin/Large Entry ${index}`,
      })),
    } satisfies ModInboxSnapshot;
    vi.mocked(modInboxCommands.getModInbox).mockResolvedValue(largeReadySnapshot);
    const offsetHeight = vi
      .spyOn(HTMLElement.prototype, 'offsetHeight', 'get')
      .mockReturnValue(1_000);
    const offsetWidth = vi
      .spyOn(HTMLElement.prototype, 'offsetWidth', 'get')
      .mockReturnValue(1_000);

    try {
      render(<ModInboxPage />);

      expect(await screen.findByText('Large Entry 0')).toBeInTheDocument();
      expect(screen.queryByText('Large Entry 80')).not.toBeInTheDocument();
    } finally {
      offsetHeight.mockRestore();
      offsetWidth.mockRestore();
    }
  });

  it('coalesces watcher refreshes that arrive while an Inbox scan is active', async () => {
    let changeHandler: (() => void) | undefined;
    let resolveInitialSnapshot!: (snapshot: ModInboxSnapshot) => void;
    const initialRequest = new Promise<ModInboxSnapshot>((resolve) => {
      resolveInitialSnapshot = resolve;
    });
    vi.mocked(modInboxCommands.getModInbox)
      .mockReset()
      .mockReturnValueOnce(initialRequest)
      .mockResolvedValueOnce(readySnapshot);
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      changeHandler = () => handler({ payload: {} } as never);
      return () => undefined;
    });

    vi.useFakeTimers();
    try {
      render(<ModInboxPage />);
      await act(async () => {
        await Promise.resolve();
      });
      expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(1);

      changeHandler?.();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(600);
      });
      expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(1);

      await act(async () => {
        resolveInitialSnapshot(readySnapshot);
        await initialRequest;
      });
      expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });

  it('preserves ready deselection and drops entries that become pending after a watcher refresh', async () => {
    let changeHandler: (() => void) | undefined;
    const refreshedSnapshot = {
      ...readySnapshot,
      readyEntries: [
        { ...readySnapshot.readyEntries[0], pendingBatchId: 'batch-1' },
        readySnapshot.readyEntries[1],
      ],
    };
    vi.mocked(modInboxCommands.getModInbox)
      .mockResolvedValueOnce(readySnapshot)
      .mockResolvedValueOnce(refreshedSnapshot);
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      changeHandler = () => handler({ payload: {} } as never);
      return () => undefined;
    });

    render(<ModInboxPage />);

    const nahidaSelection = await screen.findByRole('checkbox', { name: 'Select Nahida.zip' });
    fireEvent.click(nahidaSelection);
    expect(nahidaSelection).not.toBeChecked();

    changeHandler?.();

    await waitFor(() => {
      expect(screen.getByRole('checkbox', { name: 'Select Raiden Pack' })).toBeDisabled();
      expect(screen.getByRole('checkbox', { name: 'Select Nahida.zip' })).not.toBeChecked();
      expect(screen.getByRole('button', { name: 'Review selected (0)' })).toBeDisabled();
    });
  });

  it('creates a missing inbox and does not offer an open shortcut until it exists', async () => {
    const missingSnapshot = {
      ...readySnapshot,
      rootState: 'missing' as const,
      readyEntries: [],
    };
    vi.mocked(modInboxCommands.getModInbox).mockResolvedValue(missingSnapshot);
    vi.mocked(modInboxCommands.createModInboxFolder).mockResolvedValue(readySnapshot);

    render(<ModInboxPage />);

    expect(
      await screen.findByText('Your Mod Inbox folder does not exist yet.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open Inbox' })).toBeDisabled();
    expect(modInboxCommands.startModInboxWatcher).not.toHaveBeenCalled();

    vi.mocked(openDialog).mockResolvedValueOnce('D:/New/Inbox/Path');
    const settings: AppSettings = {
      revision: 1,
      language: 'en',
      theme: 'dark',
      active_game_id: 'game-1',
      safety: { keywords: [] },
      ai: { enabled: false, has_api_key: false, base_url: null },
      auto_close_launcher: false,
      games: [
        {
          id: 'game-1',
          name: 'Game 1',
          game_type: 0,
          mod_path: 'mods',
          ready_to_move_path: null,
          game_exe: 'game.exe',
          loader_exe: null,
          launch_args: null,
        },
      ],
    };
    vi.mocked(modInboxCommands.getSettings).mockResolvedValueOnce(settings);
    vi.mocked(modInboxCommands.saveSettings).mockResolvedValueOnce({
      settings,
      sync_warning: null,
    });

    fireEvent.click(
      within(document.getElementById('topbar-actions-portal')!).getByRole('button', {
        name: 'Choose Location',
      }),
    );

    await waitFor(() =>
      expect(openDialog).toHaveBeenCalledWith({
        directory: true,
        multiple: false,
        title: 'Choose Location',
      }),
    );
    await waitFor(() =>
      expect(modInboxCommands.saveSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          games: expect.arrayContaining([
            expect.objectContaining({ ready_to_move_path: 'D:/New/Inbox/Path' }),
          ]),
        }),
      ),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Create Folder' }));
    await waitFor(() =>
      expect(modInboxCommands.createModInboxFolder).toHaveBeenCalledWith('game-1'),
    );
    await waitFor(() => expect(screen.getByRole('button', { name: 'Open Inbox' })).toBeEnabled());
    await waitFor(() => expect(modInboxCommands.startModInboxWatcher).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole('button', { name: 'Open Inbox' }));

    expect(modInboxCommands.openModInboxFolder).toHaveBeenCalledWith('game-1');
  });

  it('ignores a stale refresh response after the active game changes', async () => {
    let resolveGameOne!: (snapshot: typeof readySnapshot) => void;
    let resolveGameTwo!: (snapshot: typeof readySnapshot) => void;
    const gameOneRequest = new Promise<typeof readySnapshot>((resolve) => {
      resolveGameOne = resolve;
    });
    const gameTwoRequest = new Promise<typeof readySnapshot>((resolve) => {
      resolveGameTwo = resolve;
    });
    vi.mocked(modInboxCommands.getModInbox)
      .mockReset()
      .mockReturnValueOnce(gameOneRequest)
      .mockReturnValueOnce(gameTwoRequest);

    const { rerender } = render(<ModInboxPage />);
    await waitFor(() => expect(modInboxCommands.getModInbox).toHaveBeenCalledWith('game-1'));

    mockStoreState.activeGameId = 'game-2';
    rerender(<ModInboxPage />);
    await waitFor(() => expect(modInboxCommands.getModInbox).toHaveBeenCalledWith('game-2'));

    resolveGameTwo({
      ...readySnapshot,
      gameId: 'game-2',
      rootPath: 'D:/Inbox/Game Two',
    });
    expect(await screen.findByText('D:/Inbox/Game Two')).toBeInTheDocument();

    await act(async () => {
      resolveGameOne(readySnapshot);
      await gameOneRequest;
    });
    expect(screen.getByText('D:/Inbox/Game Two')).toBeInTheDocument();
    expect(screen.queryByText(readySnapshot.rootPath)).not.toBeInTheDocument();
  });

  it('selects only retained processed sources and confirms that delete preserves destinations and history', async () => {
    vi.mocked(modInboxCommands.getModInbox).mockResolvedValue(processedSnapshot);
    vi.mocked(modInboxCommands.deleteProcessedModInboxSources).mockResolvedValue(processedSnapshot);
    render(<ModInboxPage />);

    fireEvent.click(await screen.findByRole('tab', { name: 'Processed' }));
    expect(
      screen.getByRole('checkbox', { name: 'Select all processed sources' }),
    ).not.toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Select Raiden Pack' })).not.toBeChecked();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Select all processed sources' }));
    fireEvent.click(screen.getByRole('button', { name: 'Delete selected (1)' }));

    expect(
      screen.getByText(
        'Selected retained sources will move to the Recycle Bin. Destination mods and import history will not be deleted.',
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Delete Source' }));

    await waitFor(() => {
      expect(modInboxCommands.deleteProcessedModInboxSources).toHaveBeenCalledWith({
        gameId: 'game-1',
        sourceIds: ['source-a'],
      });
    });
  });

  it('opens processed destinations in Explorer or resolves the stable object ID for in-app navigation', async () => {
    vi.mocked(modInboxCommands.getModInbox).mockResolvedValue(processedSnapshot);
    vi.mocked(commands.getObject).mockResolvedValue({
      id: 'object-1',
      folder_path: 'Characters/Raiden',
    } as never);
    render(<ModInboxPage />);

    fireEvent.click(await screen.findByRole('tab', { name: 'Processed' }));
    fireEvent.click(screen.getByRole('button', { name: 'Open Raiden Shogun in app' }));

    await waitFor(() => {
      expect(commands.getObject).toHaveBeenCalledWith('object-1');
      expect(mockSetWorkspaceView).toHaveBeenCalledWith('mods');
      expect(mockSetSelectedObjectFolderPath).toHaveBeenCalledWith('Characters/Raiden');
      expect(mockSetGridSelection).toHaveBeenCalledWith(
        new Set(['Characters/Raiden/Mods/DISABLED raiden-pack']),
      );
    });

    fireEvent.click(screen.getByRole('button', { name: 'Open Raiden Shogun in Explorer' }));
    expect(commands.openInExplorer).toHaveBeenCalledWith(
      'game-1',
      'Characters/Raiden/Mods/DISABLED raiden-pack',
    );
  });

  it('keeps history accessible when its destination object has been deleted', async () => {
    const unavailableDestinationSnapshot = {
      ...processedSnapshot,
      processedSources: [
        {
          ...processedSnapshot.processedSources[0],
          destinations: [
            {
              ...processedSnapshot.processedSources[0].destinations[0],
              objectId: null,
              objectName: null,
            },
          ],
        },
      ],
    };
    vi.mocked(modInboxCommands.getModInbox).mockResolvedValue(
      unavailableDestinationSnapshot as never,
    );
    render(<ModInboxPage />);

    fireEvent.click(await screen.findByRole('tab', { name: 'Processed' }));

    expect(screen.getByText('Destination unavailable')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open destination in app' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Open destination in Explorer' })).toBeEnabled();
  });

  it('shows the first 20 processed destinations until the user expands the source', async () => {
    const destinations = Array.from({ length: 21 }, (_, index) => ({
      ...processedSnapshot.processedSources[0].destinations[0],
      objectId: `object-${index}`,
      objectName: `Destination ${index}`,
      placedPath: `Characters/Target ${index}`,
      plannedName: `Target ${index}`,
    }));
    vi.mocked(modInboxCommands.getModInbox).mockResolvedValue({
      ...processedSnapshot,
      processedSources: [{ ...processedSnapshot.processedSources[0], destinations }],
    });
    render(<ModInboxPage />);

    fireEvent.click(await screen.findByRole('tab', { name: 'Processed' }));
    expect(screen.getByText('Destination 19')).toBeInTheDocument();
    expect(screen.queryByText('Destination 20')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Show all 21 destinations' }));
    expect(screen.getByText('Destination 20')).toBeInTheDocument();
  });
});
