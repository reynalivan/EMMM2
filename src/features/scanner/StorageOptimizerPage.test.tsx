import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import StorageOptimizerPage from './StorageOptimizerPage';

const startScanMutate = vi.fn();

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: 'game-1', mod_path: 'D:/Mods' } }),
}));

vi.mock('./hooks/useDedup', () => ({
  useStartDedupScan: () => ({ mutate: startScanMutate }),
  useCancelDedupScan: () => ({ mutate: vi.fn() }),
  useIgnoredPairs: () => ({ data: [] }),
}));

vi.mock('./stores/useDedupScanStore', () => ({
  useDedupScanStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      gameId: null,
      progress: {
        isScanning: false,
        totalFolders: 0,
        scannedFolders: 0,
        currentFolder: null,
        error: null,
      },
      startScan: vi.fn(),
      applyEvent: vi.fn(),
      stopScan: vi.fn(),
    }),
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
      expect.any(Object),
    );
  });
});
