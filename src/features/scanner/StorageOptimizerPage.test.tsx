import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import StorageOptimizerPage from './StorageOptimizerPage';

const startScanMutate = vi.fn();
const cancelScanMutate = vi.fn();
const startStoredScan = vi.fn();
const applyScanEvent = vi.fn();
const stopStoredScan = vi.fn();
const cancelScanState = { isPending: false };

const activeGameState = {
  current: { id: 'game-1', mod_path: 'D:/Mods' },
};

const dedupScanState = {
  gameId: null as string | null,
  progress: {
    isScanning: false,
    totalFolders: 0,
    scannedFolders: 0,
    currentFolder: '',
    error: '',
  },
  startScan: startStoredScan,
  applyEvent: applyScanEvent,
  stopScan: stopStoredScan,
};

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: activeGameState.current }),
}));

vi.mock('./hooks/useDedup', () => ({
  useStartDedupScan: () => ({ mutate: startScanMutate }),
  useCancelDedupScan: () => ({ mutate: cancelScanMutate, isPending: cancelScanState.isPending }),
  useIgnoredPairs: () => ({ data: [] }),
}));

vi.mock('./stores/useDedupScanStore', () => ({
  useDedupScanStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector(dedupScanState),
}));

vi.mock('./components/DedupFeature', () => ({
  default: () => <div>Duplicate report</div>,
}));

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe('StorageOptimizerPage', () => {
  beforeEach(() => {
    startScanMutate.mockReset();
    cancelScanMutate.mockReset();
    startStoredScan.mockReset();
    startStoredScan.mockReturnValue({ gameId: 'game-1', generation: 1 });
    applyScanEvent.mockReset();
    stopStoredScan.mockReset();
    cancelScanState.isPending = false;
    activeGameState.current = { id: 'game-1', mod_path: 'D:/Mods' };
    dedupScanState.gameId = null;
    dedupScanState.progress = {
      isScanning: false,
      totalFolders: 0,
      scannedFolders: 0,
      currentFolder: '',
      error: '',
    };
    const portalTarget = document.createElement('div');
    portalTarget.id = 'topbar-actions-portal';
    document.body.appendChild(portalTarget);
  });

  afterEach(() => {
    document.getElementById('topbar-actions-portal')?.remove();
  });

  it('keeps scan as the primary context action', () => {
    render(<StorageOptimizerPage />);

    const startButton = screen.getByRole('button', { name: 'scanner:optimizer.start_button' });
    expect(startButton).toHaveClass('btn-primary');

    fireEvent.click(startButton);
    expect(startScanMutate).toHaveBeenCalledWith(
      expect.objectContaining({ gameId: 'game-1', modsRoot: 'D:/Mods' }),
    );
  });

  it('keeps the running scan cancellable after switching games', () => {
    activeGameState.current = { id: 'game-2', mod_path: 'E:/Mods' };
    dedupScanState.gameId = 'game-1';
    dedupScanState.progress = {
      isScanning: true,
      totalFolders: 4,
      scannedFolders: 2,
      currentFolder: 'D:/Mods/Character',
      error: '',
    };
    cancelScanMutate.mockImplementation(
      (_variables: undefined, callbacks?: { onSettled?: () => void }) => callbacks?.onSettled?.(),
    );

    render(<StorageOptimizerPage />);

    fireEvent.click(screen.getByRole('button', { name: 'scanner:optimizer.stop_button' }));
    expect(cancelScanMutate).toHaveBeenCalled();
    expect(stopStoredScan).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'scanner:optimizer.stop_button' })).toBeEnabled();
    expect(screen.queryByRole('button', { name: 'scanner:optimizer.start_button' })).toBeNull();
  });

  it('disables Stop while the cancellation request is pending', () => {
    dedupScanState.gameId = 'game-1';
    dedupScanState.progress.isScanning = true;
    cancelScanState.isPending = true;

    render(<StorageOptimizerPage />);

    fireEvent.click(screen.getByRole('button', { name: 'scanner:optimizer.stop_button' }));
    expect(screen.getByRole('button', { name: 'scanner:optimizer.stop_button' })).toBeDisabled();
    expect(cancelScanMutate).not.toHaveBeenCalled();
  });
});
