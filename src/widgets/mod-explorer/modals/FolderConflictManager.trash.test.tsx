import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '../../../app/store/useAppStore';
import FolderConflictManager from './FolderConflictManager';

const notifyCommittedMutationSyncWarning = vi.fn();

vi.mock('../../../shared/lib/committedMutationWarning', () => ({
  notifyCommittedMutationSyncWarning: (...args: unknown[]) =>
    notifyCommittedMutationSyncWarning(...args),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path) => path),
}));

const renderManager = () =>
  render(<FolderConflictManager />, {
    wrapper: ({ children }) => (
      <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
    ),
  });

describe('FolderConflictManager Trash flow', () => {
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

  it('identifies the exact folder in the Trash action and confirmation', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') {
        return Promise.resolve([
          {
            path: 'C:/Mods/Alice/DISABLED Blue',
            folder_name: 'DISABLED Blue',
            is_enabled: false,
            total_size: 2048,
            file_count: 1,
            files: [],
            thumbnail_path: null,
            partial: false,
            warnings: [],
          },
        ]);
      }
      return Promise.resolve();
    });
    renderManager();

    const trashButton = await screen.findByRole('button', {
      name: 'Move “DISABLED Blue” to Trash — C:/Mods/Alice/DISABLED Blue',
    });
    expect(
      screen.getByRole('button', { name: 'Move “Blue” to Trash — C:/Mods/Alice/Blue' }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', {
        name: 'Move “Blue” to Trash — C:/Mods/DISABLED Alice/Blue',
      }),
    ).toBeInTheDocument();
    fireEvent.click(trashButton);

    const confirmation = screen.getByRole('alertdialog', { name: 'Move folder to Trash?' });
    expect(within(confirmation).getByText('C:/Mods/Alice/DISABLED Blue')).toBeInTheDocument();
    expect(within(confirmation).getByText(/1 file$/)).toBeInTheDocument();
    const cancelButton = within(confirmation).getByRole('button', { name: 'Cancel' });
    const continueButton = within(confirmation).getByRole('button', {
      name: 'Move “DISABLED Blue” to Trash & Continue',
    });
    expect(cancelButton).toHaveFocus();
    fireEvent.keyDown(confirmation, { key: 'Tab', shiftKey: true });
    expect(continueButton).toHaveFocus();
    fireEvent.keyDown(confirmation, { key: 'Tab' });
    expect(cancelButton).toHaveFocus();

    fireEvent.keyDown(confirmation, { key: 'Escape' });
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(trashButton).toHaveFocus();
  });

  it('treats a committed Trash reconcile failure as a warning, not a failed delete', async () => {
    const committedResult = {
      reconcile: null,
      sync_warning: {
        kind: 'ReconcileFailed',
        message: 'Projection refresh is pending',
      },
    };
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') return Promise.resolve([]);
      if (command === 'trash_folder_conflict_candidate') return Promise.resolve(committedResult);
      return Promise.resolve();
    });
    renderManager();

    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Move “DISABLED Blue” to Trash — C:/Mods/Alice/DISABLED Blue',
      }),
    );
    fireEvent.click(
      within(screen.getByRole('alertdialog')).getByRole('button', {
        name: 'Move “DISABLED Blue” to Trash & Continue',
      }),
    );

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('trash_folder_conflict_candidate', {
        gameId: 'game-1',
        path: 'C:/Mods/Alice/DISABLED Blue',
      }),
    );
    expect(notifyCommittedMutationSyncWarning).toHaveBeenCalledWith(committedResult);
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  });

  it('does not present missing folder metadata as a zero-size Trash total', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') return Promise.resolve([]);
      return Promise.resolve();
    });
    renderManager();

    expect(await screen.findAllByText('Folder details unavailable')).toHaveLength(3);
    fireEvent.click(
      screen.getByRole('button', {
        name: 'Move “DISABLED Blue” to Trash — C:/Mods/Alice/DISABLED Blue',
      }),
    );

    const confirmation = screen.getByRole('alertdialog');
    expect(within(confirmation).getByText('Folder details unavailable')).toBeInTheDocument();
    expect(within(confirmation).queryByText(/0 B/)).not.toBeInTheDocument();
  });

  it('labels partial folder metadata instead of presenting it as complete', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') {
        return Promise.resolve([
          {
            path: 'C:/Mods/Alice/DISABLED Blue',
            folder_name: 'DISABLED Blue',
            is_enabled: false,
            total_size: 1024,
            file_count: 2,
            files: [],
            thumbnail_path: null,
            partial: true,
            warnings: ['C:/Mods/Alice/DISABLED Blue/locked.bin'],
          },
        ]);
      }
      return Promise.resolve();
    });
    renderManager();

    expect(await screen.findByText('Some folder details could not be read.')).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole('button', {
        name: 'Move “DISABLED Blue” to Trash — C:/Mods/Alice/DISABLED Blue',
      }),
    );

    const confirmation = screen.getByRole('alertdialog');
    expect(within(confirmation).getByText('1 KB · 2 files')).toBeInTheDocument();
    expect(
      within(confirmation).getByText('Some folder details could not be read.'),
    ).toBeInTheDocument();
  });

  it('closes a stale Trash confirmation when the active game changes', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') return Promise.resolve([]);
      return Promise.resolve();
    });
    renderManager();

    fireEvent.click(
      await screen.findByRole('button', {
        name: 'Move “DISABLED Blue” to Trash — C:/Mods/Alice/DISABLED Blue',
      }),
    );
    expect(screen.getByRole('alertdialog')).toBeInTheDocument();
    act(() => useAppStore.setState({ activeGameId: 'game-2' }));

    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
  });
});
