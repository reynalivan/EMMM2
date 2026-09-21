import { act, render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import WelcomeScreen from './WelcomeScreen';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { GameType, type GameConfig } from '@/entities/game';

// Mock Tauri dependencies
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => undefined) }));

// Mock heavily styled/animated child components to simplify the test tree
vi.mock('./components/welcome/AuroraBackground', () => ({
  default: () => <div data-testid="aurora-bg">Aurora</div>,
}));
vi.mock('./components/welcome/SmartDemoStrip', () => ({
  default: () => <div data-testid="demo-strip">Strip</div>,
}));
vi.mock('./components/welcome/AnimatedLogo', () => ({
  default: () => <div data-testid="logo">Logo</div>,
}));
vi.mock('./components/ManualSetupForm', () => ({
  ManualSetupForm: ({
    onBack,
    onSuccess,
  }: {
    onBack: () => void;
    onSuccess: (game: GameConfig) => void;
  }) => (
    <div data-testid="manual-form">
      Manual Setup
      <button
        onClick={() =>
          onSuccess({
            id: 'new-game',
            name: 'New Game',
            game_type: GameType.GIMI,
            instance_path: 'C:/Instance',
            mod_path: 'C:/Mods',
            launch_mode: 'standalone',
            game_exe: 'C:/Game.exe',
            loader_exe: null,
            xxmi_launcher_exe: null,
            launch_args: null,
          })
        }
      >
        Finish
      </button>
      <button onClick={onBack}>Go Back</button>
    </div>
  ),
}));
vi.mock('./components/AutoDetectResult', () => ({
  AutoDetectResult: ({
    games,
    onConfirm,
    onAddMore,
    onBack,
    onRemoveGame,
  }: {
    games: { id: string; name: string }[];
    onConfirm: () => void;
    onAddMore: () => void;
    onBack: () => void;
    onRemoveGame: (id: string) => void;
  }) => (
    <div data-testid="result-screen">
      Result Screen: {games.length} games
      <button onClick={onConfirm}>Result Continue</button>
      <button onClick={onAddMore}>Result Add More</button>
      <button onClick={onBack}>Result Back</button>
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
    expect(screen.getByText('Welcome to EMMM')).toBeInTheDocument();
    expect(screen.getByText('XXMI Auto-Detect')).toBeInTheDocument();
    expect(screen.getByText('Add Game Manually')).toBeInTheDocument();
  });

  it('switches to manual mode', () => {
    render(<WelcomeScreen onComplete={mockOnComplete} />);
    fireEvent.click(screen.getByText('Add Game Manually'));

    // Confirm the WelcomeScreen elements are gone
    expect(screen.queryByText('Welcome to EMMM')).not.toBeInTheDocument();
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
      expect(screen.getByText('Welcome to EMMM')).toBeInTheDocument();
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
      expect(screen.getByText('Welcome to EMMM')).toBeInTheDocument();
    });
  });

  it('opens the dashboard after the first game and hands the remaining games to the backend', async () => {
    (open as ReturnType<typeof vi.fn>).mockResolvedValue('C:\\Launcher');
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'auto_detect_games') {
        return Promise.resolve([
          { id: 'first-game', name: 'First Game' },
          { id: 'second-game', name: 'Second Game' },
        ]);
      }
      if (command === 'begin_onboarding_indexing') {
        return Promise.resolve({ session_id: 'session-1' });
      }
      if (command === 'reconcile_onboarding_indexing_game') {
        return Promise.resolve({ status: 'Applied' });
      }
      return Promise.resolve(undefined);
    });

    render(<WelcomeScreen onComplete={mockOnComplete} />);
    fireEvent.click(screen.getByText('XXMI Auto-Detect'));
    await screen.findByText('Result Screen: 2 games');

    fireEvent.click(screen.getByText('Result Continue'));

    await waitFor(() => expect(mockOnComplete).toHaveBeenCalled());
    const invokedCommands = vi.mocked(invoke).mock.calls.map(([command]) => command);
    expect(
      invokedCommands.filter((command) => command === 'reconcile_onboarding_indexing_game'),
    ).toHaveLength(1);
    expect(invokedCommands).toContain('continue_onboarding_indexing_in_background');
  });

  it('keeps onboarding open when the first game was not applied', async () => {
    (open as ReturnType<typeof vi.fn>).mockResolvedValue('C:\\Launcher');
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'auto_detect_games') {
        return Promise.resolve([{ id: 'first-game', name: 'First Game' }]);
      }
      if (command === 'begin_onboarding_indexing') {
        return Promise.resolve({ session_id: 'session-1' });
      }
      if (command === 'reconcile_onboarding_indexing_game') {
        return Promise.resolve({ status: 'SourceUnavailable' });
      }
      return Promise.resolve(undefined);
    });

    render(<WelcomeScreen onComplete={mockOnComplete} />);
    fireEvent.click(screen.getByText('XXMI Auto-Detect'));
    await screen.findByText('Result Screen: 1 games');

    fireEvent.click(screen.getByText('Result Continue'));

    await waitFor(() => {
      expect(mockOnComplete).not.toHaveBeenCalled();
    });
    expect(screen.getByRole('alert')).toHaveTextContent(
      'First Game could not be indexed. Fix the issue and try again before opening the dashboard.',
    );
  });

  it('shows phase-specific preparation progress without fabricating an overall percentage', async () => {
    let progressHandler: ((event: { payload: unknown }) => void) | undefined;
    let snapshotHandler: ((event: { payload: unknown }) => void) | undefined;
    vi.mocked(listen).mockImplementation(async (event, handler) => {
      if (event === 'disk_reconcile:progress') {
        progressHandler = handler as unknown as (event: { payload: unknown }) => void;
      }
      if (event === 'onboarding_indexing:snapshot_progress') {
        snapshotHandler = handler as unknown as (event: { payload: unknown }) => void;
      }
      return () => undefined;
    });
    let unblock: () => void = () => {};
    const reconcile = new Promise<{ status: 'Applied' }>((resolve) => {
      unblock = () => resolve({ status: 'Applied' });
    });
    (invoke as ReturnType<typeof vi.fn>)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
        session_id: 'session-1',
      })
      .mockReturnValueOnce(reconcile);

    render(<WelcomeScreen onComplete={mockOnComplete} />);
    fireEvent.click(screen.getByText('Add Game Manually'));
    fireEvent.click(screen.getByText('Finish'));
    await screen.findByTestId('result-screen');
    fireEvent.click(screen.getByText('Result Continue'));

    expect(screen.queryByText(/%/)).not.toBeInTheDocument();

    await waitFor(() => expect(snapshotHandler).toBeDefined());
    act(() => {
      snapshotHandler?.({
        payload: {
          session_id: 'session-1',
          game_id: 'new-game',
          phase: 'Metadata',
          completed_games: 0,
          total_games: 1,
          completed_roots: 1,
          total_roots: 4,
          folders_classified: 0,
          current_root: 'Alice',
          elapsed_ms: 1_000,
        },
      });
    });
    expect(screen.getByRole('progressbar', { name: /preparation progress/i })).toHaveAttribute(
      'aria-valuenow',
      '1',
    );
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuemax', '4');
    expect(screen.getByText('Checking folder structure')).toBeInTheDocument();
    expect(screen.getByText('Current folder: Alice')).toBeInTheDocument();
    expect(screen.queryByText(/%/)).not.toBeInTheDocument();

    act(() => {
      snapshotHandler?.({
        payload: {
          session_id: 'session-1',
          game_id: 'new-game',
          phase: 'Classifying',
          completed_games: 0,
          total_games: 1,
          completed_roots: 2,
          total_roots: 4,
          folders_classified: 6,
          current_root: 'Bob',
          elapsed_ms: 1_500,
        },
      });
    });
    expect(screen.getByText('Classifying mod folders')).toBeInTheDocument();
    expect(screen.getByText('6 folders classified')).toBeInTheDocument();

    await waitFor(() => expect(progressHandler).toBeDefined());
    act(() => {
      progressHandler?.({
        payload: {
          game_id: 'new-game',
          run_id: 'new-game-1',
          reason: 'OnboardingCompleted',
          phase: 'ScanningRoots',
          completed_units: 1,
          total_units: 1,
          current_root: 'Alice',
          elapsed_ms: 1_000,
          eta_ms: 2_500,
        },
      });
    });
    expect(screen.getByRole('progressbar', { name: /current stage/i })).toHaveAttribute(
      'aria-valuenow',
      '1',
    );
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuemax', '1');
    expect(screen.getByText('Current stage')).toBeInTheDocument();
    expect(screen.getByText('Current stage').closest('[aria-live]')).toHaveAttribute(
      'aria-busy',
      'true',
    );
    expect(screen.getByText('Game 1 of 1 · New Game')).toBeInTheDocument();
    expect(screen.getByText('Scanning mod folders [Alice] · Step 2 of 4')).toBeInTheDocument();
    expect(screen.queryByText(/%/)).not.toBeInTheDocument();

    act(() => {
      snapshotHandler?.({
        payload: {
          session_id: 'session-1',
          game_id: 'new-game',
          phase: 'Rechecking',
          completed_games: 0,
          total_games: 0,
          completed_roots: 0,
          total_roots: 0,
          folders_classified: 0,
          current_root: null,
          elapsed_ms: 0,
        },
      });
    });
    expect(screen.getByText('Library changed. Checking it again...')).toBeInTheDocument();
    expect(screen.queryByText(/%/)).not.toBeInTheDocument();

    unblock();
    await waitFor(() => expect(mockOnComplete).toHaveBeenCalled());
  });
});
