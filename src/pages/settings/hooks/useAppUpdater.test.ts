import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useAppUpdater } from './useAppUpdater';

const checkAppUpdate = vi.fn();
const installAppUpdate = vi.fn();

vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: {
    checkAppUpdate: (...args: unknown[]) => checkAppUpdate(...args),
    installAppUpdate: (...args: unknown[]) => installAppUpdate(...args),
  },
}));

describe('useAppUpdater', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('checks for update successfully', async () => {
    const mockUpdate = { version: '1.1.0' };

    checkAppUpdate.mockResolvedValue(mockUpdate);

    const { result } = renderHook(() => useAppUpdater());

    await act(async () => {
      await result.current.checkForUpdate();
    });

    expect(result.current.isChecking).toBe(false);
    expect(result.current.update).toEqual(mockUpdate);
    expect(result.current.error).toBe(null);
  });

  it('handles update check failure gracefully', async () => {
    checkAppUpdate.mockRejectedValue(new Error('Network error'));

    const { result } = renderHook(() => useAppUpdater());

    await act(async () => {
      await result.current.checkForUpdate();
    });

    expect(result.current.isChecking).toBe(false);
    expect(result.current.update).toBe(null);
    expect(result.current.error).toContain('Network error');
  });

  it('allows dismissing an update', () => {
    const { result } = renderHook(() => useAppUpdater());

    act(() => {
      result.current.dismiss();
    });

    expect(result.current.update).toBeNull();
    expect(result.current.error).toBeNull();
    expect(result.current.progress).toBeNull();
  });
});
