import { renderHook } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { useReviewTable } from './useReviewTable';
import type { ScanResultItem } from '../../../types/scanner';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn((path) => `asset://${path}`),
}));

describe('useReviewTable', () => {
  const mockData: ScanResultItem[] = [
    {
      path: 'C:\\mods\\Item1',
      rawName: 'Item1Raw',
      displayName: 'Item 1',
      matchedObject: 'Character',
      confidence: 'High',
      isDisabled: false,
    } as unknown as import('../../../types/scanner').ScanResultItem,
  ];

  it('initializes table correctly with columns and data', () => {
    const onOpenFolder = vi.fn();
    const onRename = vi.fn();

    const { result } = renderHook(() => useReviewTable({ data: mockData, onOpenFolder, onRename }));

    const table = result.current;

    expect(table.getRowModel().rows).toHaveLength(1);
    expect(table.getAllColumns()).toHaveLength(8); // select, thumbnail, displayName, matchedObject, detectedSkin, confidence, isDisabled, actions
  });
});
