import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useObjectSelectionRepair } from './useObjectSelectionRepair';

const checkPathExistsCmd = vi.fn();
const joinMock = vi.fn();

vi.mock('../../../lib/bindings', () => ({
  commands: {
    checkPathExistsCmd: (...args: unknown[]) => checkPathExistsCmd(...args),
  },
}));

vi.mock('@tauri-apps/api/path', () => ({
  join: (...args: unknown[]) => joinMock(...args),
}));

describe('useObjectSelectionRepair', () => {
  beforeEach(() => {
    checkPathExistsCmd.mockReset();
    joinMock.mockReset();
    joinMock.mockResolvedValue('E:/Mods/Characters/Diluc');
  });

  it('clears selection and requests repair only once for the same missing path', async () => {
    checkPathExistsCmd.mockResolvedValue(false);
    const clearSelection = vi.fn();
    const requestRepairSync = vi.fn().mockResolvedValue(undefined);

    const { rerender } = renderHook(
      (props: { modRootPath: string | null; selectedObjectFolderPath: string | null }) =>
        useObjectSelectionRepair({
          ...props,
          clearSelection,
          requestRepairSync,
        }),
      {
        initialProps: {
          modRootPath: 'E:/Mods',
          selectedObjectFolderPath: 'Characters/Diluc',
        },
      },
    );

    await waitFor(() => {
      expect(clearSelection).toHaveBeenCalledTimes(1);
      expect(requestRepairSync).toHaveBeenCalledTimes(1);
    });

    rerender({
      modRootPath: 'E:/Mods',
      selectedObjectFolderPath: 'Characters/Diluc',
    });

    await waitFor(() => {
      expect(clearSelection).toHaveBeenCalledTimes(1);
      expect(requestRepairSync).toHaveBeenCalledTimes(1);
    });
  });

  it('does nothing when the selected path still exists on disk', async () => {
    checkPathExistsCmd.mockResolvedValue(true);
    const clearSelection = vi.fn();
    const requestRepairSync = vi.fn().mockResolvedValue(undefined);

    renderHook(() =>
      useObjectSelectionRepair({
        modRootPath: 'E:/Mods',
        selectedObjectFolderPath: 'Characters/Diluc',
        clearSelection,
        requestRepairSync,
      }),
    );

    await waitFor(() => {
      expect(checkPathExistsCmd).toHaveBeenCalledTimes(1);
    });

    expect(clearSelection).not.toHaveBeenCalled();
    expect(requestRepairSync).not.toHaveBeenCalled();
  });
});

