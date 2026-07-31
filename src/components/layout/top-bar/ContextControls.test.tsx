import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ContextControls from './ContextControls';
import { useAppStore } from '../../../stores/useAppStore';

const mockUseCollections = vi.fn();
const mockUseCorridor = vi.fn();

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, defaultValue?: string) => {
      const labels: Record<string, string> = {
        'context.loading': 'Loading',
        'context.collections_title': 'Collections',
        'context.save_current': 'Save Current',
        'context.manage_collections': 'Manage Collections',
        'context.no_collections': 'No collections',
        'context.unsaved_safe': 'Unsaved SAFE Preset',
        'context.unsaved_unsafe': 'Unsaved UNSAFE Preset',
      };

      if (typeof defaultValue === 'string') {
        return defaultValue;
      }

      return labels[key] ?? key;
    },
  }),
}));

vi.mock('../../../features/collections/hooks/useCollections', () => ({
  useCollections: (...args: unknown[]) => mockUseCollections(...args),
}));

vi.mock('../../../features/collections/hooks/useCorridor', () => ({
  useCorridor: (...args: unknown[]) => mockUseCorridor(...args),
}));

vi.mock('../../../features/collections/components/SaveCollectionModal', () => ({
  SaveCollectionModal: () => null,
}));

vi.mock('../../../features/collections/components/ApplyCollectionModal', () => ({
  ApplyCollectionModal: () => null,
}));

describe('ContextControls', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    useAppStore.setState({
      activeGameId: 'game-1',
      workspaceView: 'dashboard',
    });

    mockUseCollections.mockReturnValue({
      data: [
        {
          id: 'unsaved-1',
          name: '202603251217',
          is_safe: true,
          is_unsaved: true,
          is_active: true,
          signature: null,
          updated_at: '2026-03-25T12:17:00Z',
          member_count: 12,
          mod_count: 12,
        },
      ],
      isLoading: false,
    });

    mockUseCorridor.mockReturnValue({
      status: 'success',
      data: {
        game_id: 'game-1',
        is_safe: true,
        active_collection_id: 'unsaved-1',
        active_collection_name: '202603251217',
        current_signature: 'sig-1',
        is_dirty: true,
      },
    });
  });

  it('shows the same unsaved label in trigger and dropdown', () => {
    render(<ContextControls />);

    const labels = screen.getAllByText('Unsaved SAFE Preset');
    expect(labels).toHaveLength(2);
    expect(screen.queryByText('202603251217')).not.toBeInTheDocument();
  });
});
