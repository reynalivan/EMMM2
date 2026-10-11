import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  AppSettings,
  DiskReconcileResult,
  GameModsDirectoryInspection,
} from '@/shared/api/tauri/bindings';
import { commands } from '@/shared/api/tauri/bindings';
import { GameType, type GameConfig } from '@/entities/game';
import GamesTab from './GamesTab';

const fixture = vi.hoisted(() => ({
  settings: null as AppSettings | null,
  saveSettings: vi.fn<(settings: AppSettings) => Promise<AppSettings>>(),
  state: {
    activeGameId: null,
    setActiveGameId: vi.fn(),
    setWorkspaceView: vi.fn(),
  },
}));

vi.mock('@/entities/settings', () => ({
  useSettings: () => ({ settings: fixture.settings, saveSettingsAsync: fixture.saveSettings }),
}));
vi.mock('@/app/store', () => ({
  useAppStore: <T,>(selector: (state: typeof fixture.state) => T) => selector(fixture.state),
}));
vi.mock('@/features/file-watcher', () => ({ applyDiskReconcileResult: vi.fn() }));
vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: {
    inspectGameModsDirectory: vi.fn(),
    applyGameModsDirectory: vi.fn(),
    getSettings: vi.fn(),
  },
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({ t: (key: string) => key }),
}));

const gameA: GameConfig = {
  id: 'game-a',
  name: 'Game A',
  game_type: GameType.GIMI,
  mod_path: 'E:/GameA/Mods',
  game_exe: 'E:/GameA/Game.exe',
  loader_exe: null,
  launch_args: null,
};
const gameB: GameConfig = {
  ...gameA,
  id: 'game-b',
  name: 'Game B',
  mod_path: 'E:/GameB/Mods',
};
const settings: AppSettings = {
  revision: 1,
  theme: 'onyx',
  language: 'en',
  games: [gameA, gameB],
  active_game_id: null,
  safety: { keywords: [] },
  ai: { enabled: false, base_url: null, has_api_key: false },
  auto_close_launcher: false,
};
const replacementPath = 'E:/Replacement/Mods';

function inspection(classification: 'Empty' | 'Matching'): GameModsDirectoryInspection {
  return {
    game_id: gameA.id,
    candidate_path: replacementPath,
    fingerprint: 'replacement-v1',
    summary: {
      classification,
      existing_object_count: 1,
      existing_mod_count: 1,
      candidate_object_count: 0,
      candidate_mod_count: 0,
      physical_entry_count: 0,
      filesystem_identity_match_count: 0,
      relative_path_match_count: 0,
      requires_confirmation: classification === 'Empty',
    },
  };
}

const reconcile: DiskReconcileResult = {
  game_id: gameA.id,
  reconcile_revision: 1,
  reason: 'ManualRepair',
  status: 'Applied',
  scan_scope: 'Full',
  folder_conflicts: [],
  rename_confirmations: [],
  error_message: null,
  changed_roots: [],
  objects_changed: false,
  folders_changed: false,
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
};

describe('GamesTab editor session ownership', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fixture.settings = settings;
    fixture.saveSettings.mockImplementation(async (saved) => saved);
    vi.mocked(commands.applyGameModsDirectory).mockResolvedValue({
      game: { ...gameA, mod_path: replacementPath },
      inspection: inspection('Matching'),
      reconcile,
      emmm_data_moved: false,
      watcher_warning: null,
    });
    vi.mocked(commands.getSettings).mockResolvedValue({
      ...settings,
      games: [{ ...gameA, mod_path: replacementPath }, gameB],
    });
  });

  it.each(['Empty', 'Matching'] as const)(
    'keeps Game B draft isolated from the previous %s source inspection',
    async (classification) => {
      let finishInspection!: (value: GameModsDirectoryInspection) => void;
      vi.mocked(commands.inspectGameModsDirectory).mockReturnValueOnce(
        new Promise((resolve) => {
          finishInspection = resolve;
        }),
      );
      render(<GamesTab />);
      fireEvent.click(screen.getAllByTitle('settings:games.actions.edit')[0]);
      fireEvent.change(screen.getByPlaceholderText('games.form.path_placeholder'), {
        target: { value: replacementPath },
      });
      await waitFor(() => expect(screen.getByTestId('game-form-submit')).toBeEnabled());
      fireEvent.click(screen.getByTestId('game-form-submit'));
      await waitFor(() =>
        expect(commands.inspectGameModsDirectory).toHaveBeenCalledWith(gameA.id, replacementPath),
      );

      fireEvent.click(screen.getAllByTitle('settings:games.actions.edit')[1]);
      fireEvent.change(screen.getByPlaceholderText('games.form.name_placeholder'), {
        target: { value: 'Game B draft' },
      });
      await act(async () => finishInspection(inspection(classification)));

      expect(screen.getByPlaceholderText('games.form.name_placeholder')).toHaveValue(
        'Game B draft',
      );
      expect(
        screen.queryByRole('button', { name: 'grid:banners.source_confirm_empty_btn' }),
      ).toBeNull();
      expect(screen.queryByRole('status')).toBeNull();
      if (classification === 'Matching') {
        expect(commands.applyGameModsDirectory).toHaveBeenCalledExactlyOnceWith({
          game_id: gameA.id,
          candidate_path: replacementPath,
          expected_fingerprint: 'replacement-v1',
          confirm_empty: false,
          different_confirmation_game_name: null,
        });
        expect(fixture.saveSettings).toHaveBeenCalledWith(
          expect.objectContaining({
            games: [expect.objectContaining({ id: gameA.id, mod_path: replacementPath }), gameB],
          }),
        );
      } else {
        expect(commands.applyGameModsDirectory).not.toHaveBeenCalled();
      }
    },
  );
});
