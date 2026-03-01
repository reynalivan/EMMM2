import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import GeneralTab from './GeneralTab';

let mockAutoClose = false;
const mockSetAutoClose = vi.fn();

vi.mock('../../../stores/useAppStore', () => ({
  useAppStore: () => ({
    autoCloseLauncher: mockAutoClose,
    setAutoCloseLauncher: mockSetAutoClose,
  }),
}));

describe('GeneralTab (TC-04)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAutoClose = false;
  });

  it('renders Appearance and System sections', () => {
    render(<GeneralTab />);
    expect(screen.getByText('Appearance')).toBeInTheDocument();
    expect(screen.getByText('System Information')).toBeInTheDocument();
  });

  it('toggles Auto-Close launcher setting', () => {
    render(<GeneralTab />);

    // It starts with our mocked false value
    const toggle = screen.getByRole('checkbox', { name: /Auto-Close on Launch/i });
    expect(toggle).not.toBeChecked();

    fireEvent.click(toggle);

    // Should call store function with true
    expect(mockSetAutoClose).toHaveBeenCalledWith(true);
  });

  it('reflects initial store state on toggle', () => {
    mockAutoClose = true;
    render(<GeneralTab />);

    const toggle = screen.getByRole('checkbox', { name: /Auto-Close on Launch/i });
    expect(toggle).toBeChecked();
  });
});
