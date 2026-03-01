import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import WelcomeScreen from './WelcomeScreen';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { GameConfig } from '../../types/game';

// Mock Tauri dependencies
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

// Mock heavily styled/animated child components to simplify the test tree
vi.mock('../welcome/AuroraBackground', () => ({
  default: () => <div data-testid="aurora-bg">Aurora</div>,
}));
vi.mock('../welcome/SmartDemoStrip', () => ({
  default: () => <div data-testid="demo-strip">Strip</div>,
}));
vi.mock('../welcome/AnimatedLogo', () => ({
  default: () => <div data-testid="logo">Logo</div>,
}));
vi.mock('./ManualSetupForm', () => ({
  default: ({
    onBack,
    onComplete,
  }: {
    onBack: () => void;
    onComplete: (game: { id: string }) => void;
  }) => (
    <div data-testid="manual-form">
      Manual Setup
      <button onClick={() => onComplete({ id: 'new-game' })}>Finish</button>
      <button onClick={onBack}>Go Back</button>
    </div>
  ),
}));
vi.mock('./AutoDetectResult', () => ({
  default: ({
    games,
    onContinue,
    onAddMore,
    onRemoveGame,
  }: {
    games: { id: string; name: string }[];
    onContinue: () => void;
    onAddMore: () => void;
    onRemoveGame: (id: string) => void;
  }) => (
    <div data-testid="result-screen">
      Result Screen: {games.length} games
      <button onClick={onContinue}>Result Continue</button>
      <button onClick={onAddMore}>Result Add More</button>
      <button onClick={() => onRemoveGame('new-game')}>Remove</button>
    </div>
  ),
}));

describe('WelcomeScreen (TC-03)', () => {
  const mockOnComplete = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders initial welcome state properly', () => {
    render(<WelcomeScreen onComplete={mockOnComplete} />);
    expect(screen.getByText('Welcome to EMMM2')).toBeInTheDocument();
    expect(screen.getByText('XXMI Auto-Detect')).toBeInTheDocument();
    expect(screen.getByText('Add Game Manually')).toBeInTheDocument();
  });

  it('switches to manual mode', () => {
    render(<WelcomeScreen onComplete={mockOnComplete} />);
    fireEvent.click(screen.getByText('Add Game Manually'));

    // Confirm the WelcomeScreen elements are gone
    expect(screen.queryByText('Welcome to EMMM2')).not.toBeInTheDocument();
    // Confirm the Manual form is present
    expect(screen.getByTestId('manual-form')).toBeInTheDocument();
  });

  it('runs auto-detect flow normally', async () => {
    (open as ReturnType<typeof vi.fn>).mockResolvedValue('C:\\Launcher');
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValue([{ id: '1', name: 'AutoGame' }]);

    render(<WelcomeScreen onComplete={mockOnComplete} />);

    fireEvent.click(screen.getByText('XXMI Auto-Detect'));

    // Wait for async resolution to reach the Results component
    await waitFor(() => {
      expect(screen.getByTestId('result-screen')).toBeInTheDocument();
      expect(screen.getByText('Result Screen: 1 games')).toBeInTheDocument();
    });
  });

  it('shows scanning loader state during long auto_detect_games', async () => {
    (open as ReturnType<typeof vi.fn>).mockResolvedValue('C:\\Launcher');
    // Lock promise so it stays scanning
    let unblock: (value: unknown) => void = () => {};
    const block = new Promise((resolve) => {
      unblock = resolve;
    });
    (invoke as ReturnType<typeof vi.fn>).mockReturnValue(block);

    render(<WelcomeScreen onComplete={mockOnComplete} />);
    fireEvent.click(screen.getByText('XXMI Auto-Detect'));

    await waitFor(() => {
      // It should display 'Scanning for games...'
      expect(screen.getByText(/Scanning for games/i)).toBeInTheDocument();
    });

    unblock([{ id: '1', name: 'AutoGame' } as unknown as GameConfig]);

    await waitFor(() => {
      expect(screen.getByTestId('result-screen')).toBeInTheDocument();
    });
  });

  it('shows error if auto-detect fails', async () => {
    (open as ReturnType<typeof vi.fn>).mockResolvedValue('C:\\Launcher');
    (invoke as ReturnType<typeof vi.fn>).mockRejectedValue('Fake error from backend');

    render(<WelcomeScreen onComplete={mockOnComplete} />);
    fireEvent.click(screen.getByText('XXMI Auto-Detect'));

    await waitFor(() => {
      expect(screen.getByText('Fake error from backend')).toBeInTheDocument();
      // It bounces back to welcome screen automatically
      expect(screen.getByText('Welcome to EMMM2')).toBeInTheDocument();
    });
  });

  it('handles result interactions', async () => {
    render(<WelcomeScreen onComplete={mockOnComplete} />);

    // Manually navigate to manual and simulate adding a game
    fireEvent.click(screen.getByText('Add Game Manually'));
    fireEvent.click(screen.getByText('Finish')); // Trigger `onComplete` in our mock

    // Should bump to result screen
    await waitFor(() => {
      expect(screen.getByTestId('result-screen')).toBeInTheDocument();
    });

    // Check we can navigate from Result -> Manual
    fireEvent.click(screen.getByText('Result Add More'));
    await waitFor(() => {
      expect(screen.getByTestId('manual-form')).toBeInTheDocument();
    });

    // Go back to result
    fireEvent.click(screen.getByText('Go Back'));
    await waitFor(() => {
      expect(screen.getByTestId('result-screen')).toBeInTheDocument();
    });

    (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
    fireEvent.click(screen.getByText('Remove'));

    await waitFor(() => {
      expect(screen.getByText('Welcome to EMMM2')).toBeInTheDocument();
    });
  });
});
