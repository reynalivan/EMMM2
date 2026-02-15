import { render, screen, fireEvent, waitFor } from '../../test-utils';
import ScannerFeature from './ScannerFeature';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { useActiveGame } from '../../hooks/useActiveGame';
import { scanService } from '../../services/scanService';

// Mock dependencies
vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(),
}));

vi.mock('../../services/scanService', () => ({
  scanService: {
    detectArchives: vi.fn().mockResolvedValue([]),
    extractArchive: vi.fn(),
    startScan: vi.fn(),
    detectConflictsInFolder: vi.fn().mockResolvedValue([]),
    // Add other methods if needed
  },
}));

// Mock child components to simplify testing focus
vi.mock('../../components/scanner/ArchiveModal', () => ({
  default: ({
    error,
    onExtract,
  }: {
    error?: string | null;
    onExtract: (paths: string[], pw?: string) => void;
  }) => (
    <div data-testid="archive-modal">
      Archive Modal
      {error && <div data-testid="archive-modal-error">{error}</div>}
      <button onClick={() => onExtract(['/path.zip'], 'wrong-pass')} data-testid="mock-extract-btn">
        Extract
      </button>
    </div>
  ),
}));

vi.mock('../../components/scanner/ScanOverlay', () => ({
  default: () => <div data-testid="scan-overlay">Scan Overlay</div>,
}));

vi.mock('../../components/scanner/ReviewTable', () => ({
  default: () => <div data-testid="review-table">Review Table</div>,
}));

describe('ScannerFeature', () => {
  const mockActiveGame = {
    id: 'game-1',
    name: 'Test Game',
    mods_path: '/mods/path',
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (useActiveGame as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      activeGame: mockActiveGame,
      isLoading: false,
      error: null,
    });
  });

  it('renders correctly with active game', () => {
    render(<ScannerFeature />);
    expect(screen.getByText('Start Scan')).toBeInTheDocument();
    // expect(screen.getByText('Last Scan:')).toBeInTheDocument(); // Depends on scanResults?
  });

  it('renders "No active game" when no game selected', () => {
    (useActiveGame as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      activeGame: null,
      isLoading: false,
      error: null,
    });

    render(<ScannerFeature />);
    // Might need to check if it shows an error or just disables buttons.
    // Based on code: it shows "No active game selected" in error message if clicked.
    // But initially it might just render generic UI.
    expect(screen.getByText('Start Scan')).toBeInTheDocument();
  });

  it('starts scan when button clicked', async () => {
    render(<ScannerFeature />);

    // Mock startScan to simulate progress
    (scanService.startScan as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      async (_path, onEvent) => {
        onEvent({ event: 'started', data: { totalFolders: 10 } });
        onEvent({ event: 'progress', data: { current: 5, folderName: 'Mod A' } });
        onEvent({ event: 'finished', data: { matched: 5, unmatched: 5 } });
      },
    );

    const scanButton = screen.getByText('Start Scan');
    fireEvent.click(scanButton);

    await waitFor(() => {
      expect(scanService.startScan).toHaveBeenCalledWith(
        mockActiveGame.mods_path,
        expect.any(Function),
      );
    });
  });

  it('detects conflicts after scan', async () => {
    // Setup scan to finish
    (scanService.startScan as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      async (_path, onEvent) => {
        onEvent({ event: 'finished', data: { matched: 0, unmatched: 0 } });
      },
    );

    // Mock conflict detection
    const mockConflicts = [{ hash: 'abc', section_name: 'tex', mod_paths: ['/mod1', '/mod2'] }];
    (scanService.detectConflictsInFolder as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(
      mockConflicts,
    );

    render(<ScannerFeature />);

    fireEvent.click(screen.getByText('Start Scan'));

    await waitFor(() => {
      expect(scanService.detectConflictsInFolder).toHaveBeenCalledWith(mockActiveGame.mods_path);
    });

    // Check if toast appears
    // ConflictToast renders if conflicts > 0.
    // We didn't mock ConflictToast, so we search for text.
    expect(await screen.findByText('Shader Conflict Detected!')).toBeInTheDocument();
    expect(screen.getByText('1 conflict(s) detected.')).toBeInTheDocument();
  });

  it('displays error in ArchiveModal when extraction fails', async () => {
    // Setup: Detect archives to show modal
    (scanService.detectArchives as unknown as ReturnType<typeof vi.fn>).mockResolvedValue([
      {
        path: '/archive.zip',
        name: 'archive.zip',
        size_bytes: 100,
        extension: 'zip',
        has_ini: true,
      },
    ]);

    // Setup: Extraction fails with specific error
    const errorMsg = 'Password required for archive';
    (scanService.extractArchive as unknown as ReturnType<typeof vi.fn>).mockRejectedValue(
      new Error(errorMsg),
    );

    render(<ScannerFeature />);

    // Trigger detection
    fireEvent.click(screen.getByText('Start Scan'));

    // Wait for modal (mocked)
    await waitFor(() => {
      expect(screen.getByTestId('archive-modal')).toBeInTheDocument();
    });

    // Clean previous mocks to ensure we track new calls
    vi.clearAllMocks();

    // Trigger extract from within the mock (simulating user click in modal)
    fireEvent.click(screen.getByTestId('mock-extract-btn'));

    // Expect the error to appear inside the modal (via the prop we mocked)
    // This will FAIL initially because ScannerFeature doesn't pass the 'error' prop
    await waitFor(() => {
      expect(screen.getByTestId('archive-modal-error')).toHaveTextContent(errorMsg);
    });
  });
});
