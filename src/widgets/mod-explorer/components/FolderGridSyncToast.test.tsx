import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import { render, screen } from '../../../tests/testing/test-utils';
import { useAppStore } from '@/app/store';
import type { DiskReconcileProgress } from '@/shared/api/tauri/bindings';
import FolderGridSyncToast from './FolderGridSyncToast';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({
    t: (key: string) =>
      key === 'banners.disk_syncing'
        ? 'Syncing mods with disk. Editing is temporarily unavailable.'
        : 'Refreshing mods from disk…',
  }),
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
    useAppStore.setState((state) => ({
      diskReconcileByGame: {
        ...state.diskReconcileByGame,
        'game-1': { ...state.diskReconcileByGame['game-1']!, progress: progress('WatcherBatch') },
      },
    }));

    render(<FolderGridSyncToast recoveryStatus="ready" />);
    vi.advanceTimersByTime(250);

    expect(screen.getByRole('status')).toHaveTextContent('Refreshing mods from disk…');
    expect(screen.queryByText(/Editing is temporarily unavailable/)).not.toBeInTheDocument();
  });
});
