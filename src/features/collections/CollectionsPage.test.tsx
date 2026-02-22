import { render, screen, waitFor } from '../../test-utils';
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

describe('CollectionsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      safeMode: true,
      gridSelection: new Set(['mod-a']),
    });
  });

  it('renders collections fetched from backend', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd) => {
      if (cmd === 'list_collections') {
        return [
          {
            id: 'c-1',
            name: 'Abyss Team',
            game_id: 'g-1',
            is_safe_context: true,
            member_count: 5,
          },
        ];
      }
      return [];
    });

    render(<CollectionsPage />);

    await waitFor(() => expect(screen.getByText('Abyss Team')).toBeInTheDocument());
    expect(screen.getByRole('button', { name: /save collection/i })).toBeInTheDocument();
  });
});
