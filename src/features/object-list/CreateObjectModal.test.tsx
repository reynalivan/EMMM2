import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import CreateObjectModal from './CreateObjectModal';
import { useCreateObject } from '../../hooks/useObjects';

vi.mock('../../hooks/useObjects', () => ({
  useCreateObject: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false, isError: false })),
  useGameSchema: vi.fn(() => ({
    data: { categories: [{ name: 'Character', label: 'Characters', filters: [] }] },
  })),
}));
vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(() => ({ activeGame: { id: 'game-1' } })),
}));
vi.mock('../../stores/useToastStore', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

// Mock dialog natively so it doesn't complain about modal methods if needed, though here it's customized div
describe('CreateObjectModal', () => {
  it('renders correctly and validates input', async () => {
    render(<CreateObjectModal open={true} onClose={vi.fn()} />);

    expect(screen.getByText('Create New Object')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Create Object'));

    await waitFor(() => {
      expect(screen.getByText(/Name must be at least 2 characters/i)).toBeInTheDocument();
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
});
