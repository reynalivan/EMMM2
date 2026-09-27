import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useObjectListBulkToolbarProps } from './useObjectListBulkToolbarProps';

describe('useObjectListBulkToolbarProps', () => {
  it('keeps object bulk controls visible while FolderGrid owns keyboard focus', () => {
    const input = {
      activePane: 'folderGrid',
      mutationsDisabled: false,
      switchDisabled: false,
      bulkSelect: {
        selectedIds: new Set(['object-1']),
        selectionCount: 1,
        isAnySelected: true,
        clearSelection: vi.fn(),
      } as never,
      setBulkTagModal: vi.fn(),
      handleBulkDelete: vi.fn(async () => undefined),
      handleBulkPin: vi.fn(async () => undefined),
      handleBulkEnable: vi.fn(async () => undefined),
      handleBulkDisable: vi.fn(async () => undefined),
      handleBulkClassifyAndMatch: vi.fn(async () => undefined),
      handleBulkFavorite: vi.fn(async () => undefined),
      handleBulkSafe: vi.fn(async () => undefined),
    };
    const { result } = renderHook(() => useObjectListBulkToolbarProps(input));

    expect(result.current.isAnySelected).toBe(true);
  });

  it('keeps enable and disable actionable while a prior bulk switch is pending', () => {
    const handleBulkEnable = vi.fn(async () => undefined);
    const handleBulkDelete = vi.fn(async () => undefined);
    const selectedIds = new Set(['object-1']);
    const input = {
      mutationsDisabled: true,
      switchDisabled: false,
      bulkSelect: {
        selectedIds,
        selectionCount: 1,
        isAnySelected: true,
        clearSelection: vi.fn(),
      } as never,
      setBulkTagModal: vi.fn(),
      handleBulkDelete,
      handleBulkPin: vi.fn(async () => undefined),
      handleBulkEnable,
      handleBulkDisable: vi.fn(async () => undefined),
      handleBulkClassifyAndMatch: vi.fn(async () => undefined),
      handleBulkFavorite: vi.fn(async () => undefined),
      handleBulkSafe: vi.fn(async () => undefined),
    };
    const { result } = renderHook(() => useObjectListBulkToolbarProps(input));

    result.current.onEnable();
    result.current.onDelete();

    expect(handleBulkEnable).toHaveBeenCalledWith(selectedIds);
    expect(handleBulkDelete).not.toHaveBeenCalled();
    expect(result.current.toggleDisabled).toBe(false);
  });
});
