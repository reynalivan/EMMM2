import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/app/store';
import { commands } from '../../../shared/api/tauri/bindings';
import FolderConflictManager from './FolderConflictManager';

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
  },
}));

const getFolderConflictDetails = vi.mocked(commands.getFolderConflictDetails);
const trashFolderConflictCandidate = vi.mocked(commands.trashFolderConflictCandidate);

const renderManager = () =>
  render(<FolderConflictManager />, {
    wrapper: ({ children }) => (
      <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
    ),
  });

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
    useAppStore.setState({
      activeGameId: 'game-1',
      workspaceDialogState: { kind: 'folderConflicts' },
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
    );
    expect(trashFolderConflictCandidate).toHaveBeenNthCalledWith(
      2,
      'game-1',
      'C:/Mods/DISABLED Alice/Blue',
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
});
