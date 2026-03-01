import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { GamePickerModal } from './GamePickerModal';
import { useQuery } from '@tanstack/react-query';

vi.mock('@tanstack/react-query', () => ({
  useQuery: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockGames = [
  { id: 'game-1', name: 'Genshin Impact', mod_path: 'C:/Games/Genshin/Mods', schema_version: 1 },
  { id: 'game-2', name: 'Honkai Star Rail', mod_path: 'C:/Games/HSR/Mods', schema_version: 1 },
];

describe('GamePickerModal', () => {
  const onClose = vi.fn();
  const onConfirm = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('does not render when open is false', () => {
    vi.mocked(useQuery).mockReturnValue({
      data: mockGames,
      isLoading: false,
    } as never);
    const { container } = render(
      <GamePickerModal downloadIds={['1']} open={false} onClose={onClose} onConfirm={onConfirm} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('renders games list when open is true', () => {
    vi.mocked(useQuery).mockReturnValue({
      data: mockGames,
      isLoading: false,
    } as never);
    render(
      <GamePickerModal
        downloadIds={['1', '2']}
        open={true}
        onClose={onClose}
        onConfirm={onConfirm}
      />,
    );

    expect(screen.getByText('Select Target Game')).toBeInTheDocument();
    expect(screen.getByText('Genshin Impact')).toBeInTheDocument();
    expect(screen.getByText('Honkai Star Rail')).toBeInTheDocument();
  });

  it('shows loading state when query is loading', () => {
    vi.mocked(useQuery).mockReturnValue({
      data: undefined,
      isLoading: true,
    } as never);
    const { container } = render(
      <GamePickerModal downloadIds={['1']} open={true} onClose={onClose} onConfirm={onConfirm} />,
    );
    // Check for the `.loading` class element
    expect(container.querySelector('.loading')).toBeInTheDocument();
  });

  it('disables confirm button by default, enables it after selecting a game', async () => {
    vi.mocked(useQuery).mockReturnValue({
      data: mockGames,
      isLoading: false,
    } as never);
    render(
      <GamePickerModal downloadIds={['1']} open={true} onClose={onClose} onConfirm={onConfirm} />,
    );

    const confirmBtn = screen.getByText('Import Now');
    expect(confirmBtn).toBeDisabled();

    // Since daisyUI handles visibility via CSS `modal-open`, jsdom might think it's hidden
    const gameRadios = screen.getAllByRole('radio', { hidden: true }) as HTMLInputElement[];
    expect(gameRadios).toHaveLength(2);

    fireEvent.click(gameRadios[0]);

    // State update makes it enabled
    await waitFor(() => {
      expect(confirmBtn).toBeEnabled();
    });

    fireEvent.click(confirmBtn);
    expect(onConfirm).toHaveBeenCalledWith('game-1');
  });

  it('calls onClose when cancel is clicked', () => {
    vi.mocked(useQuery).mockReturnValue({
      data: mockGames,
      isLoading: false,
    } as never);
    render(
      <GamePickerModal downloadIds={['1']} open={true} onClose={onClose} onConfirm={onConfirm} />,
    );

    const cancelBtn = screen.getByText('Cancel');
    fireEvent.click(cancelBtn);
    expect(onClose).toHaveBeenCalled();
  });
});
