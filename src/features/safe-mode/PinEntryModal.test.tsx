import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import PinEntryModal from './PinEntryModal';
import { useSettings } from '../../hooks/useSettings';
import { commands } from '../../lib/bindings';

vi.mock('../../hooks/useSettings', () => ({
  useSettings: vi.fn(),
}));

vi.mock('../../lib/bindings', () => ({
  commands: {
    getPinStatus: vi.fn(),
  },
}));

describe('PinEntryModal (TC-30 Privacy & Safe Mode)', () => {
  const onSuccessMock = vi.fn();
  const onCloseMock = vi.fn();
  const verifyPinMock = vi.fn();
  const getPinStatusMock = vi.mocked(commands.getPinStatus);

  beforeEach(() => {
    getPinStatusMock.mockReset();
    vi.mocked(useSettings).mockReturnValue({
      verifyPin: verifyPinMock,
      settings: null,
    } as unknown as ReturnType<typeof useSettings>);
  });

  // TC-30-002: PIN Modal Rejection & Lockout
  it('TC-30-002: Shows error and locks out on failed PIN attempts', async () => {
    verifyPinMock.mockResolvedValueOnce(false);
    getPinStatusMock.mockResolvedValueOnce({
      has_pin: true,
      is_locked: true,
      attempts_remaining: 0,
      lockout_seconds_remaining: 30,
    });

    render(
      <PinEntryModal
        open={true}
        onClose={onCloseMock}
        onSuccess={onSuccessMock}
        title="Enter PIN"
        description="A PIN is required."
      />,
    );

    // Initial state: input is enabled
    const pinInput = screen.getByPlaceholderText('••••••');
    expect(pinInput).toBeInTheDocument();
    expect(pinInput).not.toBeDisabled();

    // Entry of full 6-digit PIN triggers validation
    fireEvent.change(pinInput, { target: { value: '111111' } });
    const submitBtn = screen.getByRole('button', { name: /Verify/i });
    fireEvent.click(submitBtn);

    // Verify error state and lockout message
    expect(await screen.findByText(/Locked for 30s/)).toBeInTheDocument();
    expect(pinInput).toBeDisabled();
    expect(onSuccessMock).not.toHaveBeenCalled();
  });

  // TC-30-004: Entering Safe Mode (via valid PIN)
  it('TC-30-004: Calls onSuccess on valid PIN entry', async () => {
    verifyPinMock.mockResolvedValueOnce(true);

    render(<PinEntryModal open={true} onClose={onCloseMock} onSuccess={onSuccessMock} />);

    const pinInput = screen.getByPlaceholderText('••••••');
    fireEvent.change(pinInput, { target: { value: '654321' } });

    const submitBtn = screen.getByRole('button', { name: /Verify/i });
    fireEvent.click(submitBtn);

    // Should close modal and call success callback
    await waitFor(() => {
      expect(onSuccessMock).toHaveBeenCalledTimes(1);
      expect(onCloseMock).toHaveBeenCalledTimes(1);
    });
  });
});
