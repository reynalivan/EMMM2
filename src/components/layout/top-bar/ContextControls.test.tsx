import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ContextControls from './ContextControls';
import { useAppStore } from '../../../stores/useAppStore';

const mockUseCollections = vi.fn();
const mockUseCollectionRuntime = vi.fn();
const mockUseCollectionRuntimeDescriptor = vi.fn();

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, defaultValue?: string) => {
      const labels: Record<string, string> = {
        'context.loading': 'Loading',
        'context.collections_title': 'Collections',
        'context.save_current': 'Save Current',
        'context.manage_collections': 'Manage Collections',
        'context.no_collections': 'No collections',
        'context.current_changes': 'Current changes',
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

vi.mock('../../../features/collections/hooks/useCollectionRuntime', () => ({
  useCollectionRuntime: (...args: unknown[]) => mockUseCollectionRuntime(...args),
  useCollectionRuntimeDescriptor: (...args: unknown[]) =>
    mockUseCollectionRuntimeDescriptor(...args),
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
          is_active: true,
          signature: null,
          updated_at: '2026-03-25T12:17:00Z',
          mod_count: 12,
        },
      ],
      isLoading: false,
    });

    mockUseCollectionRuntime.mockReturnValue({
      status: 'success',
      data: {
        game_id: 'game-1',
        active_collection_id: 'unsaved-1',
        active_collection_name: '202603251217',
        current_signature: 'sig-1',
        is_dirty: true,
      },
    });

    mockUseCollectionRuntimeDescriptor.mockReturnValue({
      status: 'success',
      data: {
        game_id: 'game-1',
        active_collection_id: 'unsaved-1',
        active_collection_name: '202603251217',
        runtime_status: 'unsaved',
        missing_count: 0,
        safety: { is_safe: true, is_safety_classified: true },
        counts: { active_mod_count: 12, object_count: 1, enabled_object_count: 1 },
        last_changes: null,
      },
    });
  });

  it('keeps a named collection visible in the dropdown despite a legacy unsaved flag', () => {
    render(<ContextControls />);

    expect(screen.getAllByText('Current changes')).toHaveLength(1);
    expect(screen.getByText('202603251217')).toBeInTheDocument();
  });

  it('shows the compact runtime descriptor in the global collection trigger', () => {
    mockUseCollectionRuntime.mockReturnValue({ status: 'success', data: undefined });
    mockUseCollectionRuntimeDescriptor.mockReturnValue({
      status: 'success',
      data: {
        game_id: 'game-1',
        active_collection_id: 'collection-1',
        active_collection_name: 'Descriptor Runtime',
        runtime_status: 'clean',
        missing_count: 0,
        safety: { is_safe: true, is_safety_classified: true },
        counts: { active_mod_count: 4, object_count: 1, enabled_object_count: 1 },
        last_changes: null,
      },
    });

    render(<ContextControls />);

    expect(screen.getByText('Descriptor Runtime')).toBeInTheDocument();
  });
});
