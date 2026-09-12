/* eslint-disable @typescript-eslint/no-explicit-any */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import GamesTab from './GamesTab';
import { useSettings } from '@/entities/settings';
import { useAppStore } from '@/app/store';
import { commands } from '../../../../shared/api/tauri/bindings';

vi.mock('@/entities/settings', () => ({
  useSettings: vi.fn(),
}));

vi.mock('@/app/store', () => ({
  useAppStore: vi.fn(),
}));

vi.mock('../../../../shared/api/tauri/bindings', () => ({
  commands: {
    inspectGameModsDirectory: vi.fn(),
    applyGameModsDirectory: vi.fn(),
    getSettings: vi.fn(),
  },
}));

vi.mock('@/features/file-watcher/hooks/useFileWatcher', () => ({
  applyDiskReconcileResult: vi.fn(),
}));

// Mock the GameFormModal so we don't need to mount it fully for simple tests
vi.mock('../../modals/GameFormModal', () => ({
  default: ({ isOpen, onClose, onSave, initialData }: any) => {
    if (!isOpen) return null;
    return (
      <div data-testid="game-form-modal">
        <button data-testid="modal-close" onClick={onClose}>
          Close
        </button>
        <button
          data-testid="modal-save"
          onClick={() =>
            onSave(
              initialData
                ? { ...initialData, name: 'Renamed Game', mod_path: 'D:/Replacement/Mods' }
                : {
                    id: 'new-id',
                    name: 'New Game',
                    game_type: 'GIMI',
                    mod_path: 'C:/Mods',
                    game_exe: 'C:/Game/GenshinImpact.exe',
                    loader_exe: 'C:/Game/3dmigotoloader.exe',
                  },
            )
          }
        >
          Save
        </button>
      </div>
    );
  },
}));

describe('GamesTab (TC-02)', () => {
  const mockSaveSettings = vi.fn();
  const mockSetActiveGameId = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mocks
    (useSettings as any).mockReturnValue({
      settings: { games: [] },
      saveSettingsAsync: mockSaveSettings,
    });

    (useAppStore as any).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector({
          activeGameId: null,
          setActiveGameId: mockSetActiveGameId,
          setWorkspaceView: vi.fn(),
        }),
    );
    Object.assign(useAppStore, {
      getState: vi.fn(() => ({
        activeGameId: null,
        setActiveGameId: mockSetActiveGameId,
        setWorkspaceView: vi.fn(),
      })),
    });

    window.confirm = vi.fn(() => true);
    vi.mocked(commands.inspectGameModsDirectory).mockResolvedValue({
      game_id: 'g1',
      candidate_path: 'D:/Replacement/Mods',
      fingerprint: 'matching-v1',
      summary: {
        classification: 'Matching',
        existing_object_count: 1,
        existing_mod_count: 1,
        candidate_object_count: 1,
        candidate_mod_count: 1,
        physical_entry_count: 1,
        filesystem_identity_match_count: 2,
        relative_path_match_count: 2,
        requires_confirmation: false,
      },
    });
    vi.mocked(commands.applyGameModsDirectory).mockResolvedValue({
      game: {
        id: 'g1',
        name: 'Genshin Impact',
        game_type: 0,
        mod_path: 'D:/Replacement/Mods',
        game_exe: 'C:/Game/Genshin.exe',
        loader_exe: null,
        launch_args: null,
      },
      inspection: {} as never,
      reconcile: {} as never,
    });
    vi.mocked(commands.getSettings).mockResolvedValue({
      revision: 4,
      theme: 'dark',
      language: 'en',
      games: [
        {
          id: 'g1',
          name: 'Genshin Impact',
          game_type: 0,
          mod_path: 'D:/Replacement/Mods',
          game_exe: 'C:/Game/Genshin.exe',
          loader_exe: null,
          launch_args: null,
        },
      ],
      active_game_id: null,
      safety: {
        keywords: [],
      },
      ai: { enabled: false, has_api_key: false, base_url: null },
      auto_close_launcher: false,
    });
  });

  it('renders empty state correctly', () => {
    render(<GamesTab />);
    expect(
      screen.getByText('No games configured. Click "Add Game" to get started.'),
    ).toBeInTheDocument();
  });

  it('renders a list of games', () => {
    (useSettings as any).mockReturnValue({
      settings: {
        games: [
          {
            id: 'g1',
            name: 'Genshin Impact',
            game_type: 'GIMI',
            mod_path: 'C:/Mods',
            game_exe: 'C:/Game/Genshin.exe',
          },
        ],
      },
      saveSettingsAsync: mockSaveSettings,
    });
    (useAppStore as any).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector({
          activeGameId: 'g1',
          setActiveGameId: mockSetActiveGameId,
        }),
    );

    render(<GamesTab />);
    expect(screen.getByText('Genshin Impact')).toBeInTheDocument();
    expect(screen.getByText('ACTIVE')).toBeInTheDocument();
    expect(screen.getByText('C:/Mods')).toBeInTheDocument();
    expect(screen.queryByTitle('Repair Index')).not.toBeInTheDocument();
  });

  it('triggers Add Game flow', () => {
    render(<GamesTab />);

    // Confirm modal is initially closed
    expect(screen.queryByTestId('game-form-modal')).not.toBeInTheDocument();

    // Click add game
    fireEvent.click(screen.getByRole('button', { name: 'Add Game' }));

    // Modal should render
    expect(screen.getByTestId('game-form-modal')).toBeInTheDocument();

    // Simulate saving a new game
    fireEvent.click(screen.getByTestId('modal-save'));

    expect(mockSaveSettings).toHaveBeenCalledWith({
      games: [
        {
          id: 'new-id',
          name: 'New Game',
          game_type: 'GIMI',
          mod_path: 'C:/Mods',
          game_exe: 'C:/Game/GenshinImpact.exe',
          loader_exe: 'C:/Game/3dmigotoloader.exe',
        },
      ],
    });
  });

  it('handles game deletion and unsets activeGameId', async () => {
    (useSettings as any).mockReturnValue({
      settings: {
        games: [
          { id: 'g1', name: 'Genshin Impact' },
          { id: 'g2', name: 'Honkai Star Rail' },
        ],
      },
      saveSettingsAsync: mockSaveSettings,
    });
    (useAppStore as any).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector({
          activeGameId: 'g1',
          setActiveGameId: mockSetActiveGameId,
        }),
    );

    render(<GamesTab />);

    // We expect two trash buttons, one for each game. Click the first one (g1)
    const deleteButtons = screen.getAllByTitle('Remove Game');
    fireEvent.click(deleteButtons[0]);

    // confirm is mocked to true
    expect(window.confirm).toHaveBeenCalled();
    expect(mockSaveSettings).toHaveBeenCalledWith({
      games: [{ id: 'g2', name: 'Honkai Star Rail' }], // g1 is removed
    });
    await waitFor(() => expect(mockSetActiveGameId).toHaveBeenCalledWith(null)); // Because g1 was active
  });

  it('routes an existing game path change through source recovery', async () => {
    (useSettings as any).mockReturnValue({
      settings: {
        revision: 3,
        games: [
          {
            id: 'g1',
            name: 'Genshin Impact',
            game_type: 'GIMI',
            mod_path: 'C:/Mods',
            game_exe: 'C:/Game/Genshin.exe',
            loader_exe: null,
            launch_args: null,
          },
        ],
      },
      saveSettingsAsync: mockSaveSettings,
    });

    render(<GamesTab />);
    fireEvent.click(screen.getByTitle('Edit Game'));
    fireEvent.click(screen.getByTestId('modal-save'));

    await waitFor(() =>
      expect(commands.inspectGameModsDirectory).toHaveBeenCalledWith('g1', 'D:/Replacement/Mods'),
    );
    expect(commands.applyGameModsDirectory).toHaveBeenCalledWith(
      expect.objectContaining({
        game_id: 'g1',
        candidate_path: 'D:/Replacement/Mods',
        expected_fingerprint: 'matching-v1',
      }),
    );
    expect(mockSaveSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        revision: 4,
        games: [expect.objectContaining({ name: 'Renamed Game', mod_path: 'D:/Replacement/Mods' })],
      }),
    );
  });

  it('keeps source confirmation open and reports an apply failure', async () => {
    (useSettings as any).mockReturnValue({
      settings: {
        revision: 3,
        games: [
          {
            id: 'g1',
            name: 'Genshin Impact',
            game_type: 'GIMI',
            mod_path: 'C:/Mods',
            game_exe: 'C:/Game/Genshin.exe',
          },
        ],
      },
      saveSettingsAsync: mockSaveSettings,
    });
    vi.mocked(commands.inspectGameModsDirectory).mockResolvedValueOnce({
      game_id: 'g1',
      candidate_path: 'D:/Replacement/Mods',
      fingerprint: 'empty-v1',
      summary: {
        classification: 'Empty',
        existing_object_count: 1,
        existing_mod_count: 1,
        candidate_object_count: 0,
        candidate_mod_count: 0,
        physical_entry_count: 0,
        filesystem_identity_match_count: 0,
        relative_path_match_count: 0,
        requires_confirmation: true,
      },
    });
    vi.mocked(commands.applyGameModsDirectory).mockRejectedValueOnce(
      new Error('source apply failed'),
    );

    render(<GamesTab />);
    fireEvent.click(screen.getByTitle('Edit Game'));
    fireEvent.click(screen.getByTestId('modal-save'));
    const confirm = await screen.findByRole('button', { name: 'Use empty folder' });
    fireEvent.click(confirm);

    expect(await screen.findByText(/source apply failed/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Use empty folder' })).toBeInTheDocument();
  });

  it('handles Set Active game', () => {
    (useSettings as any).mockReturnValue({
      settings: {
        games: [
          { id: 'g1', name: 'Genshin Impact' },
          { id: 'g2', name: 'Honkai Star Rail' },
        ],
      },
      saveSettingsAsync: mockSaveSettings,
    });
    (useAppStore as any).mockImplementation(
      (selector: (state: Record<string, unknown>) => unknown) =>
        selector({
          activeGameId: 'g1',
          setActiveGameId: mockSetActiveGameId,
        }),
    );

    render(<GamesTab />);

    // Play buttons (Set as Active). First one is disabled (already active).
    const activeButtons = screen.getAllByTitle('Set as Active');
    expect(activeButtons[0]).toBeDisabled();

    // Click second game to make it active
    fireEvent.click(activeButtons[1]);
    expect(mockSetActiveGameId).toHaveBeenCalledWith('g2');
  });
});
