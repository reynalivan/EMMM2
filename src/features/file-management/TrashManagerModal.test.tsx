import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import TrashManagerModal from './TrashManagerModal';
import * as folderCoreHooks from '../../hooks/useFolderCoreMutations';
import * as folderMutationHooks from '../../hooks/useFolderMutations';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const labels: Record<string, string> = {
        'objects:trash.title': 'Trash',
        'objects:trash.restore_tip': 'Restore to original location',
        'objects:trash.empty_tip': 'Permanently delete all items',
        'objects:trash.empty_state': 'Trash is empty',
        'common:action.close': 'Close',
        'common:date.just_now': 'just now',
      };

      return labels[key] ?? key;
    },
  }),
}));

vi.mock('../../hooks/useFolderCoreMutations', () => ({
  useRestoreMod: vi.fn(),
}));

vi.mock('../../hooks/useFolderMutations', () => ({
  useListTrash: vi.fn(),
  useEmptyTrash: vi.fn(),
}));

vi.mock('../../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(() => ({ activeGame: { id: 'genshin' } })),
}));

describe('TrashManagerModal (TC-22)', () => {
  const mockRestore = vi.fn();
  const mockEmptyTrash = vi.fn();
  const mockRefetch = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    HTMLDialogElement.prototype.showModal = vi.fn();
    HTMLDialogElement.prototype.close = vi.fn();
    (folderCoreHooks.useRestoreMod as ReturnType<typeof vi.fn>).mockReturnValue({
      mutateAsync: mockRestore,
      isPending: false,
    });
    (folderMutationHooks.useEmptyTrash as ReturnType<typeof vi.fn>).mockReturnValue({
      mutateAsync: mockEmptyTrash,
      isPending: false,
    });
  });

  it('TC-22-04: View Trash Manager (Renders items correctly)', () => {
    (folderMutationHooks.useListTrash as ReturnType<typeof vi.fn>).mockReturnValue({
      data: [
        {
          id: 'uuid-1',
          original_name: 'Ayaka Mod',
          original_path: 'E:\\Mods\\Ayaka Mod',
          deleted_at: new Date().toISOString(),
          size_bytes: 1024 * 1024 * 5, // 5 MB
          game_id: 'genshin',
        },
        {
          id: 'uuid-2',
          original_name: 'Raiden Mod',
          original_path: 'E:\\Mods\\Raiden Mod',
          deleted_at: new Date(Date.now() - 86400000).toISOString(), // 1 day ago
          size_bytes: 1024 * 1024 * 50, // 50 MB
          game_id: 'genshin',
        },
      ],
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });

    render(<TrashManagerModal open={true} onClose={vi.fn()} />);

    expect(screen.getByText('Trash')).toBeInTheDocument();
    expect(screen.getByText('Ayaka Mod')).toBeInTheDocument();
    expect(screen.getByText('Raiden Mod')).toBeInTheDocument();
    expect(screen.getByText('55 MB', { exact: false })).toBeInTheDocument();
  });

  it('TC-22-05: Restore Discarded Mod triggers mutation', async () => {
    (folderMutationHooks.useListTrash as ReturnType<typeof vi.fn>).mockReturnValue({
      data: [
        {
          id: 'uuid-1',
          original_name: 'Ayaka Mod',
          original_path: 'E:\\Mods\\Ayaka Mod',
          deleted_at: new Date().toISOString(),
          size_bytes: 1024,
          game_id: 'genshin',
        },
      ],
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    mockRestore.mockResolvedValueOnce(undefined);

    render(<TrashManagerModal open={true} onClose={vi.fn()} />);

    const restoreBtn = screen.getByTitle('Restore to original location');
    fireEvent.click(restoreBtn);

    expect(mockRestore).toHaveBeenCalledWith({ trashId: 'uuid-1', gameId: 'genshin' });
  });

  it('TC-22-06: Clean Custom Trash DB (Empty Trash)', async () => {
    (folderMutationHooks.useListTrash as ReturnType<typeof vi.fn>).mockReturnValue({
      data: [
        {
          id: 'uuid-1',
          original_name: 'Ayaka Mod',
          original_path: 'E:\\Mods\\Ayaka Mod',
          deleted_at: new Date().toISOString(),
          size_bytes: 1024,
          game_id: 'genshin',
        },
      ],
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    mockEmptyTrash.mockResolvedValueOnce(1);

    render(<TrashManagerModal open={true} onClose={vi.fn()} />);

    const emptyBtn = screen.getByTitle('Permanently delete all items');
    fireEvent.click(emptyBtn);

    expect(mockEmptyTrash).toHaveBeenCalled();
  });

  it('Renders empty state', () => {
    (folderMutationHooks.useListTrash as ReturnType<typeof vi.fn>).mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });

    render(<TrashManagerModal open={true} onClose={vi.fn()} />);
    expect(screen.getByText('Trash is empty')).toBeInTheDocument();
  });
});
