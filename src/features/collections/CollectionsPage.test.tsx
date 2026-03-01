import { render, screen, waitFor, fireEvent, act } from '../../testing/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import CollectionsPage from './CollectionsPage';
import { useAppStore } from '../../stores/useAppStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: {
      id: 'g-1',
      name: 'Genshin',
      game_type: 'GIMI',
      mod_path: 'E:/Mods',
      game_exe: 'E:/Game/Genshin.exe',
      loader_exe: null,
      launch_args: null,
    },
  }),
}));

describe('CollectionsPage - TC-31', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      safeMode: true,
      gridSelection: new Set(['mod-a']),
    });

    vi.mocked(invoke).mockImplementation(async (cmd) => {
      if (cmd === 'list_collections') {
        return [
          {
            id: 'c-1',
            name: 'Abyss Team',
            game_id: 'g-1',
            is_safe_context: true,
            member_count: 5,
            is_last_unsaved: false,
          },
        ];
      }
      if (cmd === 'create_collection') {
        return {
          collection: {
            id: 'c-new',
            name: 'New Col',
            game_id: 'g-1',
            is_safe_context: true,
            member_count: 0,
          },
          mod_ids: [],
        };
      }
      if (cmd === 'apply_collection') {
        return { changed_count: 5, warnings: [] };
      }
      return [];
    });
  });

  it('TC-31-001 / TC-31-003: Save Current State blocks empty string and submits correctly', async () => {
    render(<CollectionsPage />);

    const input = screen.getByPlaceholderText('e.g. Abyss Run 1');
    const saveBtn = screen.getByRole('button', { name: /save collection/i });

    // TC-31-003: Button should be disabled if empty
    expect(saveBtn).toBeDisabled();

    // Type a name
    fireEvent.change(input, { target: { value: 'My New Loadout' } });
    expect(saveBtn).not.toBeDisabled();

    // TC-31-001: Click save
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_collection', {
        input: {
          name: 'My New Loadout',
          game_id: 'g-1',
          is_safe_context: true,
          auto_snapshot: true,
          mod_ids: [],
        },
      });
    });
  });

  it('TC-31-005: Apply Atomic Execution triggers confirmation modal', async () => {
    render(<CollectionsPage />);

    // Wait for the list to load
    await waitFor(() => expect(screen.getByText('Abyss Team')).toBeInTheDocument());

    const initialApplyBtns = screen.getAllByRole('button', { name: /apply/i });
    fireEvent.click(initialApplyBtns[0]);

    // Modal appears
    await waitFor(() => expect(screen.getByText(/Confirm Apply/i)).toBeInTheDocument());

    // Get the apply button in the modal by test id
    const modalApplyBtn = screen.getByTestId('modal-apply-btn');

    await act(async () => {
      fireEvent.click(modalApplyBtn);
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('apply_collection', {
        collectionId: 'c-1',
        gameId: 'g-1',
      });
    });
  });
});
