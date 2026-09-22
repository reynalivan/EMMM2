import { describe, expect, it } from 'vitest';
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
    expect(screen.getByRole('status')).toHaveClass('fixed', 'pointer-events-none');
    expect(screen.getByRole('status')).not.toHaveClass('inset-0');
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
});
