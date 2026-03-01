import { describe, it, expect, beforeEach } from 'vitest';
import { useScannerStore } from './useScannerStore';
import { act } from '@testing-library/react';

describe('useScannerStore', () => {
  beforeEach(() => {
    // Reset store before each test
    act(() => {
      useScannerStore.getState().resetScanner();
    });
  });

  it('initializes with default state', () => {
    const state = useScannerStore.getState();
    expect(state.isScanning).toBe(false);
    expect(state.progress.current).toBe(0);
    expect(state.progress.total).toBe(0);
    expect(state.scanResults).toEqual([]);
    expect(state.stats).toEqual({ matched: 0, unmatched: 0 });
  });

  it('setIsScanning updates scanning state', () => {
    act(() => {
      useScannerStore.getState().setIsScanning(true);
    });
    expect(useScannerStore.getState().isScanning).toBe(true);

    act(() => {
      useScannerStore.getState().setIsScanning(false);
    });
    expect(useScannerStore.getState().isScanning).toBe(false);
  });

  it('updateProgress updates progress state', () => {
    act(() => {
      useScannerStore.getState().updateProgress(5, 'test_folder', 5000);
    });

    const { progress } = useScannerStore.getState();
    expect(progress.current).toBe(5);
    expect(progress.folderName).toBe('test_folder');
    expect(progress.etaMs).toBe(5000);
    expect(progress.label).toContain('Scanning test_folder...');
    expect(progress.label).toContain('~5s remaining');
  });

  it('updateProgress formats minutes correctly', () => {
    act(() => {
      useScannerStore.getState().updateProgress(5, 'test_folder', 65000);
    });

    expect(useScannerStore.getState().progress.label).toContain('~2m remaining');
  });

  it('setTotalFolders updates total progress', () => {
    act(() => {
      useScannerStore.getState().setTotalFolders(100);
    });

    expect(useScannerStore.getState().progress.total).toBe(100);
    expect(useScannerStore.getState().progress.label).toBe('Found 100 folders to scan');
  });

  const mockResult = {
    folderPath: '/test1',
  } as unknown as import('../types/scanner').ScanResultItem;
  act(() => {
    useScannerStore.getState().addScanResult(mockResult);
  });

  expect(useScannerStore.getState().scanResults).toEqual([{ folderPath: '/test1' }]);

  const mockResult2 = {
    folderPath: '/test2',
  } as unknown as import('../types/scanner').ScanResultItem;
  act(() => {
    useScannerStore.getState().addScanResult(mockResult2);
  });

  expect(useScannerStore.getState().scanResults).toEqual([mockResult, mockResult2]);
});

it('setScanResults overwrites results array', () => {
  const results = [
    { folderPath: '/test1' } as unknown as import('../types/scanner').ScanResultItem,
    { folderPath: '/test2' } as unknown as import('../types/scanner').ScanResultItem,
  ];

  act(() => {
    useScannerStore.getState().setScanResults(results);
  });

  expect(useScannerStore.getState().scanResults).toEqual(results);
});

it('setStats updates statistics', () => {
  act(() => {
    useScannerStore.getState().setStats(10, 5);
  });

  expect(useScannerStore.getState().stats).toEqual({ matched: 10, unmatched: 5 });
});
it('resetScanner restores default state', () => {
  act(() => {
    useScannerStore.getState().setIsScanning(true);
    useScannerStore.getState().updateProgress(50, 'folder', 100);
    useScannerStore.getState().setStats(10, 5);
    useScannerStore.getState().resetScanner();
  });

  const state = useScannerStore.getState();
  expect(state.isScanning).toBe(false);
  expect(state.progress.current).toBe(0);
  expect(state.progress.total).toBe(0);
  expect(state.stats).toEqual({ matched: 0, unmatched: 0 });
  expect(state.progress.label).toBe('Ready to scan');
});
