import { describe, expect, it, vi } from 'vitest';
import type { DiskReconcileProgress } from '@/shared/api/tauri/bindings';
import { render, screen } from '@/tests/testing/test-utils';
import { GameIndexingOverlay } from './GameIndexingOverlay';

function makeProgress(overrides: Partial<DiskReconcileProgress> = {}): DiskReconcileProgress {
  return {
    game_id: 'srmi',
    run_id: 'srmi-1',
    reason: 'ManualRepair',
    phase: 'ScanningRoots',
    completed_units: 2,
    total_units: 5,
    current_root: 'C:\\Games\\SRMI\\Mods\\#Character',
    elapsed_ms: 1_500,
    eta_ms: 2_000,
    ...overrides,
  };
}

describe('GameIndexingOverlay', () => {
  it('renders backend-backed indexing progress for the selected game', () => {
    render(<GameIndexingOverlay gameName="Star Rail" progress={makeProgress()} />);

    expect(screen.getByRole('status')).toHaveAttribute('aria-busy', 'true');
    expect(screen.getByRole('status')).toHaveClass('h-full');
    expect(screen.getByRole('heading', { name: 'Indexing Star Rail' })).toBeInTheDocument();
    expect(screen.getByText('Scanning Character')).toBeInTheDocument();
    expect(screen.queryByText(/C:\\Games/)).toBeNull();

    const progressbar = screen.getByRole('progressbar', { name: 'Indexing progress' });
    expect(progressbar).toHaveAttribute('aria-valuemin', '0');
    expect(progressbar).toHaveAttribute('aria-valuemax', '5');
    expect(progressbar).toHaveAttribute('aria-valuenow', '2');
  });

  it('does not invent a progress percentage before the scan has a total', () => {
    render(<GameIndexingOverlay gameName="Star Rail" progress={null} />);

    expect(screen.getByText('Preparing the full check')).toBeInTheDocument();
    expect(screen.queryByRole('progressbar')).toBeNull();
  });

  it('shows preparation progress before disk reconcile begins', () => {
    render(
      <GameIndexingOverlay
        gameName="Star Rail"
        progress={null}
        backgroundPhase="Preparing"
        snapshotProgress={{
          session_id: 'session-1',
          game_id: 'srmi',
          phase: 'Classifying',
          completed_games: 0,
          total_games: 2,
          completed_roots: 3,
          total_roots: 8,
          folders_classified: 42,
          current_root: 'C:\\Games\\SRMI\\Mods\\#Character',
          elapsed_ms: 700,
        }}
      />,
    );

    expect(screen.getByText('Classifying mod folders')).toBeInTheDocument();
    expect(screen.getByText('42 folders classified')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '3');
  });

  it('shows final validation as a separate phase after scanning all folders', () => {
    render(
      <GameIndexingOverlay
        gameName="Star Rail"
        progress={makeProgress({ phase: 'Finalizing', completed_units: 5 })}
      />,
    );

    expect(screen.getByText('Finishing up')).toBeInTheDocument();
    expect(screen.queryByRole('progressbar')).toBeNull();
  });

  it('shows failure and lets the user retry', () => {
    const onRetry = vi.fn();
    render(
      <GameIndexingOverlay
        gameName="Star Rail"
        progress={null}
        phase="failed"
        error="Indexing failed"
        onRetry={onRetry}
      />,
    );

    expect(screen.getByRole('alert')).toHaveTextContent('Indexing failed');
    screen.getByRole('button', { name: 'Retry' }).click();
    expect(onRetry).toHaveBeenCalledOnce();
  });
});
