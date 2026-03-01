import { render, screen, fireEvent } from '../../../testing/test-utils';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import ScanOverlay from './ScanOverlay';
import { useScannerStore } from '../../../stores/useScannerStore';

describe('ScanOverlay Component (TC-25)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useScannerStore.setState({
      isScanning: false,
      progress: {
        current: 0,
        total: 100,
        folderName: '',
        label: 'Idle',
        etaMs: 0,
      },
      stats: {
        matched: 0,
        unmatched: 0,
      },
      scanResults: [],
    });

    if (!HTMLDialogElement.prototype.showModal) {
      HTMLDialogElement.prototype.showModal = function () {
        this.open = true;
      };
    }
    if (!HTMLDialogElement.prototype.close) {
      HTMLDialogElement.prototype.close = function () {
        this.open = false;
      };
    }
  });

  it('TC-25-02: Reflects scan progress emissions smoothly', () => {
    useScannerStore.setState({
      isScanning: true,
      progress: {
        current: 50,
        total: 100,
        folderName: 'TestMod',
        label: 'Processing TestMod',
        etaMs: 1000,
      },
      stats: {
        matched: 10,
        unmatched: 5,
      },
    });

    render(<ScanOverlay onCancel={vi.fn()} />);

    // Verify progress string
    expect(screen.getByText('50 / 100')).toBeInTheDocument();

    // Verify percentage (50 / 100 = 50%)
    expect(screen.getByText('50%')).toBeInTheDocument();

    // Verify label
    expect(screen.getByText('Processing TestMod')).toBeInTheDocument();

    // Verify stats
    expect(screen.getByText('10')).toBeInTheDocument(); // matched
    expect(screen.getByText('5')).toBeInTheDocument(); // unmatched
  });

  it('TC-25-05: Calls onCancel when cancel physical button is clicked', () => {
    const handleCancel = vi.fn();
    useScannerStore.setState({ isScanning: true });

    render(<ScanOverlay onCancel={handleCancel} />);

    const cancelBtn = screen.getByRole('button', { name: /cancel scan/i });
    fireEvent.click(cancelBtn);

    expect(handleCancel).toHaveBeenCalledTimes(1);
  });

  it('Shows empty/0 state when scanning starts empty', () => {
    useScannerStore.setState({
      isScanning: true,
      progress: { current: 0, total: 0, folderName: '', label: '', etaMs: 0 },
    });

    render(<ScanOverlay onCancel={vi.fn()} />);

    expect(screen.getByText('0%')).toBeInTheDocument();
    expect(screen.getByText('0 / 0')).toBeInTheDocument();
  });
});
