import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ManualSetupForm from './ManualSetupForm';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

// Mock Tauri modules
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

// Mock lucide icons
vi.mock('lucide-react', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>();
  return {
    ...actual,
    Loader2: () => <div data-testid="loader2">Loader</div>,
  };
});

describe('ManualSetupForm (TC-03)', () => {
  const mockOnBack = vi.fn();
  const mockOnComplete = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows validation errors for empty submits', async () => {
    render(<ManualSetupForm onBack={mockOnBack} onComplete={mockOnComplete} />);

    // Submit an empty form
    fireEvent.submit(screen.getByRole('button', { name: /Add Game/i }));

    // Wait for the zod schema errors to populate
    await waitFor(() => {
      expect(screen.getByText('Please select a game type')).toBeInTheDocument();
      expect(screen.getByText('Please select a game folder')).toBeInTheDocument();
    });

    // API shouldn't be called
    expect(invoke).not.toHaveBeenCalled();
  });

  it('allows browsing for a folder', async () => {
    (open as ReturnType<typeof vi.fn>).mockResolvedValue('C:\\Mocked\\Path');
    render(<ManualSetupForm onBack={mockOnBack} onComplete={mockOnComplete} />);

    fireEvent.click(screen.getByRole('button', { name: /Browse/i }));

    await waitFor(() => {
      expect(open).toHaveBeenCalled();
      const input = screen.getByDisplayValue('C:\\Mocked\\Path');
      expect(input).toBeInTheDocument();
    });
  });

  it('submits correctly and locks submit button during save', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve({ id: 'dummy' }), 100)),
    );

    render(<ManualSetupForm onBack={mockOnBack} onComplete={mockOnComplete} />);

    // Set form values
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'GIMI' } });

    // Need to trigger browse since input is readOnly
    (open as ReturnType<typeof vi.fn>).mockResolvedValue('C:\\Path');
    fireEvent.click(screen.getByRole('button', { name: /Browse/i }));

    await waitFor(() => expect(screen.getByDisplayValue('C:\\Path')).toBeInTheDocument());

    const submitBtn = screen.getByRole('button', { name: /Add Game/i });
    fireEvent.submit(submitBtn);

    // Verify it locks/shows loader
    await waitFor(() => {
      expect(submitBtn).toBeDisabled();
      expect(screen.queryByTestId('loader2')).toBeInTheDocument();
    });

    // Wait to finish
    await waitFor(() => {
      expect(mockOnComplete).toHaveBeenCalledWith({ id: 'dummy' });
    });
  });

  it('displays server errors when invoke fails', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockRejectedValue('Fake backend error');

    render(<ManualSetupForm onBack={mockOnBack} onComplete={mockOnComplete} />);

    // Set form values
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'GIMI' } });

    (open as ReturnType<typeof vi.fn>).mockResolvedValue('C:\\Path');
    fireEvent.click(screen.getByRole('button', { name: /Browse/i }));
    await waitFor(() => expect(screen.getByDisplayValue('C:\\Path')).toBeInTheDocument());

    fireEvent.submit(screen.getByRole('button', { name: /Add Game/i }));

    await waitFor(() => {
      expect(screen.getByText('Fake backend error')).toBeInTheDocument();
      // Button unlocks after error
      expect(screen.getByRole('button', { name: /Add Game/i })).not.toBeDisabled();
    });
  });
});
