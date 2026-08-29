import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useAppUpdater } from './useAppUpdater';
import { check } from '@tauri-apps/plugin-updater';

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn(),
}));

describe('useAppUpdater', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('checks for update successfully', async () => {
    const mockUpdate = { version: '1.1.0' };

    // Using TS ignores since Tauri plugin updates have rich methods
    // @ts-expect-error test mock returns simplified version object
    vi.mocked(check).mockResolvedValue(mockUpdate);

    const { result } = renderHook(() => useAppUpdater());

    await act(async () => {
      await result.current.checkForUpdate();
    });

    expect(result.current.isChecking).toBe(false);
    expect(result.current.update).toEqual(mockUpdate);
    expect(result.current.error).toBe(null);
  });

  it('handles update check failure gracefully', async () => {
    vi.mocked(check).mockRejectedValue(new Error('Network error'));

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
