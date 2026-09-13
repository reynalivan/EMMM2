import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import CreateObjectModal from './CreateObjectModal';
import { useCreateObject } from '@/features/workspace-runtime';

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({
    t: (key: string) => {
      const labels: Record<string, string> = {
        'create_modal.title': 'Create New Object',
        'create_modal.submit': 'Create Object',
        'create_modal.placeholder_name': 'e.g. Eula',
        'create_modal.validation.name_too_short': 'Name must have at least 2 characters.',
      };

      return labels[key] ?? key;
    },
  }),
}));

vi.mock('@/features/workspace-runtime', () => ({
  useCreateObject: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false, isError: false })),
}));

vi.mock('../hooks/useObjectQueries', () => ({
  useGameSchema: vi.fn(() => ({
    data: {
      categories: [
        { name: 'Character', label: 'Characters', filters: [], subcategories: [] },
        { name: 'Other', label: 'Other', filters: [], subcategories: ['Bangboo'] },
      ],
    },
  })),
}));
vi.mock('@/entities/game', () => ({
  useActiveGame: vi.fn(() => ({ activeGame: { id: 'game-1' } })),
}));
vi.mock('@/shared/ui/toast', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

// Mock dialog natively so it doesn't complain about modal methods if needed, though here it's customized div
describe('CreateObjectModal', () => {
  it('renders correctly and validates input', async () => {
    render(<CreateObjectModal open={true} onClose={vi.fn()} />);

    expect(screen.getByText('Create New Object')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Create Object'));

    await waitFor(() => {
      expect(screen.getByText(/Name must have at least 2 characters/i)).toBeInTheDocument();
    });
  });

  it('submits form correctly', async () => {
    const mockMutateAsync = vi.fn().mockResolvedValue('new-id');
    vi.mocked(useCreateObject).mockReturnValue({
      mutateAsync: mockMutateAsync,
    } as unknown as ReturnType<typeof useCreateObject>);

    const onClose = vi.fn();
    render(<CreateObjectModal open={true} onClose={onClose} />);

    fireEvent.change(screen.getByPlaceholderText('e.g. Eula'), { target: { value: 'NewChar' } });
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'Character' } });

    fireEvent.click(screen.getByText('Create Object'));

    await waitFor(() => {
      expect(mockMutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'NewChar',
          object_type: 'Character',
        }),
      );
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('only shows subcategories declared by the selected schema category', () => {
    render(<CreateObjectModal open={true} onClose={vi.fn()} />);

    expect(screen.queryByText('create_modal.sub_category')).not.toBeInTheDocument();

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'Other' } });

    expect(screen.getByText('create_modal.sub_category')).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Bangboo' })).toBeInTheDocument();
  });

  it('can open after an initial closed render without changing hook order', () => {
    const { rerender } = render(<CreateObjectModal open={false} onClose={vi.fn()} />);

    expect(screen.queryByText('Create New Object')).not.toBeInTheDocument();
    rerender(<CreateObjectModal open onClose={vi.fn()} />);
    expect(screen.getByText('Create New Object')).toBeInTheDocument();
  });
});
