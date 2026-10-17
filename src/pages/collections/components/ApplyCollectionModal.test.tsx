import { fireEvent, render, screen, waitFor } from '../../../tests/testing/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApplyCollectionModal } from './ApplyCollectionModal';
import { useAppStore } from '@/app/store';

const mutateAsync = vi.fn();
const replaceMutateAsync = vi.fn();
const replaceMutate = vi.fn();
const dialogState = vi.hoisted(() => ({ applying: false, replacing: false, previewError: false }));

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
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
      current_tree_nodes: [],
      target_tree_nodes: [],
      effective_target_tree_nodes: [],
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
      effective_target_projected_state: {
        summary: {
          active_root_count: 0,
          enabled_object_count: 0,
          object_count: 0,
        },
      },
      safe_mode_enabled: false,
    },
    isLoading: false,
    isError: dialogState.previewError,
    error: dialogState.previewError ? new Error('Preview unavailable') : null,
  }),
  useApplyCollection: () => ({
    mutateAsync,
    isPending: dialogState.applying,
  }),
  useReplaceCollectionWithCurrentState: () => ({
    mutate: replaceMutate,
    mutateAsync: replaceMutateAsync,
    isPending: dialogState.replacing,
  }),
  useApplyProgress: () => ({
    data: null,
  }),
}));

describe('ApplyCollectionModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    dialogState.applying = false;
    dialogState.replacing = false;
    dialogState.previewError = false;
    replaceMutate.mockImplementation((params: unknown) => {
      void replaceMutateAsync(params);
    });
    useAppStore.setState({
      activeGameId: 'game-1',
    });
  });

  it('opens a native modal and dismisses an idle cancel request', () => {
    const onClose = vi.fn();
    render(<ApplyCollectionModal collectionId="collection-1" onClose={onClose} />);
    const dialog = screen.getByRole('dialog');
    expect(HTMLDialogElement.prototype.showModal).toHaveBeenCalled();
    expect(dialog).toHaveAttribute('open');
    fireEvent(dialog, new Event('cancel', { cancelable: true }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it.each(['applying', 'replacing'] as const)('blocks dismissal while %s', (operation) => {
    dialogState[operation] = true;
    const onClose = vi.fn();
    render(<ApplyCollectionModal collectionId="collection-1" onClose={onClose} />);
    const dialog = screen.getByRole('dialog');
    const cancel = new Event('cancel', { cancelable: true });
    fireEvent(dialog, cancel);
    expect(cancel.defaultPrevented).toBe(true);
    fireEvent(dialog, new Event('close'));
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByText('common:actions.close')).toBeDisabled();
  });

  it('allows dismissal when preview loading fails', () => {
    dialogState.previewError = true;
    const onClose = vi.fn();
    render(<ApplyCollectionModal collectionId="collection-1" onClose={onClose} />);
    expect(HTMLDialogElement.prototype.showModal).toHaveBeenCalled();
    fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
    expect(onClose).toHaveBeenCalledOnce();
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
      mods_enabled: 1,
      mods_disabled: 0,
      warnings: ['Missing mod on disk: Mods/Missing'],
      final_state_name: 'Preset',
      partial_apply: true,
      skipped_missing_paths: ['Mods/Missing'],
      runtime_path_rewrites: [],
    });
    replaceMutateAsync.mockResolvedValueOnce({
      id: 'collection-1',
      name: 'Test Preset',
      is_safe: true,
      is_active: true,
      signature: 'partial-signature',
      updated_at: 'now',
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
