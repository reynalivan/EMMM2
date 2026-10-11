import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/app/store';
import { commands } from '../../../shared/api/tauri/bindings';
import FolderConflictManager from './FolderConflictManager';
import { FileInUseDialog } from '@/features/file-watcher';

const toastError = vi.hoisted(() => vi.fn());
vi.mock('@/shared/ui/toast', () => ({ toast: { error: toastError } }));

const notifyCommittedMutationSyncWarning = vi.fn();

vi.mock('../../../shared/lib/committedMutationWarning', () => ({
  notifyCommittedMutationSyncWarning: (...args: unknown[]) =>
    notifyCommittedMutationSyncWarning(...args),
}));

vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: {
    getFolderConflictDetails: vi.fn(),
    resolveFolderNameConflict: vi.fn(),
    trashFolderConflictCandidate: vi.fn(),
    getFileLockInfo: vi.fn(),
    dismissFileLockRecovery: vi.fn(),
  },
}));

const getFolderConflictDetails = vi.mocked(commands.getFolderConflictDetails);
const trashFolderConflictCandidate = vi.mocked(commands.trashFolderConflictCandidate);

const renderManager = (showFileLockDialog = false) =>
  render(
    <>
      <FolderConflictManager />
      {showFileLockDialog && <FileInUseDialog />}
    </>,
    {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    },
  );

describe('FolderConflictManager pending Trash actions', () => {
  beforeAll(() => {
    HTMLDialogElement.prototype.showModal = vi.fn(function mock(this: HTMLDialogElement) {
      this.open = true;
    });
    HTMLDialogElement.prototype.close = vi.fn(function mock(this: HTMLDialogElement) {
      this.open = false;
    });
  });

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(commands.getFileLockInfo).mockImplementation(async (id) => ({
      recovery_id: id,
      path: 'C:/Mods/DISABLED Alice/Blue',
      status: 'unknown',
      message: null,
      processes: [],
      action_outcome: null,
    }));
    vi.mocked(commands.dismissFileLockRecovery).mockResolvedValue(undefined);
    useAppStore.setState({
      activeGameId: 'game-1',
      workspaceDialogState: { kind: 'folderConflicts' },
      folderConflictReportsByGame: {},
      folderConflictsByGame: {
        'game-1': [
          {
            group_id: 'group-1',
            identity: 'alice/blue',
            display_name: 'Blue',
            candidates: [
              {
                path: 'C:/Mods/Alice/Blue',
                folder_name: 'Blue',
                base_name: 'Blue',
                is_enabled: true,
              },
              {
                path: 'C:/Mods/Alice/DISABLED Blue',
                folder_name: 'DISABLED Blue',
                base_name: 'Blue',
                is_enabled: false,
              },
              {
                path: 'C:/Mods/DISABLED Alice/Blue',
                folder_name: 'Blue',
                base_name: 'Blue',
                is_enabled: false,
              },
            ],
          },
        ],
      },
    });
  });

  it('keeps Trash actions local until the yellow Apply button is clicked', async () => {
    getFolderConflictDetails.mockResolvedValue([]);
    trashFolderConflictCandidate.mockResolvedValue({ reconcile: null, sync_warning: null });
    renderManager();

    const trashActions = await screen.findAllByRole('button', { name: 'Mark as Trash' });
    expect(trashActions).toHaveLength(2);
    fireEvent.click(trashActions[0]);
    fireEvent.click(trashActions[1]);

    expect(trashFolderConflictCandidate).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Mark 2 folders for Trash' })).toBeEnabled();
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Mark 2 folders for Trash' }));

    await waitFor(() => expect(trashFolderConflictCandidate).toHaveBeenCalledTimes(2));
    expect(trashFolderConflictCandidate).toHaveBeenNthCalledWith(
      1,
      'game-1',
      'C:/Mods/Alice/DISABLED Blue',
      null,
    );
    expect(trashFolderConflictCandidate).toHaveBeenNthCalledWith(
      2,
      'game-1',
      'C:/Mods/DISABLED Alice/Blue',
      null,
    );
    expect(notifyCommittedMutationSyncWarning).toHaveBeenCalledTimes(2);
  });

  it('switches each non-kept folder between Rename and Mark as Trash', async () => {
    getFolderConflictDetails.mockResolvedValue([]);
    renderManager();

    expect(await screen.findAllByRole('textbox')).toHaveLength(2);
    const trashActions = screen.getAllByRole('button', { name: 'Mark as Trash' });
    fireEvent.click(trashActions[0]);

    expect(screen.getAllByRole('textbox')).toHaveLength(1);
    expect(
      screen.getByText('This folder will be moved to Trash when you apply the changes.'),
    ).toBeInTheDocument();
    fireEvent.click(screen.getAllByRole('button', { name: 'Rename' })[0]);

    expect(screen.getAllByRole('textbox')).toHaveLength(2);
    expect(trashFolderConflictCandidate).not.toHaveBeenCalled();
  });

  it('keeps retrying the locked rename step without repeating a completed Trash action', async () => {
    getFolderConflictDetails.mockResolvedValue([]);
    trashFolderConflictCandidate.mockResolvedValue({ reconcile: null, sync_warning: null });
    const lock = {
      type: 'FileLock' as const,
      payload: { path: 'C:/Mods/DISABLED Alice/Blue', recovery_id: 'rename-context' },
    };
    vi.mocked(commands.resolveFolderNameConflict).mockRejectedValue(lock);
    renderManager(true);
    fireEvent.click((await screen.findAllByRole('button', { name: 'Mark as Trash' }))[0]);
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'Blue Other' } });
    fireEvent.click(screen.getByRole('button', { name: 'Apply 2 changes' }));
    await screen.findByTestId('file-lock-dialog');
    for (let attempt = 1; attempt <= 2; attempt++) {
      await waitFor(() => expect(screen.getByTestId('file-lock-retry')).toBeEnabled());
      fireEvent.click(screen.getByTestId('file-lock-retry'));
      await waitFor(() =>
        expect(commands.resolveFolderNameConflict).toHaveBeenCalledTimes(attempt + 1),
      );
      await waitFor(() => expect(screen.getByTestId('file-lock-retry')).toBeEnabled());
      expect(screen.getByTestId('file-lock-dialog')).toBeInTheDocument();
      expect(trashFolderConflictCandidate).toHaveBeenCalledTimes(1);
    }
    expect(commands.resolveFolderNameConflict).toHaveBeenLastCalledWith(
      'game-1',
      'group-1',
      [
        { path: 'C:/Mods/Alice/Blue', base_name: 'Blue' },
        { path: 'C:/Mods/DISABLED Alice/Blue', base_name: 'Blue Other' },
      ],
      'rename-context',
    );
    fireEvent.click(screen.getByTestId('file-lock-cancel'));
    await waitFor(() => expect(screen.queryByTestId('file-lock-dialog')).not.toBeInTheDocument());
    await waitFor(() =>
      expect(commands.dismissFileLockRecovery).toHaveBeenCalledWith('rename-context'),
    );
    expect(trashFolderConflictCandidate).toHaveBeenCalledTimes(1);
    expect(commands.resolveFolderNameConflict).toHaveBeenCalledTimes(3);
    expect(toastError).not.toHaveBeenCalled();
  });
});
