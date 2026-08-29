import { act, fireEvent, render, screen, waitFor } from '../../tests/testing/test-utils';
import { listen } from '@tauri-apps/api/event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '../../shared/api/tauri/bindings';
import { openImportBatchWizard } from '@/features/import-batches/launcher';
import { modInboxCommands } from './api';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import ModInboxPage from './ModInboxPage';

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

vi.mock('../../app/store/useAppStore', () => ({
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

vi.mock('../import-batches/launcher', () => ({
  openImportBatchWizard: vi.fn(),
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

describe('ModInboxPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockStoreState.activeGameId = 'game-1';
    vi.mocked(modInboxCommands.getModInbox).mockResolvedValue(readySnapshot);
    vi.mocked(modInboxCommands.startModInboxWatcher).mockResolvedValue(undefined);
    vi.mocked(modInboxCommands.stopModInboxWatcher).mockResolvedValue(undefined);

    vi.mocked(modInboxCommands.openModInboxFolder).mockResolvedValue(undefined);
    vi.mocked(commands.openInExplorer).mockResolvedValue(undefined as never);
    vi.mocked(listen).mockResolvedValue(vi.fn());
  });

  it('starts a scoped watcher, refreshes for inbox events, and stops it on unmount', async () => {
    let changeHandler: (() => void) | undefined;
    const unlisten = vi.fn();
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      changeHandler = () => handler({ payload: {} } as never);
      return unlisten;
    });

    const { unmount } = render(<ModInboxPage />);

    await waitFor(() => {
      expect(modInboxCommands.startModInboxWatcher).toHaveBeenCalledWith('game-1');
      expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(1);
    });
    expect(listen).toHaveBeenCalledWith('mod-inbox://changed', expect.any(Function));

    changeHandler?.();
    await waitFor(() => expect(modInboxCommands.getModInbox).toHaveBeenCalledTimes(2));

    unmount();
    await waitFor(() => {
      expect(unlisten).toHaveBeenCalledTimes(1);
      expect(modInboxCommands.stopModInboxWatcher).toHaveBeenCalledWith('game-1');
    });
  });

  it('selects every ready entry and opens the created batch in the shared wizard', async () => {
    vi.mocked(modInboxCommands.createModInboxBatch).mockResolvedValue({ id: 'batch-1' } as never);
    render(<ModInboxPage />);

    await screen.findByText('Raiden Pack');
    fireEvent.click(screen.getByRole('checkbox', { name: 'Select all ready entries' }));
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
    vi.mocked(modInboxCommands.getSettings).mockResolvedValueOnce({
      language: 'en',
      theme: 'dark',
      discord_rpc: true,
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
    } as any);
    vi.mocked(modInboxCommands.saveSettings).mockResolvedValueOnce(undefined as any);

    fireEvent.click(screen.getByRole('button', { name: 'Choose Location' }));

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
});
