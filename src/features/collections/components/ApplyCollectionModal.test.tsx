import { fireEvent, render, screen, waitFor } from '../../../testing/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApplyCollectionModal } from './ApplyCollectionModal';
import { useAppStore } from '../../../stores/useAppStore';

const mutateAsync = vi.fn();
const replaceMutateAsync = vi.fn();
const replaceMutate = vi.fn();

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
  useReplaceCollectionWithCurrentState: () => ({
    mutate: replaceMutate,
    mutateAsync: replaceMutateAsync,
    isPending: false,
  }),
  useApplyProgress: () => ({
    data: null,
  }),
}));

describe('ApplyCollectionModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    replaceMutate.mockImplementation((params: unknown) => {
      void replaceMutateAsync(params);
    });
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

  it('offers updating the original collection after partial apply', async () => {
    mutateAsync.mockResolvedValueOnce({
      success: true,
      mods_enabled: 1,
      mods_disabled: 0,
      objects_toggled: 0,
      undo_collection_id: null,
      new_signature: 'partial-signature',
      warnings: ['Missing mod on disk: Mods/Missing'],
      final_state_name: 'Unsaved SAFE Preset',
      final_mode: 'SAFE',
      partial_apply: true,
      skipped_missing_paths: ['Mods/Missing'],
      final_state_is_dirty: true,
      runtime_path_rewrites: [],
    });
    replaceMutateAsync.mockResolvedValueOnce({
      id: 'collection-1',
      name: 'Test Preset',
      is_safe: true,
      is_unsaved: false,
      is_active: true,
      is_undo_target: false,
      signature: 'partial-signature',
      updated_at: 'now',
      member_count: 1,
      mod_count: 1,
    });

    render(<ApplyCollectionModal collectionId="collection-1" onClose={vi.fn()} />);

    fireEvent.click(screen.getByTestId('modal-apply-btn'));

    await waitFor(() => {
      expect(screen.getByText('Update Original Collection')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Update Original Collection'));

    await waitFor(() => {
      expect(replaceMutate).toHaveBeenCalledWith(
        {
          gameId: 'game-1',
          collectionId: 'collection-1',
        },
        expect.objectContaining({
          onSuccess: expect.any(Function),
        }),
      );
      expect(replaceMutateAsync).toHaveBeenCalledWith({
        gameId: 'game-1',
        collectionId: 'collection-1',
      });
    });
  });
});
