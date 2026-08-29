import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '../../../stores/useAppStore';
import FolderConflictManager from './FolderConflictManager';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path) => path),
}));

describe('FolderConflictManager', () => {
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

  it('does not mount or fetch details until the conflict dialog is requested', () => {
    useAppStore.setState({ workspaceDialogState: { kind: 'none' } });

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('shows one explicit keep action, renames the other folders, and submits the reviewed plan', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') {
        return Promise.resolve([
          {
            path: 'C:/Mods/Alice/Blue',
            folder_name: 'Blue',
            is_enabled: true,
            total_size: 1,
            file_count: 1,
            files: [],
            thumbnail_path: null,
            partial: false,
            warnings: [],
          },
          {
            path: 'C:/Mods/Alice/DISABLED Blue',
            folder_name: 'DISABLED Blue',
            is_enabled: false,
            total_size: 2,
            file_count: 2,
            files: [],
            thumbnail_path: null,
            partial: false,
            warnings: [],
          },
          {
            path: 'C:/Mods/DISABLED Alice/Blue',
            folder_name: 'Blue',
            is_enabled: false,
            total_size: 3,
            file_count: 3,
            files: [],
            thumbnail_path: null,
            partial: false,
            warnings: [],
          },
        ]);
      }
      if (command === 'resolve_folder_name_conflict') {
        return Promise.resolve({
          game_id: 'game-1',
          reason: 'InternalMutation',
          status: 'Applied',
          folder_conflicts: [],
          error_message: null,
          changed_roots: [],
          objects_changed: true,
          folders_changed: true,
          collections_changed: false,
          runtime_file_changed: false,
          overlay_refresh_triggered: false,
          thumbnail_roots: [],
          cleared_selection_paths: [],
          path_updates: [],
          collection_reference_impact: {
            affected_collection_count: 0,
            affected_collection_names: [],
            rewritten_paths: [],
            missing_paths: [],
          },
          change_summary: {
            object_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
            mod_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
            object_sample_names: [],
            mod_sample_names: [],
            has_user_visible_changes: false,
          },
        });
      }
      return Promise.resolve();
    });

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const inputs = await screen.findAllByRole('textbox');
    expect(inputs).toHaveLength(2);
    expect(screen.getByText('Keep current name')).toBeInTheDocument();
    expect(screen.getAllByText('Rename folder')).toHaveLength(2);
    expect(screen.getByText('No filesystem changes')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Open folder' })).not.toBeInTheDocument();
    fireEvent.change(inputs[0], { target: { value: 'Blue Two' } });
    fireEvent.change(inputs[1], { target: { value: 'Blue Three' } });
    fireEvent.click(screen.getByRole('button', { name: 'Rename 2 folders & Next' }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('resolve_folder_name_conflict', {
        gameId: 'game-1',
        groupId: 'group-1',
        renames: [
          { path: 'C:/Mods/Alice/Blue', base_name: 'Blue' },
          { path: 'C:/Mods/Alice/DISABLED Blue', base_name: 'Blue Two' },
          { path: 'C:/Mods/DISABLED Alice/Blue', base_name: 'Blue Three' },
        ],
      });
    });
  });

  it('keeps the resolved total visible when the queue becomes empty', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') return Promise.resolve([]);
      return Promise.resolve();
    });

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    await screen.findAllByRole('textbox');
    act(() => useAppStore.getState().setFolderConflicts('game-1', []));

    expect(await screen.findByText('All conflicts resolved')).toBeInTheDocument();
    expect(screen.getByText('1 of 1 conflicts resolved')).toBeInTheDocument();
  });

  it('switches the keep action to the exact folder selected by the user', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') return Promise.resolve([]);
      return Promise.resolve();
    });

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const initialRenameInputs = await screen.findAllByRole('textbox');
    fireEvent.change(initialRenameInputs[0], { target: { value: 'Blue Two' } });
    fireEvent.click(
      screen.getByRole('button', {
        name: 'Keep current name for C:/Mods/Alice/DISABLED Blue',
      }),
    );

    expect(screen.getAllByRole('textbox')).toHaveLength(2);
    expect(
      screen.getByRole('button', {
        name: 'Keep current name for C:/Mods/Alice/DISABLED Blue',
      }),
    ).toHaveAttribute('aria-pressed', 'true');
    expect(
      screen.getByRole('button', { name: 'Keep current name for C:/Mods/Alice/Blue' }),
    ).toHaveAttribute('aria-pressed', 'false');

    fireEvent.click(
      screen.getByRole('button', { name: 'Keep current name for C:/Mods/Alice/Blue' }),
    );
    expect(screen.getAllByRole('textbox')[0]).toHaveValue('Blue');
  });

  it('preserves rename drafts when the same conflict report is refreshed', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') return Promise.resolve([]);
      return Promise.resolve();
    });

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const renameInputs = await screen.findAllByRole('textbox');
    fireEvent.change(renameInputs[0], { target: { value: 'Blue Alternative' } });
    const currentGroups = useAppStore.getState().folderConflictsByGame['game-1'];
    act(() => {
      useAppStore.getState().setFolderConflicts(
        'game-1',
        currentGroups.map((group) => ({
          ...group,
          candidates: group.candidates.map((candidate) => ({ ...candidate })),
        })),
      );
    });

    await waitFor(() => expect(screen.getAllByRole('textbox')[0]).toHaveValue('Blue Alternative'));
  });

  it('keeps remaining drafts and the queue open when Trash only shrinks a group', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') return Promise.resolve([]);
      return Promise.resolve();
    });

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const renameInputs = await screen.findAllByRole('textbox');
    fireEvent.change(renameInputs[1], { target: { value: 'Blue Three' } });
    const [currentGroup] = useAppStore.getState().folderConflictsByGame['game-1'];
    act(() => {
      useAppStore.getState().setFolderConflicts('game-1', [
        {
          ...currentGroup,
          candidates: currentGroup.candidates.filter(
            (candidate) => candidate.path !== 'C:/Mods/Alice/DISABLED Blue',
          ),
        },
      ]);
    });

    await waitFor(() => expect(screen.getAllByRole('textbox')).toHaveLength(1));
    expect(screen.getByRole('textbox')).toHaveValue('Blue Three');
    expect(screen.getAllByText('0 resolved · 1 remaining (1 total)')).toHaveLength(2);
  });

  it('preserves each conflict draft while navigating between queue items', async () => {
    const [blueGroup] = useAppStore.getState().folderConflictsByGame['game-1'];
    useAppStore.getState().setFolderConflicts('game-1', [
      blueGroup,
      {
        group_id: 'group-2',
        identity: 'alice/red',
        display_name: 'Red',
        candidates: blueGroup.candidates.slice(0, 2).map((candidate, index) => ({
          ...candidate,
          path: `C:/Mods/Alice/${index === 0 ? '' : 'DISABLED '}Red`,
          folder_name: index === 0 ? 'Red' : 'DISABLED Red',
          base_name: 'Red',
        })),
      },
    ]);
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'get_folder_conflict_details') return Promise.resolve([]);
      return Promise.resolve();
    });

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const blueInputs = await screen.findAllByRole('textbox');
    fireEvent.change(blueInputs[0], { target: { value: 'Blue Alternative' } });
    fireEvent.click(screen.getByRole('button', { name: /2\. Red/ }));
    await waitFor(() => expect(screen.getByRole('textbox')).toHaveValue('Red'));
    fireEvent.click(screen.getByRole('button', { name: /1\. Blue/ }));

    await waitFor(() => expect(screen.getAllByRole('textbox')[0]).toHaveValue('Blue Alternative'));
  });
});
