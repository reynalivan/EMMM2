import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/app/store';
import { commands } from '../../../shared/api/tauri/bindings';
import FolderConflictManager from './FolderConflictManager';

vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: {
    getFolderConflictDetails: vi.fn(),
    resolveFolderNameConflict: vi.fn(),
    trashFolderConflictCandidate: vi.fn(),
  },
}));

const getFolderConflictDetails = vi.mocked(commands.getFolderConflictDetails);
const resolveFolderNameConflict = vi.mocked(commands.resolveFolderNameConflict);

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

  it('does not mount or fetch details until the conflict dialog is requested', () => {
    useAppStore.setState({ workspaceDialogState: { kind: 'none' } });

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(getFolderConflictDetails).not.toHaveBeenCalled();
  });

  it('shows one explicit keep action, renames the other folders, and submits the reviewed plan', async () => {
    const [blueGroup] = useAppStore.getState().folderConflictsByGame['game-1'];
    useAppStore.setState({
      folderConflictsByGame: {
        'game-1': [
          blueGroup,
          {
            ...blueGroup,
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
        ],
      },
    });
    getFolderConflictDetails.mockResolvedValue([
      {
        path: 'C:/Mods/Alice/Blue',
        folder_name: 'Blue',
        is_enabled: true,
        total_size: 1,
        file_count: 1,
        created_at: null,
        modified_at: null,
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
        created_at: null,
        modified_at: null,
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
        created_at: null,
        modified_at: null,
        files: [],
        thumbnail_path: null,
        partial: false,
        warnings: [],
      },
    ]);
    resolveFolderNameConflict.mockResolvedValue({
      game_id: 'game-1',
      reconcile_revision: 2,
      reason: 'InternalMutation',
      status: 'Applied',
      folder_conflicts: [],
      rename_confirmations: [],
      error_message: null,
      changed_roots: [],
      objects_changed: true,
      folders_changed: true,
      collections_changed: false,
      runtime_file_changed: false,
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
      pending_runtime_effects: { collections_dirty: false, overlay_refresh: false },
      warnings: [],
    });

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const inputs = await screen.findAllByRole('textbox');
    expect(inputs).toHaveLength(2);
    const keepActions = screen.getAllByRole('radio', { name: 'Keep this name instead' });
    expect(keepActions).toHaveLength(3);
    expect(keepActions[0]).toBeChecked();
    expect(screen.getAllByRole('button', { name: 'Rename' })).toHaveLength(2);
    expect(screen.getByText('Keep')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Open folder' })).not.toBeInTheDocument();
    fireEvent.change(inputs[0], { target: { value: 'Blue Two' } });
    fireEvent.change(inputs[1], { target: { value: 'Blue Three' } });
    fireEvent.click(screen.getByRole('button', { name: /Rename 2 folders.*Next/ }));

    await waitFor(() => {
      expect(resolveFolderNameConflict).toHaveBeenCalledWith('game-1', 'group-1', [
        { path: 'C:/Mods/Alice/Blue', base_name: 'Blue' },
        { path: 'C:/Mods/Alice/DISABLED Blue', base_name: 'Blue Two' },
        { path: 'C:/Mods/DISABLED Alice/Blue', base_name: 'Blue Three' },
      ]);
    });
    expect(await screen.findByRole('button', { name: 'Red2' })).toBeInTheDocument();
    expect(screen.queryByText('All conflicts resolved')).not.toBeInTheDocument();
  });

  it('validates empty and duplicate rename names when the input loses focus', async () => {
    getFolderConflictDetails.mockResolvedValue([]);

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const inputs = await screen.findAllByRole('textbox');
    const applyButton = screen.getByRole('button', { name: 'Rename 2 folders' });
    fireEvent.change(inputs[0], { target: { value: '' } });
    fireEvent.blur(inputs[0]);

    const emptyNameError = await screen.findByText(
      'Enter a name without leading or trailing spaces.',
    );
    expect(emptyNameError).toBeInTheDocument();
    expect(emptyNameError).toHaveClass('block', 'w-full', 'break-words', 'whitespace-normal');
    expect(emptyNameError.closest('label')).toHaveClass('flex', 'flex-col', 'items-stretch');
    expect(inputs[0]).toHaveClass('w-full');
    expect(applyButton).toBeDisabled();

    fireEvent.change(inputs[0], { target: { value: 'Blue-03' } });
    fireEvent.blur(inputs[0]);

    expect(await screen.findAllByText('Each candidate needs a unique final name.')).toHaveLength(2);
    expect(applyButton).toBeDisabled();

    fireEvent.change(inputs[0], { target: { value: 'Blue-04' } });
    fireEvent.blur(inputs[0]);

    await waitFor(() => expect(applyButton).toBeEnabled());
  });

  it('keeps folder info above the conflict scroll area and does not change the kept folder', async () => {
    getFolderConflictDetails.mockResolvedValue([
      {
        path: 'C:/Mods/Alice/Blue',
        folder_name: 'Blue',
        is_enabled: true,
        total_size: 1,
        file_count: 1,
        created_at: null,
        modified_at: null,
        files: ['Blue.ini'],
        thumbnail_path: null,
        partial: false,
        warnings: [],
      },
    ]);

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const keepAction = (
      await screen.findAllByRole('radio', {
        name: 'Keep this name instead',
      })
    )[0];
    const infoTrigger = await screen.findByRole('button', { name: 'Folder Info' });
    expect(keepAction).toBeChecked();

    fireEvent.click(infoTrigger);

    const infoPanel = await screen.findByRole('dialog', { name: 'Folder Info' });
    expect(infoPanel).toHaveClass('fixed', 'z-[1000]');
    expect(infoPanel.parentElement).toBe(
      screen.getByRole('dialog', { name: 'Folder name conflicts' }),
    );
    expect(keepAction).toBeChecked();

    fireEvent.keyDown(infoPanel, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Folder Info' })).not.toBeInTheDocument();
  });

  it('identifies an empty report as externally resolved without crediting a dialog action', async () => {
    getFolderConflictDetails.mockResolvedValue([]);

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    await screen.findAllByRole('textbox');
    act(() => {
      useAppStore.setState({
        folderConflictsByGame: { 'game-1': [] },
        folderConflictReportsByGame: {
          'game-1': {
            revision: 2,
            groups: [],
            status: 'resolvedExternally',
            reason: 'WatcherBatch',
          },
        },
      });
    });

    expect(await screen.findByText('Resolved outside this dialog')).toBeInTheDocument();
    expect(screen.queryByText('All conflicts resolved')).not.toBeInTheDocument();
  });

  it('switches the keep action to the exact folder selected by the user', async () => {
    getFolderConflictDetails.mockResolvedValue([]);

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const initialRenameInputs = await screen.findAllByRole('textbox');
    expect(initialRenameInputs[0]).toHaveValue('Blue-02');
    expect(initialRenameInputs[1]).toHaveValue('Blue-03');
    fireEvent.change(initialRenameInputs[0], { target: { value: 'Blue Two' } });
    fireEvent.click(screen.getAllByRole('radio', { name: 'Keep this name instead' })[1]);

    expect(screen.getAllByRole('textbox')).toHaveLength(2);
    const updatedKeepActions = screen.getAllByRole('radio', { name: 'Keep this name instead' });
    expect(updatedKeepActions[1]).toBeChecked();
    expect(updatedKeepActions[0]).not.toBeChecked();

    fireEvent.click(updatedKeepActions[0]);
    expect(screen.getAllByRole('textbox')[0]).toHaveValue('Blue-02');
  });

  it('preserves rename drafts when the same conflict report is refreshed', async () => {
    getFolderConflictDetails.mockResolvedValue([]);

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
    getFolderConflictDetails.mockResolvedValue([]);

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
    expect(screen.getByText('0 resolved · 1 remaining (1 total)')).toBeInTheDocument();
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
    getFolderConflictDetails.mockResolvedValue([]);

    render(<FolderConflictManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const blueInputs = await screen.findAllByRole('textbox');
    fireEvent.change(blueInputs[0], { target: { value: 'Blue Alternative' } });
    fireEvent.click(screen.getByRole('button', { name: 'Red2' }));
    await waitFor(() => expect(screen.getByRole('textbox')).toHaveValue('Red-02'));
    fireEvent.click(screen.getByRole('button', { name: 'Blue3' }));

    await waitFor(() => expect(screen.getAllByRole('textbox')[0]).toHaveValue('Blue Alternative'));
  });
});
