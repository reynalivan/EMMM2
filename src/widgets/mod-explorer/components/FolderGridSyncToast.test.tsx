import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import { act, render, screen } from '../../../tests/testing/test-utils';
import { useAppStore } from '@/app/store';
import type { DiskReconcileProgress } from '@/shared/api/tauri/bindings';
import FolderGridSyncToast from './FolderGridSyncToast';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

function progress(reason: DiskReconcileProgress['reason']): DiskReconcileProgress {
  return {
    game_id: 'game-1',
    run_id: 'run-1',
    reason,
    phase: 'ScanningRoots',
    completed_units: 1,
    total_units: 2,
    current_root: 'Alice',
    elapsed_ms: 20,
    eta_ms: 20,
  };
}

function setProgress(overrides: Partial<DiskReconcileProgress> = {}) {
  useAppStore.setState((state) => ({
    diskReconcileByGame: {
      ...state.diskReconcileByGame,
      'game-1': {
        ...state.diskReconcileByGame['game-1']!,
        progress: { ...progress('WatcherBatch'), ...overrides },
      },
    },
  }));
}

describe('FolderGridSyncToast', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  beforeEach(() => {
    vi.useFakeTimers();
    useAppStore.setState({
      activeGameId: 'game-1',
      diskReconcileByGame: {
        'game-1': {
          at: 0,
          pending: false,
          unavailable: null,
          progress: null,
          revision: 0,
        },
      },
    });
  });

  it('does not show the editing blocker for an internal mutation progress event', () => {
    useAppStore.setState((state) => ({
      diskReconcileByGame: {
        ...state.diskReconcileByGame,
        'game-1': {
          ...state.diskReconcileByGame['game-1']!,
          progress: progress('InternalMutation'),
        },
      },
    }));

    render(<FolderGridSyncToast recoveryStatus="ready" />);

    expect(screen.queryByTestId('workspace-reconcile-sync-toast')).not.toBeInTheDocument();
    expect(screen.queryByText(/Editing is temporarily unavailable/)).not.toBeInTheDocument();
  });

  it('labels watcher progress as a non-blocking refresh', () => {
    setProgress();

    render(<FolderGridSyncToast recoveryStatus="ready" />);
    expect(screen.getByTestId('workspace-reconcile-sync-toast')).toHaveClass('opacity-0');
    act(() => vi.advanceTimersByTime(250));

    expect(screen.getByRole('status')).toHaveTextContent('Refreshing mods from disk…');
    expect(screen.queryByText(/Editing is temporarily unavailable/)).not.toBeInTheDocument();
    expect(screen.getByTestId('workspace-reconcile-sync-toast')).toHaveClass('opacity-100');
  });

  it('labels scanning folders and resets the count unit when projecting mods', () => {
    setProgress({ completed_units: 8, total_units: 10 });
    render(<FolderGridSyncToast recoveryStatus="ready" />);

    expect(screen.getByText('Scanning mod folders')).toBeInTheDocument();
    expect(screen.getByText('8 of 10 folders')).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: 'Scanning mod folders' })).toHaveAttribute(
      'value',
      '8',
    );

    act(() => {
      setProgress({
        phase: 'Projecting',
        completed_units: 1,
        total_units: 130,
        current_root: null,
      });
    });

    expect(screen.queryByText('8 of 10 folders')).not.toBeInTheDocument();
    expect(screen.getByText('Updating library')).toBeInTheDocument();
    expect(screen.getByText('1 / 130 mods processed')).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: 'Updating library' })).toHaveAttribute(
      'max',
      '130',
    );
    expect(screen.getByRole('progressbar')).toHaveAttribute('value', '1');
  });

  it.each([
    ['DiscoveringRoots', 10, 'Finding mod folders'],
    ['Finalizing', 10, 'Finishing up'],
    ['ScanningRoots', null, 'Scanning mod folders'],
    ['Projecting', 0, 'Updating library'],
  ] as const)('keeps %s with total %s indeterminate', (phase, total_units, activity) => {
    setProgress({ phase, total_units });
    render(<FolderGridSyncToast recoveryStatus="ready" />);

    expect(screen.getByText(activity)).toBeInTheDocument();
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    expect(screen.queryByText(/^\d.*(?:folders|mods processed)$/)).not.toBeInTheDocument();
  });

  it('keeps the recovery warning and fades out when syncing ends', () => {
    const { rerender } = render(<FolderGridSyncToast recoveryStatus="syncing" />);
    act(() => vi.advanceTimersByTime(250));
    expect(screen.getByRole('status')).toHaveTextContent('Editing is temporarily unavailable.');

    rerender(<FolderGridSyncToast recoveryStatus="ready" />);
    expect(screen.getByTestId('workspace-reconcile-sync-toast')).toHaveClass('opacity-0');
    act(() => vi.advanceTimersByTime(150));
    expect(screen.queryByTestId('workspace-reconcile-sync-toast')).not.toBeInTheDocument();
  });
});
