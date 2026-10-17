import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ManualSetupForm } from './ManualSetupForm';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

const discovery = vi.hoisted(() => ({
  cancel: vi.fn(),
  validate: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@/features/game-executable-detection', () => ({
  useGameExecutableDiscovery: () => discovery,
  validateSelectedGameExecutables: discovery.validate,
  GameExecutableField: ({
    value,
    onChange,
    gameType,
    disabled,
  }: {
    value: string;
    onChange: (value: string, detected: boolean) => void;
    gameType: number;
    disabled: boolean;
  }) => (
    <div>
      <input
        aria-label="Game executable"
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value, false)}
      />
      <button
        type="button"
        disabled={disabled}
        onClick={() => onChange(`C:/Detected/${gameType}.exe`, true)}
      >
        Select executable
      </button>
    </div>
  ),
}));

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
    discovery.validate.mockResolvedValue(undefined);
  });

  it('shows validation errors for empty submits', async () => {
    render(<ManualSetupForm onBack={mockOnBack} onSuccess={mockOnComplete} />);

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
    render(<ManualSetupForm onBack={mockOnBack} onSuccess={mockOnComplete} />);

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

    render(<ManualSetupForm onBack={mockOnBack} onSuccess={mockOnComplete} />);

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

    render(<ManualSetupForm onBack={mockOnBack} onSuccess={mockOnComplete} />);

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

  it('preserves separate optional executable drafts when switching game type', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'manual', game_type: 4, game_exe: null });
    vi.mocked(open).mockResolvedValue('C:/EFMI');
    render(<ManualSetupForm onBack={mockOnBack} onSuccess={mockOnComplete} />);
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'GIMI' } });
    fireEvent.change(screen.getByLabelText('Game executable'), {
      target: { value: 'C:/CustomGenshin.exe' },
    });
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'EFMI' } });
    expect(screen.getByLabelText('Game executable')).toHaveValue('');
    fireEvent.click(screen.getByText('Select executable'));
    expect(screen.getByLabelText('Game executable')).toHaveValue('C:/Detected/4.exe');
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'GIMI' } });
    expect(screen.getByLabelText('Game executable')).toHaveValue('C:/CustomGenshin.exe');
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'EFMI' } });
    fireEvent.click(screen.getByRole('button', { name: /Browse/i }));
    await screen.findByDisplayValue('C:/EFMI');
    fireEvent.click(screen.getByRole('button', { name: /Add Game/i }));
    await waitFor(() =>
      expect(mockOnComplete).toHaveBeenCalledWith(
        { id: 'manual', game_type: 4, game_exe: 'C:/Detected/4.exe' },
        'C:/Detected/4.exe',
      ),
    );
    expect(discovery.validate).toHaveBeenCalledWith(
      [{ id: 'manual', game_type: 4, game_exe: 'C:/Detected/4.exe' }],
      new Map([['manual', 'C:/Detected/4.exe']]),
    );
  });

  it('does not finish manual setup when a discovered executable fails revalidation', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'manual', game_type: 0 });
    vi.mocked(open).mockResolvedValue('C:/GIMI');
    discovery.validate.mockRejectedValueOnce(new Error('Game executable is no longer available'));
    render(<ManualSetupForm onBack={mockOnBack} onSuccess={mockOnComplete} />);
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'GIMI' } });
    fireEvent.click(screen.getByText('Select executable'));
    fireEvent.click(screen.getByRole('button', { name: /Browse/i }));
    await screen.findByDisplayValue('C:/GIMI');
    fireEvent.click(screen.getByRole('button', { name: /Add Game/i }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Game executable is no longer available',
    );
    expect(mockOnComplete).not.toHaveBeenCalled();
    expect(screen.getByLabelText('Game executable')).toHaveValue('C:/Detected/0.exe');
  });

  it('ignores a manual importer response after navigating back', async () => {
    let finish: (value: unknown) => void = () => undefined;
    vi.mocked(invoke).mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    vi.mocked(open).mockResolvedValue('C:/GIMI');
    render(<ManualSetupForm onBack={mockOnBack} onSuccess={mockOnComplete} />);
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'GIMI' } });
    fireEvent.click(screen.getByRole('button', { name: /Browse/i }));
    await screen.findByDisplayValue('C:/GIMI');
    fireEvent.click(screen.getByRole('button', { name: /Add Game/i }));
    await waitFor(() => expect(screen.getByRole('button', { name: /Add Game/i })).toBeDisabled());
    fireEvent.click(screen.getByRole('button', { name: /Back/i }));
    finish({ id: 'stale-game', game_type: 0 });
    await waitFor(() => expect(mockOnBack).toHaveBeenCalledOnce());
    expect(mockOnComplete).not.toHaveBeenCalled();
    expect(discovery.cancel).toHaveBeenCalled();
  });
});
