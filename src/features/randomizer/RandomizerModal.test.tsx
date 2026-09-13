import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '../../tests/testing/test-utils';
import RandomizerModal from './RandomizerModal';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => path,
  invoke: vi.fn(),
}));

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) => {
      const messages: Record<string, string> = {
        'randomizer.title': 'Randomizer',
        'randomizer.desc': 'Pick random mods',
        'randomizer.consulting': 'Consulting the RNG Gods...',
        'randomizer.deselect_all': 'Deselect All',
        'randomizer.select_all': 'Select All',
        'randomizer.reroll': 'Reroll All',
        'randomizer.roll': 'Roll Luck',
        'randomizer.no_eligible': 'No eligible character mods found',
        'randomizer.empty_desc': 'No results yet',
        'randomizer.scope_title': 'Roll scope',
        'randomizer.scope_character': 'Character',
        'randomizer.scope_weapon': 'Weapon',
        'randomizer.scope_ui': 'UI',
        'randomizer.scope_other': 'Other',
        'randomizer.scope_unclassified': 'Unclassified',
        'randomizer.scope_required': 'Select a scope',
        'randomizer.backup_enabled': 'Backup selected mods to new Collection',
        'randomizer.backup_unavailable': 'There are no active mods to back up.',
        'randomizer.backup_name_placeholder': 'Collection name',
        'randomizer.backup_created': `Created ${String(vars?.name)}`,
        'randomizer.backup_reused': `Reused ${String(vars?.name)}`,
        'randomizer.review_title': 'Changes to apply',
        'randomizer.reviewing': 'Reviewing...',
        'common:actions.close': 'Close',
      };
      if (key === 'randomizer.selection_status') {
        return `${String(vars?.selected)} of ${String(vars?.total)} selected`;
      }
      if (key === 'randomizer.apply') return `Apply (${String(vars?.count)})`;
      if (key === 'randomizer.review_changes') return `Review changes (${String(vars?.count)})`;
      if (key === 'randomizer.confirm_apply') return `Confirm & Apply (${String(vars?.count)})`;
      if (key === 'randomizer.applying') return 'Applying...';
      return messages[key] ?? key;
    },
  }),
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: { success: vi.fn() },
}));

import { invoke } from '@tauri-apps/api/core';
import { toast } from '@/shared/ui/toast';

const proposals = [
  {
    object_id: 'obj-1',
    object_name: 'Hu Tao',
    object_type: 'Character',
    mode: 'exclusive',
    is_safe: true,
    active_mod_names: [],
    mod_id: 'mod-a',
    name: 'Hu Tao Galaxy Skin',
    thumbnail_path: null,
    folder_path: 'E:/Mods/Hu Tao Galaxy',
  },
  {
    object_id: 'obj-2',
    object_name: 'Kazuha',
    object_type: 'Character',
    mode: 'exclusive',
    is_safe: true,
    active_mod_names: [],
    mod_id: 'mod-b',
    name: 'Kazuha Samurai Skin',
    thumbnail_path: null,
    folder_path: 'E:/Mods/Kazuha Samurai',
  },
];

const activeRuntime = {
  is_dirty: true,
  current_mods: [{ folder_path: 'E:/Mods/Old active mod' }],
};
const emptyRuntime = { is_dirty: false, current_mods: [] };

function previewResponse() {
  return {
    fingerprint: 'preview-1',
    enable_count: 2,
    disable_count: 0,
    unsafe_mod_names: [],
    runtime_conflicts: [],
    items: proposals.map((proposal) => ({
      object_id: proposal.object_id,
      object_name: proposal.object_name,
      object_type: proposal.object_type,
      mode: proposal.mode,
      selected_mod_name: proposal.name,
      selected_mod_id: proposal.mod_id,
      active_mod_names: proposal.active_mod_names,
      disable_count: 0,
    })),
  };
}

function mockCommands(runtime = activeRuntime) {
  vi.mocked(invoke).mockImplementation((command) => {
    if (command === 'suggest_random_mods') return Promise.resolve(proposals);
    if (command === 'get_collection_runtime_state') return Promise.resolve(runtime);
    if (command === 'preview_randomized_loadout') return Promise.resolve(previewResponse());
    if (command === 'apply_randomized_loadout') {
      return Promise.resolve({
        impact: { rewrites: [], cleared_selection_paths: [], refresh_scopes: [] },
        backup: { collection_id: 'backup-1', collection_name: 'Backup', reused: false },
        sync_warning: null,
      });
    }
    return Promise.resolve(null);
  });
}

async function reviewAndApply() {
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: /review changes/i }));
  });
  await screen.findByText('Changes to apply');
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: /confirm.*apply/i }));
  });
}

async function rollRandomizer() {
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: /roll luck/i }));
  });
}

describe('RandomizerModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    HTMLDialogElement.prototype.showModal = vi.fn(function (this: HTMLDialogElement) {
      this.open = true;
    });
    HTMLDialogElement.prototype.close = vi.fn();
    mockCommands();
  });

  it('waits for Roll and sends the selected scope with safety and history', async () => {
    render(<RandomizerModal open onClose={vi.fn()} gameId="g-1" />);

    expect(
      vi.mocked(invoke).mock.calls.some(([command]) => command === 'suggest_random_mods'),
    ).toBe(false);
    expect(screen.getByRole('button', { name: /roll luck/i })).toHaveClass('btn-primary');
    expect(screen.queryByRole('button', { name: /review changes/i })).not.toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'Character' })).toBeChecked();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Weapon' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Unclassified' }));

    await rollRandomizer();

    expect(invoke).toHaveBeenCalledWith('suggest_random_mods', {
      input: {
        game_id: 'g-1',
        safety_filter: 'all',
        scope: { categories: ['Character', 'Weapon'], include_unclassified: true },
        recent_mod_ids_by_object: {},
        excluded_object_ids: [],
      },
    });

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /reroll all/i }));
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('suggest_random_mods', {
        input: expect.objectContaining({
          scope: { categories: ['Character', 'Weapon'], include_unclassified: true },
          recent_mod_ids_by_object: { 'obj-1': ['mod-a'], 'obj-2': ['mod-b'] },
          excluded_object_ids: [],
        }),
      });
    });
  });

  it('enables backup by default and pre-fills an editable name', async () => {
    render(<RandomizerModal open onClose={vi.fn()} gameId="g-1" />);

    await rollRandomizer();
    await waitFor(() => expect(screen.getByText('Hu Tao Galaxy Skin')).toBeInTheDocument());
    expect(screen.getByRole('checkbox', { name: /backup selected mods/i })).toBeChecked();
    expect(
      screen.getByDisplayValue(/^Backup before shuffle \d{4}-\d{2}-\d{2} \d{2}:\d{2}$/),
    ).toBeEnabled();
  });

  it('hides backup when the current runtime has already been saved', async () => {
    mockCommands(emptyRuntime);
    render(<RandomizerModal open onClose={vi.fn()} gameId="g-1" />);

    await rollRandomizer();
    await screen.findByText('Hu Tao Galaxy Skin');
    expect(
      screen.queryByRole('checkbox', { name: /backup selected mods/i }),
    ).not.toBeInTheDocument();
    expect(screen.queryByPlaceholderText('Collection name')).not.toBeInTheDocument();
  });

  it('applies without backup when there are no active mods', async () => {
    mockCommands(emptyRuntime);
    render(<RandomizerModal open onClose={vi.fn()} gameId="g-1" />);
    await rollRandomizer();
    await screen.findByText('Hu Tao Galaxy Skin');

    await reviewAndApply();

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('apply_randomized_loadout', {
        input: expect.objectContaining({ backup: null }),
      });
    });
  });

  it('validates a required backup name before Apply', async () => {
    render(<RandomizerModal open onClose={vi.fn()} gameId="g-1" />);
    await rollRandomizer();
    const name = await screen.findByDisplayValue(/^Backup before shuffle/);
    fireEvent.change(name, { target: { value: '   ' } });

    expect(screen.getByRole('button', { name: /review changes/i })).toBeDisabled();
  });

  it('applies all selected mods as one batch and reports a created backup', async () => {
    const onClose = vi.fn();
    render(<RandomizerModal open onClose={onClose} gameId="g-1" />);
    await rollRandomizer();
    await screen.findByText('Hu Tao Galaxy Skin');
    const name = await screen.findByDisplayValue(/^Backup before shuffle/);
    fireEvent.change(name, { target: { value: 'Before gacha' } });

    await reviewAndApply();

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('apply_randomized_loadout', {
        input: {
          game_id: 'g-1',
          mod_ids: ['mod-a', 'mod-b'],
          safety_filter: 'all',
          scope: { categories: ['Character'], include_unclassified: false },
          preview_fingerprint: 'preview-1',
          backup: { collection_name: 'Before gacha' },
        },
      });
      expect(toast.success).toHaveBeenCalledWith('Created Backup');
      expect(onClose).toHaveBeenCalledOnce();
    });
  });

  it('reports when an identical backup Collection was reused', async () => {
    vi.mocked(invoke).mockImplementation((command) => {
      if (command === 'suggest_random_mods') return Promise.resolve(proposals);
      if (command === 'get_collection_runtime_state') return Promise.resolve(activeRuntime);
      if (command === 'preview_randomized_loadout') return Promise.resolve(previewResponse());
      if (command === 'apply_randomized_loadout') {
        return Promise.resolve({
          impact: { rewrites: [], cleared_selection_paths: [], refresh_scopes: [] },
          backup: { collection_id: 'backup-1', collection_name: 'Existing backup', reused: true },
          sync_warning: null,
        });
      }
      return Promise.resolve(null);
    });
    render(<RandomizerModal open onClose={vi.fn()} gameId="g-1" />);
    await rollRandomizer();
    await screen.findByText('Hu Tao Galaxy Skin');

    await reviewAndApply();

    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith('Reused Existing backup');
    });
  });

  it('keeps the modal open if the atomic batch fails', async () => {
    vi.mocked(invoke).mockImplementation((command) => {
      if (command === 'suggest_random_mods') return Promise.resolve(proposals);
      if (command === 'get_collection_runtime_state') return Promise.resolve(activeRuntime);
      if (command === 'preview_randomized_loadout') return Promise.resolve(previewResponse());
      if (command === 'apply_randomized_loadout') return Promise.reject(new Error('Rename failed'));
      return Promise.resolve(null);
    });
    const onClose = vi.fn();
    render(<RandomizerModal open onClose={onClose} gameId="g-1" />);
    await rollRandomizer();
    await screen.findByText('Hu Tao Galaxy Skin');

    await reviewAndApply();

    await waitFor(() => expect(screen.getByText('Rename failed')).toBeInTheDocument());
    expect(onClose).not.toHaveBeenCalled();
  });

  it('clears a rolled recommendation when the scope changes', async () => {
    render(<RandomizerModal open onClose={vi.fn()} gameId="g-1" />);
    await rollRandomizer();
    await screen.findByText('Hu Tao Galaxy Skin');

    fireEvent.click(screen.getByRole('checkbox', { name: 'Weapon' }));

    await waitFor(() => {
      expect(screen.queryByText('Hu Tao Galaxy Skin')).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /review changes/i })).not.toBeInTheDocument();
    });
  });

  it('requires a non-empty scope before Roll', () => {
    render(<RandomizerModal open onClose={vi.fn()} gameId="g-1" />);

    fireEvent.click(screen.getByRole('checkbox', { name: 'Character' }));

    expect(screen.getByText('Select a scope')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /roll luck/i })).toBeDisabled();
  });
});
