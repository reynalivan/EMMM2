import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ModeSwitchConfirmModal from './ModeSwitchConfirmModal';
import { useAppStore } from '../../stores/useAppStore';

const mockUseCorridor = vi.fn();
const mockUseQuery = vi.fn();

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown> | string) => {
      const labels: Record<string, string> = {
        'layout:context.unsaved_safe': 'Unsaved SAFE Preset',
        'layout:context.unsaved_unsafe': 'Unsaved UNSAFE Preset',
        'safe_mode:switch.title_safe': 'Switch to SAFE Mode',
        'safe_mode:switch.title_unsafe': 'Switch to UNSAFE Mode',
        'safe_mode:switch.desc':
          'Review the changes to your active loadout before switching corridors.',
        'safe_mode:switch.loading': 'Loading preview...',
        'safe_mode:switch.snapshot': 'Snapshot',
        'safe_mode:switch.restore': 'Restore',
        'safe_mode:switch.active_mods': 'Current Active Mods',
        'safe_mode:switch.all_disabled': 'All Disabled',
        'safe_mode:labels.safe': 'SAFE',
        'safe_mode:labels.unsafe': 'UNSAFE',
        'safe_mode:switch.target_desc.active': 'Last Active Collection',
        'common:actions.cancel': 'Cancel',
        'common:actions.confirm': 'Confirm',
      };

      if (key === 'safe_mode:switch.leaving') {
        return `Current ${String((options as Record<string, unknown>)?.mode ?? '')} State`;
      }

      if (key === 'safe_mode:switch.target') {
        return `Destination ${String((options as Record<string, unknown>)?.mode ?? '')} State`;
      }

      if (key === 'safe_mode:switch.empty') {
        return `${String((options as Record<string, unknown>)?.name ?? '')} is empty (All Disabled).`;
      }

      if (key === 'safe_mode:switch.missing_target') {
        return 'No saved target state. All mods will remain disabled.';
      }

      if (key === 'safe_mode:switch.system_fallback_title') {
        return 'System Restore';
      }

      if (key === 'safe_mode:switch.target_desc.system_fallback') {
        return 'Restoring mods remembered by corridor system state.';
      }

      if (key === 'safe_mode:switch.target_desc.none') {
        return 'No remembered active collection';
      }

      if (typeof options === 'string') {
        return options;
      }

      return labels[key] ?? key;
    },
  }),
}));

vi.mock('../collections/hooks/useCorridor', () => ({
  useCorridor: (...args: unknown[]) => mockUseCorridor(...args),
}));

vi.mock('@tanstack/react-query', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-query')>('@tanstack/react-query');

  return {
    ...actual,
    useQuery: (...args: unknown[]) => mockUseQuery(...args),
  };
});

vi.mock('../collections/components/CollectionTreeView', () => ({
  CollectionTreeView: ({ emptyMessage }: { emptyMessage?: string }) => (
    <div>{emptyMessage ?? 'tree'}</div>
  ),
}));

function renderModal() {
  return render(
    <ModeSwitchConfirmModal
      open={true}
      targetSafeMode={false}
      onClose={vi.fn()}
      onConfirm={vi.fn()}
    />,
  );
}

describe('ModeSwitchConfirmModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    useAppStore.setState({
      activeGameId: 'game-1',
      safeMode: true,
    });

    mockUseCorridor.mockReturnValue({
      status: 'success',
      data: {
        game_id: 'game-1',
        is_safe: true,
        active_collection_id: 'unsaved-safe',
        active_collection_name: '202603251500',
        active_collection_is_unsaved: true,
        undo_collection_id: null,
        current_signature: 'sig-1',
        is_dirty: false,
      },
    });

    mockUseQuery.mockReturnValue({
      data: {
        leaving_state_name: '202603251500',
        leaving_state_is_unsaved: true,
        leaving_state_is_safe: true,
        leaving_mods: [],
        leaving_objects: [],
        leaving_tree_nodes: [],
        target_state_name: '202603251501',
        target_state_is_unsaved: true,
        target_state_is_safe: false,
        target_state_kind: 'unsaved',
        target_mods: [],
        target_objects: [],
        target_tree_nodes: [],
      },
      isLoading: false,
    });
  });

  it('renders safe and unsafe unsaved labels instead of raw snapshot names', () => {
    renderModal();

    expect(screen.getByText('Unsaved SAFE Preset')).toBeInTheDocument();
    expect(screen.getByText('Unsaved UNSAFE Preset')).toBeInTheDocument();

    expect(screen.queryByText('202603251500')).not.toBeInTheDocument();
    expect(screen.queryByText('202603251501')).not.toBeInTheDocument();
  });

  it('renders explicit all-disabled label for an empty target corridor', () => {
    mockUseQuery.mockReturnValue({
      data: {
        leaving_state_name: '202603251500',
        leaving_state_is_unsaved: true,
        leaving_state_is_safe: true,
        leaving_mods: [],
        leaving_objects: [],
        leaving_tree_nodes: [],
        target_state_name: null,
        target_state_is_unsaved: false,
        target_state_is_safe: false,
        target_state_kind: 'none',
        target_mods: [],
        target_objects: [],
        target_tree_nodes: [],
      },
      isLoading: false,
    });

    renderModal();

    expect(screen.getByText('All Disabled')).toBeInTheDocument();
  });
});
