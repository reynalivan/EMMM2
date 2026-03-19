import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { render, screen, waitFor } from '../../testing/test-utils';
import ModeSwitchConfirmModal from './ModeSwitchConfirmModal';
import { useAppStore } from '../../stores/useAppStore';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../collections/components/ModGroupList', () => ({
  ModGroupList: ({ groups }: { groups: Array<{ mods: Array<{ actual_name: string }> }> }) => (
    <div data-testid="mod-group-list">
      {groups.flatMap((group) => group.mods.map((mod) => mod.actual_name)).join(',')}
    </div>
  ),
}));

function buildPreview(overrides: Record<string, unknown> = {}) {
  return {
    leaving_mods: [
      {
        id: 'safe-mod',
        actual_name: 'Safe Mod',
        folder_path: 'E:/Mods/SafeObject/SafeMod',
        is_safe: true,
        object_id: 'obj-safe',
        object_name: 'Amber',
        object_type: 'Character',
        node_type: 'FlatModRoot',
      },
    ],
    leaving_object_states: [
      {
        object_id: 'obj-safe',
        name: 'Amber',
        object_type: 'Character',
        is_enabled: true,
        thumbnail_hint: null,
      },
    ],
    leaving_state_name: 'My Save',
    leaving_state_kind: 'named',
    target_mods: [
      {
        id: 'unsafe-mod',
        actual_name: 'Unsafe Mod',
        folder_path: 'E:/Mods/UnsafeObject/UnsafeMod',
        is_safe: false,
        object_id: 'obj-unsafe',
        object_name: 'Lisa',
        object_type: 'Character',
        node_type: 'FlatModRoot',
      },
    ],
    target_object_states: [
      {
        object_id: 'obj-unsafe',
        name: 'Lisa',
        object_type: 'Character',
        is_enabled: true,
        thumbnail_hint: null,
      },
    ],
    target_state_name: 'My Unsafe',
    target_state_kind: 'named',
    target_description: 'Last Active Collection',
    ...overrides,
  };
}

function buildRuntimeSnapshot(overrides: Record<string, unknown> = {}) {
  return {
    game_id: 'g-1',
    is_safe: true,
    active_collection_id: 'safe-collection',
    state_name: 'My Save',
    state_kind: 'named',
    roots: [],
    object_states: [],
    signature: 'sig-safe',
    snapshot_source: 'disk_scan',
    reconciled_count: 0,
    ...overrides,
  };
}

describe('ModeSwitchConfirmModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({ activeGameId: 'g-1', safeMode: true });
  });

  it('renders active collection labels for leaving and target corridors', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'get_corridor_runtime_snapshot') {
        return buildRuntimeSnapshot();
      }

      if (command === 'preview_corridor_switch') {
        return buildPreview();
      }

      throw new Error(`Unexpected command: ${command}`);
    });

    render(
      <ModeSwitchConfirmModal
        open={true}
        targetSafeMode={false}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('My Save')).toBeInTheDocument();
    });

    expect(screen.getByText('Current SAFE State')).toBeInTheDocument();
    expect(screen.getByText('Current Active Mods')).toBeInTheDocument();
    expect(screen.getByText('My Unsafe')).toBeInTheDocument();
    expect(screen.getByText('Destination UNSAFE State')).toBeInTheDocument();
    expect(screen.getByText('Last Active Collection')).toBeInTheDocument();
    expect(screen.queryByText('No saved target state. All mods will remain disabled.')).toBeNull();
  });

  it('renders unsaved preset name for remembered empty target snapshots', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'get_corridor_runtime_snapshot') {
        return buildRuntimeSnapshot();
      }

      if (command === 'preview_corridor_switch') {
        return buildPreview({
          target_mods: [],
          target_object_states: [],
          target_state_name: 'Unsaved Preset',
          target_state_kind: 'unsaved',
        });
      }

      throw new Error(`Unexpected command: ${command}`);
    });

    render(
      <ModeSwitchConfirmModal
        open={true}
        targetSafeMode={false}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('Unsaved Preset')).toBeInTheDocument();
    });

    expect(screen.getByText('Unsaved Preset is empty (All Disabled).')).toBeInTheDocument();
  });

  it('renders all-disabled messaging only when target corridor has no remembered state', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'get_corridor_runtime_snapshot') {
        return buildRuntimeSnapshot();
      }

      if (command === 'preview_corridor_switch') {
        return buildPreview({
          target_mods: [],
          target_object_states: [],
          target_state_name: null,
          target_state_kind: 'none',
        });
      }

      throw new Error(`Unexpected command: ${command}`);
    });

    render(
      <ModeSwitchConfirmModal
        open={true}
        targetSafeMode={true}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('All Disabled')).toBeInTheDocument();
    });

    expect(screen.getByText('Destination SAFE State')).toBeInTheDocument();
    expect(screen.getByText('No remembered active collection')).toBeInTheDocument();
    expect(
      screen.getByText('No saved target state. All mods will remain disabled.'),
    ).toBeInTheDocument();
  });

  it('refetches switch preview when the current strict runtime state changes', async () => {
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'get_corridor_runtime_snapshot') {
        return buildRuntimeSnapshot({ active_collection_id: 'safe-collection-2', state_name: 'My New Collection', signature: 'sig-new' });
      }

      if (command === 'preview_corridor_switch') {
        return buildPreview({ leaving_state_name: 'My New Collection' });
      }

      throw new Error(`Unexpected command: ${command}`);
    });

    render(
      <ModeSwitchConfirmModal
        open={true}
        targetSafeMode={false}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('My New Collection')).toBeInTheDocument();
    });

    expect(vi.mocked(invoke)).toHaveBeenCalledWith('get_corridor_runtime_snapshot', {
      gameId: 'g-1',
      isSafe: true,
    });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith('preview_corridor_switch', {
      targetEnabled: false,
    });
  });
});
