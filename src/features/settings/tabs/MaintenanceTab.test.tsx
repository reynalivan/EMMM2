/* eslint-disable @typescript-eslint/no-explicit-any */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterAll } from 'vitest';
import MaintenanceTab from './MaintenanceTab';
import { invoke } from '@tauri-apps/api/core';

const mockNavigate = vi.fn();
vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockAddToast = vi.fn();
vi.mock('../../../stores/useToastStore', () => ({
  useToastStore: () => ({
    addToast: mockAddToast,
  }),
}));

const mockRunMaintenance = vi.fn();
vi.mock('../../../hooks/useSettings', () => ({
  useSettings: () => ({
    runMaintenance: mockRunMaintenance,
  }),
}));

// Mock window.confirm
const originalConfirm = window.confirm;

if (!HTMLDialogElement.prototype.showModal) {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
}
if (!HTMLDialogElement.prototype.close) {
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
  };
}

vi.mock('../../scanner/DedupFeature', () => ({
  default: () => <div data-testid="dedup-feature">DedupFeature</div>,
}));

describe('MaintenanceTab (TC-04)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.confirm = () => true;
  });

  afterAll(() => {
    window.confirm = originalConfirm;
  });

  it('handles Trash empty confirmation and invocation', async () => {
    (invoke as any).mockResolvedValue('Trash emptied');
    render(<MaintenanceTab />);

    fireEvent.click(screen.getByRole('button', { name: /Empty Trash/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('empty_trash');
      expect(mockAddToast).toHaveBeenCalledWith(
        'success',
        expect.stringContaining('Trash Emptied'),
      );
    });
  });

  it('bypasses empty trash if confirm is false', async () => {
    window.confirm = () => false;
    render(<MaintenanceTab />);

    fireEvent.click(screen.getByRole('button', { name: /Empty Trash/i }));

    // Should NOT call invoke
    expect(invoke).not.toHaveBeenCalled();
  });

  it('triggers maintenance hook correctly', () => {
    render(<MaintenanceTab />);
    fireEvent.click(screen.getByRole('button', { name: /Run Maintenance/i }));
    expect(mockRunMaintenance).toHaveBeenCalled();
  });

  it('triggers Clear Cache command', async () => {
    (invoke as any).mockResolvedValue('Cache cleared 12 items');
    render(<MaintenanceTab />);

    fireEvent.click(screen.getByRole('button', { name: /Clear Old Cache/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('clear_old_thumbnails');
      expect(mockAddToast).toHaveBeenCalledWith('success', 'Cache cleared 12 items');
    });
  });

  it('handles Database Reset with modal confirmation and navigation', async () => {
    (invoke as any).mockResolvedValue(true);

    // We mock localStorage
    const spyRemoveItem = vi.spyOn(Storage.prototype, 'removeItem');

    render(<MaintenanceTab />);

    // Click initial button to open Modal
    fireEvent.click(screen.getByRole('button', { name: /Reset & Re-Setup/i }));

    // Inside modal, click confirm
    fireEvent.click(screen.getByRole('button', { name: /Yes, Reset Everything/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('reset_database');
      expect(spyRemoveItem).toHaveBeenCalledWith('vibecode-storage');
      expect(mockAddToast).toHaveBeenCalledWith(
        'success',
        expect.stringContaining('Database reset'),
      );
      expect(mockNavigate).toHaveBeenCalledWith('/welcome', { replace: true });
    });
  });
});
