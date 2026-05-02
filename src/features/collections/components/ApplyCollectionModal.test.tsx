import { fireEvent, render, screen, waitFor } from '../../../testing/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApplyCollectionModal } from './ApplyCollectionModal';
import { useAppStore } from '../../../stores/useAppStore';

const mutateAsync = vi.fn();

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOptions?: string | { count?: number }) => {
      if (typeof fallbackOrOptions === 'string') {
        return fallbackOrOptions;
      }

      if (key === 'apply.panels.mod_count') {
        return `${fallbackOrOptions?.count ?? 0} mods`;
      }

      return key;
    },
  }),
}));

vi.mock('../hooks/useCollections', () => ({
  useApplyCollectionPreview: () => ({
    data: {
      collection_name: 'Test Preset',
      current_mods: [],
      current_objects: [],
      current_tree_nodes: [],
      target_mods: [],
      target_objects: [],
      target_tree_nodes: [],
      current_state_name: 'Current',
      current_state_is_unsaved: false,
      current_projected_state: {
        summary: {
          active_root_count: 0,
          enabled_object_count: 0,
          object_count: 0,
        },
      },
      target_projected_state: {
        summary: {
          active_root_count: 0,
          enabled_object_count: 0,
          object_count: 0,
        },
      },
    },
    isLoading: false,
    isError: false,
    error: null,
  }),
  useApplyCollection: () => ({
    mutateAsync,
    isPending: false,
  }),
  useApplyProgress: () => ({
    data: null,
  }),
}));

describe('ApplyCollectionModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      activeGameId: 'game-1',
      safeMode: true,
    });
  });

  it('shows missing mods dialog from typed backend error payload', async () => {
    mutateAsync.mockRejectedValueOnce(
      new Error(
        JSON.stringify({
          type: 'MissingMods',
          payload: {
            count: 2,
            paths: ['Mods/A', 'Mods/B'],
          },
        }),
      ),
    );

    render(<ApplyCollectionModal collectionId="collection-1" onClose={vi.fn()} />);

    fireEvent.click(screen.getByTestId('modal-apply-btn'));

    await waitFor(() => {
      expect(screen.getByText('Missing Mods')).toBeInTheDocument();
    });

    expect(screen.getByText('Mods/A')).toBeInTheDocument();
    expect(screen.getByText('Mods/B')).toBeInTheDocument();
  });
});
