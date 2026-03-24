import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useSafeModeToggle } from '../features/collections/hooks/useSafeModeToggle';
import { useAppStore } from '../stores/useAppStore';

// -- Mocks --

const switchMutateMock = vi.fn();

vi.mock('../features/collections/hooks/useCorridorSwitch', () => ({
  useCorridorSwitch: () => ({
    mutate: switchMutateMock,
    isPending: false,
  }),
}));

let hasPinData: boolean = false;

vi.mock('../features/collections/hooks/usePin', () => ({
  useV2HasPin: () => ({
    data: hasPinData,
    isLoading: false,
  }),
}));

describe('useSafeModeToggle (V2)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hasPinData = false;
    useAppStore.setState({
      activeGameId: 'g-1',
      safeMode: true,
    } as unknown as ReturnType<typeof useAppStore.getState>);
  });

  it('opens confirm modal directly when no PIN is set', async () => {
    const { result } = renderHook(() => useSafeModeToggle());

    expect(result.current.confirmModalOpen).toBe(false);
    expect(result.current.pinModalOpen).toBe(false);

    await act(async () => {
      await result.current.toggleSafeMode();
    });

    expect(result.current.confirmModalOpen).toBe(true);
    expect(result.current.pinModalOpen).toBe(false);
  });

  it('opens PIN modal first when PIN is set and switching Safe → Unsafe', async () => {
    hasPinData = true;

    const { result } = renderHook(() => useSafeModeToggle());

    await act(async () => {
      await result.current.toggleSafeMode();
    });

    expect(result.current.pinModalOpen).toBe(true);
    expect(result.current.confirmModalOpen).toBe(false);
  });

  it('transitions from PIN modal to confirm modal on handlePinSuccess', async () => {
    hasPinData = true;

    const { result } = renderHook(() => useSafeModeToggle());

    await act(async () => {
      await result.current.toggleSafeMode();
    });
    expect(result.current.pinModalOpen).toBe(true);

    await act(async () => {
      result.current.handlePinSuccess();
    });
    expect(result.current.pinModalOpen).toBe(false);
    expect(result.current.confirmModalOpen).toBe(true);
  });

  it('calls switchMutation.mutate on handleConfirmSwitch', async () => {
    const { result } = renderHook(() => useSafeModeToggle());

    // Open confirm modal
    await act(async () => {
      await result.current.toggleSafeMode();
    });
    expect(result.current.confirmModalOpen).toBe(true);

    // Confirm the switch
    await act(async () => {
      result.current.handleConfirmSwitch();
    });

    expect(switchMutateMock).toHaveBeenCalledTimes(1);
    expect(switchMutateMock).toHaveBeenCalledWith(
      { gameId: 'g-1', targetSafe: false },
      expect.objectContaining({ onSettled: expect.any(Function) }),
    );
    expect(result.current.confirmModalOpen).toBe(false);
  });

  it('closes confirm modal via closeConfirmModal', async () => {
    const { result } = renderHook(() => useSafeModeToggle());

    await act(async () => {
      await result.current.toggleSafeMode();
    });
    expect(result.current.confirmModalOpen).toBe(true);

    await act(async () => {
      result.current.closeConfirmModal();
    });
    expect(result.current.confirmModalOpen).toBe(false);
  });

  it('closes PIN modal via closePinModal', async () => {
    hasPinData = true;

    const { result } = renderHook(() => useSafeModeToggle());

    await act(async () => {
      await result.current.toggleSafeMode();
    });
    expect(result.current.pinModalOpen).toBe(true);

    await act(async () => {
      result.current.closePinModal();
    });
    expect(result.current.pinModalOpen).toBe(false);
  });

  it('does not open confirm when no activeGameId and handleConfirmSwitch is called', async () => {
    useAppStore.setState({
      activeGameId: null,
      safeMode: true,
    } as unknown as ReturnType<typeof useAppStore.getState>);

    const { result } = renderHook(() => useSafeModeToggle());

    await act(async () => {
      await result.current.toggleSafeMode();
    });

    // Confirm modal should open (toggle doesn't check gameId, handleConfirmSwitch does)
    expect(result.current.confirmModalOpen).toBe(true);

    await act(async () => {
      result.current.handleConfirmSwitch();
    });

    // Should NOT have called mutate because activeGameId is null
    expect(switchMutateMock).not.toHaveBeenCalled();
  });

  it('exposes safeMode and isSwitching from store/mutation', () => {
    const { result } = renderHook(() => useSafeModeToggle());

    expect(result.current.safeMode).toBe(true);
    expect(result.current.isSwitching).toBe(false);
  });
});
