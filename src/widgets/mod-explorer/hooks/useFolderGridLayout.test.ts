import { describe, expect, it } from 'vitest';
import { calculateFolderGridLayout } from './useFolderGridLayout';

describe('calculateFolderGridLayout', () => {
  it('uses the measured scroll-container width for stable grid metrics', () => {
    expect(calculateFolderGridLayout(1200, true, 20)).toEqual({
      columnCount: 6,
      cardWidth: 190,
      cardHeight: 260,
      rowCount: 4,
    });
  });

  it('keeps one-column list metrics independent of a stale grid width', () => {
    expect(calculateFolderGridLayout(0, false, 3)).toEqual({
      columnCount: 1,
      cardWidth: 0,
      cardHeight: 0,
      rowCount: 3,
    });
  });
});
